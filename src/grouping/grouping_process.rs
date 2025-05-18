use crate::database::database::DBConn;
use crate::database::group::arrangement::{Arrangement, ArrangementDependencyType, ArrangementDetails};
use crate::database::group::group::Group;
use crate::database::group::shared_group::SharedGroup;
use crate::database::picture::picture::Picture;
use crate::database::picture::picture_tag::PictureTag;
use crate::database::tag::tag::Tag;
use crate::grouping::strategy_filtering::FilterType;
use crate::grouping::strategy_grouping::StrategyGrouping;
use crate::grouping::topological_sorts::{topological_sort, topological_sort_filtered, topological_sort_from};
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;
use validator::ValidateRequired;
// Process:
// - Create arrangement:
//   Group only on this arrangement as no other arrangement can reference it.
// - Edit arrangement:
//   Clear the arrangements groups. If possible do a difference system to skip unchanged pictured.
//   If needed to recreate groups, establish a mapping with references and shared groups.
//   Group only on this arrangement and all arrangement that depends on it recursively.
// - Delete arrangement:
//   Make sure there are no dependent arrangements or shared groups.
//   Delete the groups and the arrangement
//
// - Add a new picture:
//   Group the picture against all arrangements in topological order.
// - Edit picture attributes
//   Remove the pictures from the groups it doesn’t match anymore (from arrangements matching the dependency type).
//   Group the picture against all arrangements matching the dependency type in topological order.
//   Remove the picture from the shared groups associated with the groups in the step 1.
//   If possible, establish a difference to prevent removing the picture from other user contexts .
// - Delete pictures permanently:
//   Just cascade delete the picture. No reference will stay.
//
//
// |                    | Pictures | Arrangements
// |--------------------|----------|-------------------
// | Create arrangement | All      | Created
// | Edit   arrangement | All      | Edited + Dependants
// | Add/Edit  pictures | Edited   | All

pub fn group_pictures(
    conn: &mut DBConn,
    user_id: i32,
    picture_ids_filter: Option<&Vec<i64>>,
    arrangement_id_filter: Option<i32>,
    dependency_type_filter: Option<&ArrangementDependencyType>,
    do_ungroup: bool,
) -> Result<(), ErrorResponder> {
    // Fetch all not manual arrangements and the list of their group ids
    let mut arrangements = Arrangement::list_arrangements_and_groups(conn, user_id)?;

    if arrangement_id_filter.is_some() && dependency_type_filter.is_some() {
        return Err(ErrorType::InvalidInput("Cannot filter by arrangement id and dependency type at the same time".to_string()).res());
    }
    if do_ungroup && picture_ids_filter.is_none() {
        return Err(ErrorType::InvalidInput("Cannot ungroup without a list of picture ids".to_string()).res());
    }

    // Filter arrangements if needed
    arrangements = if let Some(arrangement_id) = arrangement_id_filter {
        let origin_arrangement = arrangements
            .iter()
            .find(|arrangement| arrangement.arrangement.id == arrangement_id)
            .ok_or(
                ErrorType::InvalidInput(format!("Arrangement of ID {} is not an arrangement of the user {}", arrangement_id, user_id).to_string())
                    .res(),
            )?
            .clone();

        arrangements.retain(|arrangement| arrangement.arrangement.groups_dependant || arrangement_id == arrangement.arrangement.id);
        topological_sort_from(arrangements, &origin_arrangement)
    } else if let Some(dependency_type) = dependency_type_filter {
        topological_sort_filtered(arrangements, dependency_type)
    } else {
        topological_sort(arrangements)
    };

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
                    group_add_pictures(conn, &group_pictures, *group_id)?;
                }
                if remaining_pictures_ids.len() != 0 {
                    let (other_group_id, update) = filter_grouping.get_or_create_other_group_id(conn, arrangement.arrangement.id)?;
                    update_strategy = update;
                    group_add_pictures(conn, &remaining_pictures_ids, other_group_id)?;
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
                        group_add_pictures(conn, &remaining_pictures_ids, group_id)?;
                    }
                }
                if remaining_pictures_ids.len() != 0 {
                    let (other_group_id, update) = tag_grouping.get_or_create_other_group_id(conn, arrangement.arrangement.id)?;
                    update_strategy |= update;
                    group_add_pictures(conn, &remaining_pictures_ids, other_group_id)?;
                }
            }
            StrategyGrouping::GroupByExifValues(e) => {}
            StrategyGrouping::GroupByExifInterval(e) => {}
            StrategyGrouping::GroupByLocation(l) => {}
        }

        if update_strategy {
            let strategy = arrangement.strategy.clone();
            arrangement.groups = strategy.groupings.get_groups();
            arrangement.arrangement.set_strategy(conn, strategy)?;
        }
    }

    if do_ungroup {
        // Ungrouping any picture that is in a group of any related arrangement, but does not match the group anymore.
        for arrangement in arrangements.iter() {
            info!(
                "Ungrouping pictures from arrangement: {:?} of user {:?}",
                arrangement.arrangement.id, user_id
            );
            let arrangement_filter = &arrangement.strategy.filter;
            match &arrangement.strategy.groupings {
                StrategyGrouping::GroupByFilter(filter_grouping) => {
                    for (filter, group_id) in &filter_grouping.filters {
                        let ungroup_pictures = filter
                            .clone()
                            .not()
                            .or(arrangement_filter.clone().not())
                            .and(FilterType::IncludeGroups(vec![*group_id]).to_strategy())
                            .filter_pictures(conn, None)?;

                        group_remove_pictures(conn, &ungroup_pictures, *group_id)?;
                    }
                }
                StrategyGrouping::GroupByTags(tag_grouping) => {
                    for tag in Tag::list_tags(conn, tag_grouping.tag_group_id)? {
                        // let ungroup_pictures = PictureTag::filter_pictures_from_tag(conn, tag.id, None)?;
                        // group_remove_pictures(conn, &ungroup_pictures, tag_grouping.tag_id_to_group_id[&tag.id])?;
                    }
                }
                StrategyGrouping::GroupByExifValues(e) => {}
                StrategyGrouping::GroupByExifInterval(e) => {}
                StrategyGrouping::GroupByLocation(l) => {}
            }
        }
    }

    Ok(())
}

