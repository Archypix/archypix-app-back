use crate::database::database::DBConn;
use crate::database::group::group::Group;
use crate::grouping::grouping_process::group_add_pictures;
use crate::grouping::strategy_filtering::StrategyFiltering;
use crate::grouping::strategy_grouping::{StrategyGroupingTrait, UngroupRecord};
use crate::utils::errors_catcher::ErrorResponder;
use indexmap::IndexMap;
use itertools::Itertools;
use rocket::serde::{Deserialize, Serialize};
use rocket_okapi::JsonSchema;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FilterGroupingRequest {
    pub filters: Vec<FilterGroupingValueRequest>,
}
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FilterGroupingValueRequest {
    pub id: i32, // <= 0 for new groups
    pub name: String,
    pub filter: StrategyFiltering,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FilterGrouping {
    pub filters: Vec<(i32, StrategyFiltering)>, // (group_id, filter)
    pub other_group_id: Option<i32>,            // Id of the group for the pictures that do not match any filter
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
        self.filters.iter().any(|f| f.1.is_groups_dependant())
    }
    pub fn is_tags_dependant(&self) -> bool {
        self.filters.iter().any(|f| f.1.is_tags_dependant())
    }
    pub fn is_exif_dependant(&self) -> bool {
        self.filters.iter().any(|f| f.1.is_exif_dependant())
    }
}
impl StrategyGroupingTrait for FilterGrouping {
    type Request = FilterGroupingRequest;

    fn get_groups(&self) -> Vec<i32> {
        let mut groups: Vec<i32> = self.filters.iter().map(|f| f.0).collect();
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

    /// Create one group per filter and no other group by default.
    fn create(conn: &mut DBConn, arrangement_id: i32, request: &Self::Request) -> Result<Box<Self>, ErrorResponder> {
        let filters = request
            .filters
            .iter()
            .map(|value| {
                let group = Group::insert(conn, arrangement_id, value.name.clone(), false)?;
                Ok((group.id, value.filter.clone()))
            })
            .collect::<Result<Vec<(i32, StrategyFiltering)>, ErrorResponder>>()?;
        Ok(Box::new(FilterGrouping {
            filters,
            other_group_id: None,
        }))
    }

    /// Tries to match FilteringStrategy to the existing ones.
    /// Mark unmatched groups as "to be deleted" in the database.
    /// Create new groups for unmatched new groups.
    fn edit(&mut self, conn: &mut DBConn, arrangement_id: i32, request: &Self::Request) -> Result<(), ErrorResponder> {
        let mut request = request.clone();
        let old_groups_ids = self.filters.iter().map(|f| f.0).collect_vec();

        // Editing existing groups and delete unmatched ones
        old_groups_ids.iter().try_for_each(|group_id| {
            if let Some(value) = request.filters.iter().find(|v| v.id == *group_id) {
                Group::rename(conn, *group_id, value.name.clone())?;
                self.filters.iter_mut().find(|f| f.0 == *group_id).map(|f| f.1 = value.filter.clone());
            } else {
                Group::mark_as_to_be_deleted(conn, *group_id)?;
                self.filters.retain(|f| f.0 != *group_id);
            }
            Ok::<(), ErrorResponder>(())
        })?;

        // Create new groups (with id <= 0 or unmatched)
        request.filters.iter_mut().try_for_each(|value| {
            if value.id <= 0 || !self.filters.iter().any(|f| f.0 == value.id) {
                let group = Group::insert(conn, arrangement_id, value.name.clone(), false)?;
                self.filters.push((group.id, value.filter.clone()));
                value.id = group.id;
            }
            Ok::<(), ErrorResponder>(())
        })?;

        // Sort groups in the order of the request
        self.filters = self
            .filters
            .clone()
            .into_iter()
            .sorted_by(|a, b| {
                if let Some(i) = request.filters.iter().position(|v| v.id == a.0) {
                    if let Some(i2) = request.filters.iter().position(|v| v.id == b.0) {
                        return i.cmp(&i2);
                    }
                    return Ordering::Less;
                }
                Ordering::Greater
            })
            .collect();
        Ok(())
    }
    /// Marks all groups as "to be deleted" in the database, allowing the strategy to be deleted (and replaced by another one).
    fn delete(&self, conn: &mut DBConn, arrangement_id: i32) -> Result<(), ErrorResponder> {
        for group_id in self.get_groups() {
            Group::mark_as_to_be_deleted(conn, group_id)?;
        }
        Ok(())
    }
}
