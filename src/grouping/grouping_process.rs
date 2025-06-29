use crate::database::database::DBConn;
use crate::database::group::arrangement::{Arrangement, ArrangementDependencyType, ArrangementDetails};
use crate::database::group::group::Group;
use crate::database::group::shared_group::SharedGroup;
use crate::database::picture::picture::Picture;
use crate::database::picture::picture_tag::PictureTag;
use crate::database::tag::tag::Tag;
use crate::grouping::strategy_filtering::FilterType;
use crate::grouping::strategy_grouping::{StrategyGrouping, StrategyGroupingTrait, UngroupRecord};
use crate::grouping::topological_sorts::{topological_sort, topological_sort_filtered, topological_sort_from};
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use itertools::Itertools;
use rocket::yansi::Paint;
use std::collections::{HashMap, HashSet};
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

/// Group pictures into arrangements’ groups.
/// If `do_ungroup` is true, pictures that do not match the arrangement filter will be ungrouped, but `picture_ids_filter` must be provided.
/// If `arrangement_id_filter` is provided, only pictures from this arrangement will be grouped.
/// If `dependency_type_filter` is provided, only pictures from arrangements of this dependency type will be grouped.
/// `arrangement_id_filter` and `dependency_type_filter` cannot be used at the same time.
pub fn group_pictures(
    conn: &mut DBConn,
    user_id: i32,
    picture_ids_filter: Option<&Vec<i64>>,
    arrangement_id_filter: Option<i32>,
    dependency_type_filter: Option<&ArrangementDependencyType>,
    do_ungroup: bool,
) -> Result<(), ErrorResponder> {
    debug!("Grouping pictures for user {}, pictures: {:?}", user_id, picture_ids_filter);
    debug!(
        "Parameters: arrangement_id_filter: {:?}, dependency_type_filter: {:?}, do_ungroup: {}",
        arrangement_id_filter, dependency_type_filter, do_ungroup
    );

    // Fetch all not manual arrangements and the list of their group ids
    let mut arrangements = Arrangement::list_arrangements_and_groups(conn, user_id)?;

    if arrangement_id_filter.is_some() && dependency_type_filter.is_some() {
        return Err(ErrorType::InvalidInput("Cannot filter by arrangement id and dependency type at the same time".to_string()).res());
    }
    if do_ungroup && picture_ids_filter.is_none() {
        // TODO: No optimization developed for wen editing arrangements. Editing arrangements works like if all pictures were edited for now.
        //return Err(ErrorType::InvalidInput("Cannot ungroup without a list of picture ids".to_string()).res());
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

    let mut ungroup_record = UngroupRecord::new(do_ungroup);

    for arrangement in arrangements.iter_mut() {
        // Keep only pictures that match this arrangement
        let pictures_ids: HashSet<i64> = HashSet::from_iter(arrangement.strategy.filter.filter_pictures(conn, picture_ids_filter)?.into_iter());

        info!(
            "Grouping {} pictures into arrangement {} of user {}",
            pictures_ids.len(),
            arrangement.arrangement.id,
            user_id
        );
        debug!("Pictures ids: {:?}", pictures_ids);

        // Add pictures to groups
        let mut update_strategy = false;
        let a_id = arrangement.arrangement.id;
        let preserve_unicity = arrangement.strategy.preserve_unicity;
        match &mut arrangement.strategy.groupings {
            StrategyGrouping::GroupByFilter(filter_grouping) => {
                update_strategy |= filter_grouping.group_pictures(conn, a_id, preserve_unicity, &mut ungroup_record, &pictures_ids)?;
            }
            StrategyGrouping::GroupByTags(tag_grouping) => {
                update_strategy |= tag_grouping.group_pictures(conn, a_id, preserve_unicity, &mut ungroup_record, &pictures_ids)?;
            }
            StrategyGrouping::GroupByExifValues(e) => {}
            StrategyGrouping::GroupByExifInterval(e) => {}
            StrategyGrouping::GroupByLocation(l) => {}
        }

        if update_strategy {
            let strategy = arrangement.strategy.clone();
            arrangement.arrangement.set_strategy(conn, Some(strategy))?;
        }
    }

    if do_ungroup {
        // Add records for pictures that do not match the arrangement filter
        for arrangement in arrangements.iter() {
            info!(
                "Ungrouping pictures from arrangement: {:?} of user {:?}",
                arrangement.arrangement.id, user_id
            );
            // Pictures that do not match the arrangement filter, but that are in a group of the arrangement.
            let group_ids = arrangement.strategy.groupings.get_groups().clone();
            let ungroup_pictures_ids = arrangement
                .strategy
                .filter
                .clone()
                .not()
                .and(FilterType::IncludeGroups(group_ids.clone()).to_strategy())
                .filter_pictures(conn, picture_ids_filter)?;

            let ungroup_pictures_ids_set = HashSet::from_iter(ungroup_pictures_ids.into_iter());
            group_ids.into_iter().for_each(|group_id| {
                ungroup_record.add(group_id, ungroup_pictures_ids_set.clone());
            });
        }
        // Ungroup all records
        ungroup_record
            .map
            .into_iter()
            .try_for_each(|(group_id, picture_ids)| group_remove_pictures(conn, group_id, &picture_ids.into_iter().collect_vec()))?;
    }

    Ok(())
}

/// Add pictures to a group and then check for each user to which the group is shared:
/// - For the pictures the user gained access to:
///   - Add the defaults tags to these pictures.
///   - Group them in his context.
/// - If share match conversion is enabled, apply it to all pictures.
pub fn group_add_pictures(conn: &mut DBConn, group_id: i32, picture_ids: &Vec<i64>) -> Result<(), ErrorResponder> {
    debug!("Adding {} pictures to group {}, (ids: {:?})", picture_ids.len(), group_id, picture_ids);
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

        debug!(
            "Propagating {} added pictures to user {}",
            gained_access_pictures.len(),
            shared_group.user_id
        );
        group_pictures(conn, shared_group.user_id, Some(&gained_access_pictures), None, None, false)?;

        // Applying share match conversion if enabled.
        if let Some(smc_group_id) = shared_group.match_conversion_group_id {
            // TODO: Apply share match conversion on pictures added_pictures for user shared_group.user_id and destination group smc_group_id
        }
    }

    Ok(())
}

