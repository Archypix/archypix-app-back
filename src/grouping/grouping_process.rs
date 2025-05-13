use crate::database::database::DBConn;
use crate::database::group::arrangement::{Arrangement, ArrangementDetails};
use crate::database::group::group::Group;
use crate::database::group::shared_group::SharedGroup;
use crate::database::picture::picture::Picture;
use crate::database::picture::picture_tag::PictureTag;
use crate::database::schema::shared_groups;
use crate::database::tag::tag::Tag;
use crate::grouping::strategy_grouping::StrategyGrouping;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use rocket::yansi::Paint;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};
// Requirements:
// - Create arrangement:
//   Group only on this arrangement as no other arrangement can reference it.
// - Edit arrangement:
//   Group only on this arrangement and all arrangement that depends on it recursively.
// - Delete arrangement:
//   No grouping to do, just deleting all the groups after making sure that no other arrangement depends on it.
// - Add new picture / edit picture attributes:
//   Group the picture against all arrangements in topological order.
// |                    | Pictures | Arrangements
// |--------------------|----------|-------------------
// | Create arrangement | All      | Created
// | Edit   arrangement | All      | Edited + Dependants
// | Add/Edit  pictures | Edited   | All

pub fn group_new_pictures(
    conn: &mut DBConn,
    user_id: i32,
    picture_ids_filter: Option<&Vec<i64>>,
    arrangement_id_filter: Option<i32>,
    already_processed_users: &mut HashSet<i32>,
) -> Result<(), ErrorResponder> {
    // Fetch all not manual arrangements and the list of their group ids
    let mut arrangements = Arrangement::list_arrangements_and_groups(conn, user_id)?;

    // Filter arrangements if needed
    if let Some(arrangement_id) = arrangement_id_filter {
        let origin_arrangement = arrangements
            .iter()
            .find(|arrangement| arrangement.arrangement.id == arrangement_id)
            .ok_or(
                ErrorType::InvalidInput(format!("Arrangement of ID {} is not an arrangement of the user {}", arrangement_id, user_id).to_string())
                    .res(),
            )?
            .clone();

        arrangements.retain(|arrangement| arrangement.arrangement.groups_dependant || arrangement_id == arrangement.arrangement.id);
        arrangements = topological_sort_from(arrangements, &origin_arrangement);
    } else {
        arrangements = topological_sort(arrangements);
    }

    for mut arrangement in arrangements.iter_mut() {
        // Keep only pictures that match this arrangement
        info!(
            "Grouping pictures into arrangement: {:?} of user {:?}",
            arrangement.arrangement.id, user_id
        );

        let pictures_ids = arrangement.strategy.filter.filter_pictures(conn, picture_ids_filter)?;

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
                    let group_pictures = filter.filter_pictures(conn, Some(pictures_to_group))?;
                    remaining_pictures_ids.retain(|&x| group_pictures.contains(&x));
                    add_pictures_to_group_and_group_via_shared_group(conn, &group_pictures, *group_id, already_processed_users)?;
                }
                if remaining_pictures_ids.len() != 0 {
                    let (other_group_id, update) = filter_grouping.get_or_create_other_group_id(conn, arrangement.arrangement.id)?;
                    update_strategy = update;
                    add_pictures_to_group_and_group_via_shared_group(conn, &remaining_pictures_ids, other_group_id, already_processed_users)?;
                }
            }
            StrategyGrouping::GroupByTags(tag_grouping) => {
                let mut remaining_pictures_ids = pictures_ids.clone();
                let tags = Tag::list_tags(conn, tag_grouping.tag_group_id)?;
                for tag in tags {
                    let pictures_to_group = if arrangement.strategy.preserve_unicity {
                        &remaining_pictures_ids
                    } else {
                        &pictures_ids
                    };
                    let group_pictures = PictureTag::filter_pictures_from_tag(conn, tag.id, pictures_to_group)?;
                    if group_pictures.len() != 0 {
                        let (group_id, update) = tag_grouping.get_or_create_tag_group_id(conn, &tag, arrangement.arrangement.id)?;
                        update_strategy |= update;
                        remaining_pictures_ids.retain(|&x| group_pictures.contains(&x));
                        add_pictures_to_group_and_group_via_shared_group(conn, &remaining_pictures_ids, group_id, already_processed_users)?;
                    }
                }
                if remaining_pictures_ids.len() != 0 {
                    let (other_group_id, update) = tag_grouping.get_or_create_other_group_id(conn, arrangement.arrangement.id)?;
                    update_strategy |= update;
                    add_pictures_to_group_and_group_via_shared_group(conn, &remaining_pictures_ids, other_group_id, already_processed_users)?;
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
    }
    already_processed_users.insert(user_id);

    Ok(())
}