/*/// Ungroup any picture from the picture_ids list that is in a group but does not match the group anymore.
/// - Sort the arrangements that match the dependency type in topological order.
/// - For each arrangement group,
///   - remove pictures that don't match the arrangement filter, or that don’t match the group
///   - do not matter about shared groups as this will be handled after an eventual regrouping in regroup_edited_pictures.
///   - and return the list of the removed pictures,
/// - Return a hashmap with the group id as key and the list of removed pictures as value.
fn ungroup_pictures(
    conn: &mut DBConn,
    user_id: i32,
    picture_ids: &Vec<i64>,
    dependency_type: &ArrangementDependencyType,
) -> Result<(), ErrorResponder> {
    todo!();

}

/// Ungroup pictures that do not match a group anymore and then group them back.
/// - Calling ungroup_pictures and storing the removed pictures.
/// - Calling group_pictures to regroup the pictures as if they were new pictures.
/// - Propagating ungrouped pictures to the shared groups.
///   This is done only after to prevent the recipient from losing access to the pictures immediately before gaining access back.
pub fn regroup_edited_pictures(
    conn: &mut DBConn,
    user_id: i32,
    picture_ids: &Vec<i64>,
    dependency_type: &ArrangementDependencyType,
) -> Result<(), ErrorResponder> {
    todo!();
}*/

/// Add pictures to a group and then check for each user to which the group is shared:
/// - For the pictures the user gained access to:
///   - Add the defaults tags to these pictures.
///   - Group them in his context.
/// - If share match conversion is enabled, apply it to all pictures.
fn group_add_pictures(conn: &mut DBConn, picture_ids: &Vec<i64>, group_id: i32) -> Result<(), ErrorResponder> {
    if picture_ids.len() == 0 {
        return Ok(());
    }
    let shared_groups = SharedGroup::from_group_id(conn, group_id)?;

    // Save the pictures that are already accessible to each recipient of a shared instance of this group.
    let mut users_accessible_pictures: HashMap<i32, HashSet<i64>> = HashMap::new();
    for shared_group in shared_groups.iter() {
        let mut accessible_pictures =
            HashSet::from_iter(Picture::filter_user_accessible_pictures(conn, shared_group.user_id, picture_ids)?.into_iter());
        if let Some(already_accessible_pictures) = users_accessible_pictures.get(&shared_group.user_id) {
            accessible_pictures = already_accessible_pictures.union(&accessible_pictures).into_iter().cloned().collect()
        }
        users_accessible_pictures.insert(shared_group.user_id, accessible_pictures);
    }

    let added_pictures: HashSet<i64> = HashSet::from_iter(Group::add_pictures(conn, group_id, picture_ids)?);

    for shared_group in shared_groups {
        let empty_hashset = HashSet::new();
        let accessible_pictures = users_accessible_pictures.get(&shared_group.user_id).unwrap_or(&empty_hashset);

        // Group new pictures on which the user gained access to.
        let gained_access_pictures = added_pictures.difference(accessible_pictures);
        let gained_access_pictures = Vec::from_iter(gained_access_pictures.into_iter().cloned());

        // Even if the picture is newly accessible, it can already have tags from the time it was accessible.
        // Then, we are adding defaults tags only to pictures that have no tag from the group.
        PictureTag::add_default_tags_to_pictures_without_tags(conn, shared_group.user_id, &gained_access_pictures)?;

        group_pictures(conn, shared_group.user_id, Some(&gained_access_pictures), None, None, false)?;

        // Applying share match conversion if enabled.
        if let Some(smc_group_id) = shared_group.match_conversion_group_id {
            // TODO: Apply share match conversion on pictures added_pictures for user shared_group.user_id and destination group smc_group_id
        }
    }

    Ok(())
}

pub fn group_remove_pictures(conn: &mut DBConn, picture_ids: &Vec<i64>, group_id: i32) -> Result<(), ErrorResponder> {
    if picture_ids.len() == 0 {
        return Ok(());
    }
    let shared_groups = SharedGroup::from_group_id(conn, group_id)?;

    todo!()
}