/// Remove the pictures from the group, and remove them from all groups of users who lost access to them.
pub fn group_remove_pictures(conn: &mut DBConn, group_id: i32, picture_ids: &Vec<i64>) -> Result<(), ErrorResponder> {
    debug!(
        "Removing {} pictures from group {}, (ids: {:?})",
        picture_ids.len(),
        group_id,
        picture_ids
    );
    if picture_ids.len() == 0 {
        return Ok(());
    }
    let removed_pictures = Group::remove_pictures(conn, group_id, &picture_ids)?;
    if removed_pictures.len() == 0 {
        return Ok(());
    }
    group_manage_removed_pictures(conn, group_id, removed_pictures)
}

/// Remove all the pictures of the group, and remove them from all groups of users who lost access to them.
pub fn group_clear_pictures(conn: &mut DBConn, group_id: i32) -> Result<(), ErrorResponder> {
    debug!("Removing all pictures from group {}", group_id);
    let removed_pictures = Group::clear_and_get_pictures(conn, group_id)?;
    if removed_pictures.len() == 0 {
        return Ok(());
    }
    group_manage_removed_pictures(conn, group_id, removed_pictures)
}
/// Propagate the removal of the pictures to all groups of users who lost access to them.
fn group_manage_removed_pictures(conn: &mut DBConn, group_id: i32, removed_pictures: Vec<i64>) -> Result<(), ErrorResponder> {
    let shared_groups = SharedGroup::from_group_id(conn, group_id)?;
    for shared_group in shared_groups.iter() {
        let unaccessible_pictures = Picture::filter_user_accessible_pictures(conn, shared_group.user_id, &removed_pictures)?;

        debug!(
            "Propagating {} removed pictures to user {}",
            unaccessible_pictures.len(),
            shared_group.user_id
        );
        // Delete pictures from user groups
        Group::from_user_id_all(conn, shared_group.user_id)?
            .into_iter()
            .try_for_each(|group| group_remove_pictures(conn, group.id, &unaccessible_pictures))?;
    }
    Ok(())
}
