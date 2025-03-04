use crate::database::database::DBConn;
use crate::database::group::arrangement::{Arrangement, ArrangementDetails};
use crate::database::group::group::Group;
use crate::database::picture::picture_tag::PictureTag;
use crate::database::tag::tag::Tag;
use crate::grouping::strategy_grouping::StrategyGrouping;
use crate::utils::errors_catcher::ErrorResponder;
use std::collections::{HashMap, HashSet};

pub fn group_new_pictures(conn: &mut DBConn, user_id: u32, pictures: Vec<u64>) -> Result<(), ErrorResponder> {
    // Fetch all not manual arrangements and the list of their groups ids
    let arrangements = Arrangement::list_arrangements_and_groups(conn, user_id)?;

    // Sort topologically the arrangements in function of their dependencies over a group of another arrangement
    let arrangements: Vec<ArrangementDetails> = topological_sort(arrangements);

    for mut arrangement in arrangements {
        // Keep only pictures that match this arrangement
        info!("Processing arrangement: {:?}", arrangement.arrangement.id);
        let pictures_ids = arrangement.strategy.filter.filter_pictures(conn, &pictures)?;

        // Add pictures to groups

        let mut update_strategy = false;
        match &mut arrangement.strategy.groupings {
            StrategyGrouping::GroupByFilter(filter_grouping) => {
                let mut remaining_pictures_ids = pictures_ids.clone();
                for (filter, group_id) in &filter_grouping.filters {
                    let pictures_to_group = if arrangement.strategy.preserve_unicity {
                        &remaining_pictures_ids
                    } else {
                        &pictures_ids
                    };
                    let group_pictures = filter.filter_pictures(conn, pictures_to_group)?;
                    remaining_pictures_ids.retain(|&x| group_pictures.contains(&x));
                    Group::add_pictures(conn, *group_id, group_pictures)?;
                }
                if remaining_pictures_ids.len() != 0 {
                    let (other_group_id, update) = filter_grouping.get_or_create_other_group_id(conn, arrangement.arrangement.id)?;
                    update_strategy = update;
                    Group::add_pictures(conn, other_group_id, remaining_pictures_ids)?;
                }
            }
            StrategyGrouping::GroupByTags(tag_grouping) => {
                let mut remaining_pictures_ids = pictures_ids.clone();
                let tags = Tag::list_tags(conn, user_id)?;
                // TODO: Fetch all tags of the group,
                //  then for each tag, fetch the pictures that have this tag and add them to the matching group
                //  add the remaining pictures to the other group
                //  Take in account preserve_unicity to maybe add a picture to multiple groups
                for tag in tags {
                    let pictures_to_group = if arrangement.strategy.preserve_unicity {
                        &remaining_pictures_ids
                    } else {
                        &pictures_ids
                    };
                    let group_pictures = PictureTag::get_tag_pictures(conn, tag.id, pictures_to_group)?;
                    if group_pictures.len() != 0 {
                        let (group_id, update) = tag_grouping.get_or_create_tag_group_id(conn, &tag, arrangement.arrangement.id)?;
                        update_strategy |= update;
                        remaining_pictures_ids.retain(|&x| group_pictures.contains(&x));
                        Group::add_pictures(conn, group_id, group_pictures)?;
                    }
                }
                if remaining_pictures_ids.len() != 0 {
                    let (other_group_id, update) = tag_grouping.get_or_create_other_group_id(conn, arrangement.arrangement.id)?;
                    update_strategy |= update;
                    Group::add_pictures(conn, other_group_id, remaining_pictures_ids)?;
                }
            }
            StrategyGrouping::GroupByExifValues(e) => {}
            StrategyGrouping::GroupByExifInterval(e) => {}
            StrategyGrouping::GroupByLocation(l) => {}
        }

        if update_strategy {
            let strategy = arrangement.strategy.clone();
            arrangement.arrangement.set_strategy(conn, strategy)?;
        }

        // If the group is shared, add the picture to the groups of the other users
    }

    Ok(())
}

pub fn topological_sort(mut arrangements: Vec<ArrangementDetails>) -> Vec<ArrangementDetails> {
    let mut sorted = Vec::new();
    let mut visited = HashSet::new();
    let mut temp_stack = HashSet::new();

    let mut id_map: HashMap<u32, &ArrangementDetails> = HashMap::new();
    for arrangement in &arrangements {
        id_map.insert(arrangement.arrangement.id, &arrangement);
    }

    // Recursive DFS helper for topological sort
    fn visit<'a>(
        node_id: u32,
        id_map: &'a HashMap<u32, &'a ArrangementDetails>,
        visited: &mut HashSet<u32>,
        temp_stack: &mut HashSet<u32>,
        sorted: &mut Vec<&'a ArrangementDetails>,
    ) -> Result<(), String> {
        // Detect a cycle
        if temp_stack.contains(&node_id) {
            info!("Cycle detected in dependency graph");
            return Err("Cycle detected in dependency graph".to_string());
        }
        if visited.contains(&node_id) {
            return Ok(()); // Already processed
        }

        // Temporarily mark this node
        temp_stack.insert(node_id);

        // Process all dependencies of this node
        if let Some(node) = id_map.get(&node_id) {
            for &dep in &node.dependant_arrangements {
                visit(dep, id_map, visited, temp_stack, sorted)?;
            }
        }

        // Mark this node as fully processed and add to the result
        temp_stack.remove(&node_id);
        visited.insert(node_id);
        if let Some(node) = id_map.get(&node_id) {
            sorted.push(node);
        }
        Ok(())
    }

    // Execute the topological sort for all nodes
    for arrangement in id_map.values() {
        let _res = visit(arrangement.arrangement.id, &id_map, &mut visited, &mut temp_stack, &mut sorted);
    }
    // Sort the owned values
    arrangements.clone().sort_by(|a, b| {
        sorted
            .iter()
            .position(|o| o.arrangement.id == a.arrangement.id)
            .unwrap_or(0)
            .cmp(&sorted.iter().position(|o| o.arrangement.id == b.arrangement.id).unwrap_or(0))
    });
    arrangements
}
