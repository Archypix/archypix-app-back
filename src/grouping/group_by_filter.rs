use crate::database::database::DBConn;
use crate::database::group::arrangement::ArrangementDetails;
use crate::database::group::group::Group;
use crate::grouping::grouping_process::{group_add_pictures, group_remove_pictures};
use crate::grouping::strategy_filtering::StrategyFiltering;
use crate::grouping::strategy_grouping::{StrategyGroupingTrait, UngroupRecord};
use crate::utils::errors_catcher::ErrorResponder;
use itertools::Itertools;
use rocket::serde::{Deserialize, Serialize};
use rocket_okapi::JsonSchema;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FilterGroupingRequest {
    pub filters: HashMap<String, StrategyFiltering>, // Key is the group name, value is the filter
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FilterGrouping {
    pub filters: HashMap<i32, StrategyFiltering>, // Key is the group id, value is the filter
    pub other_group_id: Option<i32>,              // Id of the group for the pictures that do not match any filter
}
impl FilterGrouping {
    fn get_or_create_other_group(&mut self, conn: &mut DBConn, arrangement_id: i32) -> Result<(i32, bool), ErrorResponder> {
        if let Some(id) = self.other_group_id {
            Ok((id, false))
        } else {
            let id = Group::insert(conn, arrangement_id, "Other".to_string(), false)?.id;
            self.other_group_id = Some(id);
            Ok((id, true))
        }
    }
    pub fn is_groups_dependant(&self) -> bool {
        self.filters.values().into_iter().any(|f| f.is_groups_dependant())
    }
    pub fn is_tags_dependant(&self) -> bool {
        self.filters.values().into_iter().any(|f| f.is_tags_dependant())
    }
    pub fn is_exif_dependant(&self) -> bool {
        self.filters.values().into_iter().any(|f| f.is_exif_dependant())
    }
}
impl StrategyGroupingTrait for FilterGrouping {
    type Request = FilterGroupingRequest;

    fn get_groups(&self) -> Vec<i32> {
        let mut groups: Vec<i32> = self.filters.keys().cloned().collect();
        if let Some(id) = self.other_group_id {
            (&mut groups).push(id);
        }
        groups
    }

    fn group_pictures(
        &mut self,
        conn: &mut DBConn,
        arrangement_id: i32,
        preserve_unicity: bool,
        ungroup_record: &mut UngroupRecord,
        picture_ids: &HashSet<i64>,
    ) -> Result<bool, ErrorResponder> {
        let mut update_strategy = false;
        let mut remaining_pictures_ids = picture_ids.clone();

        for (group_id, filter) in &self.filters {
            let pictures_to_group = if preserve_unicity { &remaining_pictures_ids } else { &picture_ids };

            let group_pictures: HashSet<i64> = HashSet::from_iter(
                filter
                    .filter_pictures(conn, Some(&pictures_to_group.iter().cloned().collect_vec()))?
                    .into_iter(),
            );
            remaining_pictures_ids = remaining_pictures_ids.difference(&group_pictures).cloned().collect();

            group_add_pictures(conn, *group_id, &group_pictures.iter().cloned().collect_vec())?;
            if ungroup_record.enable {
                let ungroup_pictures = picture_ids.difference(&group_pictures).cloned().collect();
                ungroup_record.add(*group_id, ungroup_pictures);
            }
        }
        if remaining_pictures_ids.len() != 0 {
            let (other_group_id, group_created) = self.get_or_create_other_group(conn, arrangement_id)?;
            update_strategy = group_created;
            group_add_pictures(conn, other_group_id, &remaining_pictures_ids.iter().cloned().collect_vec())?;
        }
        // If the other group is not just created, and there is an other group, remove the other group pictures.
        if ungroup_record.enable && !update_strategy && self.other_group_id.is_some() {
            let ungroup_pictures = picture_ids.difference(&remaining_pictures_ids).cloned().collect();
            ungroup_record.add(self.other_group_id.unwrap(), ungroup_pictures);
        }
        Ok(update_strategy)
    }

    fn create(conn: &mut DBConn, arrangement_id: i32, request: &Self::Request) -> Result<Box<Self>, ErrorResponder> {
        todo!()
    }

    fn edit(&mut self, conn: &mut DBConn, arrangement_id: i32, request: &Self::Request) -> Result<(), ErrorResponder> {
        todo!()
    }

    fn delete(&self, conn: &mut DBConn, arrangement_id: i32) -> Result<(), ErrorResponder> {
        todo!()
    }
}