/// Add pictures to a group and then check for each user to which the group is shared:
/// - Group the pictures on which the user gained access in his context.
/// - If share match conversion is enabled, apply it to all pictures.
pub fn add_pictures_to_group_and_group_via_shared_group(
    conn: &mut DBConn,
    picture_ids: &Vec<i64>,
    group_id: i32,
    already_processed_users: &mut HashSet<i32>,
) -> Result<(), ErrorResponder> {
    let shared_groups = SharedGroup::from_group_id(conn, group_id)?;

    let mut users_accessible_pictures = HashMap::new();

    for shared_group in shared_groups.iter() {
        // TODO: Even if the user to which the group is shared already has access to the picture, we need to apply share match conversion.
        //  If the user has just gained access to the picture, in addition to share match conversion should be applied the grouping strategies.

        // TODO: check if the user has access to the pictures.
        let accessible_pictures = Picture::filter_user_accessible_pictures(conn, shared_group.user_id, picture_ids)?;
        if accessible_pictures.len() != 0 {
            users_accessible_pictures.insert(shared_group.user_id, accessible_pictures);
        }
    }

    Group::add_pictures(conn, group_id, picture_ids)?;

    for shared_group in shared_groups {

        // TODO: Group new pictures on which the user gained access to.
        // group_new_pictures(conn, shared_group.user_id, Some(picture_ids), None, already_processed_users)?;

        // TODO: apply share match conversion if enabled.
    }

    Ok(())
}

/// Sort the arrangements in topological order keeping only the subtree being the origin arrangement and its dependants.
/// First explore all arrangements that depend on the origin arrangement
/// Then apply a topological sort on these arrangements only.
pub fn topological_sort_from(arrangements: Vec<ArrangementDetails>, origin_arrangement: &ArrangementDetails) -> Vec<ArrangementDetails> {
    let mut visited: HashSet<i32> = HashSet::new();
    let mut processing: VecDeque<i32> = VecDeque::new(); // arrangement_id, group_ids

    visited.insert(origin_arrangement.arrangement.id);
    processing.push_back(origin_arrangement.arrangement.id);

    // Process the arrangements that depend on the processing arrangement
    while let Some(processing_id) = processing.pop_front() {
        let new_processing_ids = arrangements
            .iter()
            // Keep only arrangements that are not already visited
            .filter(|a| !visited.contains(&a.arrangement.id))
            // Keep only arrangements that depend on processing_a
            .filter(|a| a.dependant_arrangements.contains(&processing_id))
            .map(|a| a.arrangement.id)
            .collect::<Vec<i32>>();

        for new_processing_id in new_processing_ids {
            visited.insert(new_processing_id);
            processing.push_back(new_processing_id);
        }
    }

    // Remove arrangements that have not been visited
    let arrangements = arrangements
        .into_iter()
        .filter(|a| visited.contains(&a.arrangement.id))
        .collect::<Vec<ArrangementDetails>>();

    // Sort topologically the remaining arrangements.
    topological_sort(arrangements)
}

/// Topologically sort the arrangements in function of their dependencies over a group of another arrangement.
/// If A depends on B, B will appear before A in the sorted list.
pub fn topological_sort(mut arrangements: Vec<ArrangementDetails>) -> Vec<ArrangementDetails> {
    let mut sorted = Vec::new();
    let mut visited = HashSet::new();
    let mut temp_stack = HashSet::new();

    let mut id_map: HashMap<i32, &ArrangementDetails> = HashMap::new();
    for arrangement in &arrangements {
        id_map.insert(arrangement.arrangement.id, arrangement);
    }

    debug!(
        "Sorting topologically arrangements: {:?}",
        arrangements.iter().map(|a| a.arrangement.id).collect::<Vec<i32>>()
    );
    // Recursive DFS helper for topological sort
    fn visit<'a>(
        node_id: i32,
        id_map: &'a HashMap<i32, &'a ArrangementDetails>,
        visited: &mut HashSet<i32>,
        temp_stack: &mut HashSet<i32>,
        sorted: &mut Vec<i32>,
    ) -> Result<(), String> {
        // Detect a cycle
        if temp_stack.contains(&node_id) {
            info!("Cycle detected in dependency graph");
            return Ok(()); //Err("Cycle detected in dependency graph".to_string());
        }
        if visited.contains(&node_id) {
            return Ok(()); // Already processed
        }

        // Temporarily mark this node
        temp_stack.insert(node_id);

        debug!("    Looking for dependents of arrangement {}", node_id);
        // Process all dependencies of this node
        if let Some(node) = id_map.get(&node_id) {
            for &dep in &node.dependant_arrangements {
                debug!("      Found dependency of {} : {}", node_id, dep);
                visit(dep, id_map, visited, temp_stack, sorted)?;
            }
        }

        // Mark this node as fully processed and add to the result
        temp_stack.remove(&node_id);
        visited.insert(node_id);
        sorted.push(node_id);
        Ok(())
    }

    // Execute the topological sort for all nodes
    for arrangement in id_map.values() {
        debug!("  Starting DFS from arrangement ID: {}", arrangement.arrangement.id);
        let _res = visit(arrangement.arrangement.id, &id_map, &mut visited, &mut temp_stack, &mut sorted);
    }

    let sorted_indices: HashMap<i32, usize> = sorted.iter().enumerate().map(|(i, &id)| (id, i)).collect();

    // Sort the owned values
    arrangements.sort_by(|a, b| {
        if let Some(i) = sorted_indices.get(&a.arrangement.id) {
            if let Some(i2) = sorted_indices.get(&b.arrangement.id) {
                return i.cmp(i2);
            }
            return Ordering::Less;
        }
        Ordering::Greater
    });
    arrangements
}
