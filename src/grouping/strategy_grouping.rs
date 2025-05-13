use crate::database::database::DBConn;
use crate::database::group::group::Group;
use crate::database::tag::tag::Tag;
use crate::grouping::arrangement_strategy::ExifDataTypeValue;
use crate::grouping::strategy_filtering::StrategyFiltering;
use crate::utils::errors_catcher::ErrorResponder;
use rocket::serde::{Deserialize, Serialize};
use rocket_okapi::JsonSchema;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub enum StrategyGrouping {
    GroupByFilter(FilterGrouping),
    GroupByTags(TagGrouping),
    GroupByExifValues(ExifValuesGrouping),
    GroupByExifInterval(ExifIntervalGrouping),
    GroupByLocation(LocationGrouping),
}

impl StrategyGrouping {
    pub fn get_groups(&self) -> Vec<i32> {
        match self {
            StrategyGrouping::GroupByFilter(f) => {
                let mut groups: Vec<i32> = f.filters.iter().map(|(_, id)| *id).collect();
                if let Some(id) = f.other_group_id {
                    (&mut groups).push(id);
                }
                groups
            }
            StrategyGrouping::GroupByTags(t) => {
                let mut groups: Vec<i32> = t.tag_id_to_group_id.values().cloned().collect();
                if let Some(id) = t.other_group_id {
                    (&mut groups).push(id);
                }
                groups
            }
            StrategyGrouping::GroupByExifValues(e) => e.values_to_group_id.clone(),
            StrategyGrouping::GroupByExifInterval(e) => {
                let mut groups: Vec<i32> = e.group_ids_decreasing.clone();
                groups.append(&mut e.group_ids_increasing.clone());
                groups
            }
            StrategyGrouping::GroupByLocation(l) => l.clusters_ids.clone(),
        }
    }
    pub fn get_dependant_groups(&self) -> Vec<i32> {
        let mut set = Vec::new();
        match self {
            StrategyGrouping::GroupByFilter(f) => {
                for (filter, _) in &f.filters {
                    set.extend(filter.get_dependant_groups());
                }
            }
            _ => {}
        }
        set
    }
    pub(crate) fn is_groups_dependant(&self) -> bool {
        match self {
            StrategyGrouping::GroupByFilter(f) => f.is_groups_dependant(),
            _ => false,
        }
    }
    pub(crate) fn is_tags_dependant(&self) -> bool {
        match self {
            StrategyGrouping::GroupByFilter(f) => f.is_tags_dependant(),
            StrategyGrouping::GroupByTags(_) => true,
            _ => false,
        }
    }
    pub(crate) fn is_exif_dependant(&self) -> bool {
        match self {
            StrategyGrouping::GroupByFilter(f) => f.is_exif_dependant(),
            StrategyGrouping::GroupByExifValues(_) | StrategyGrouping::GroupByExifInterval(_) | StrategyGrouping::GroupByLocation(_) => true,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FilterGrouping {
    pub filters: Vec<(StrategyFiltering, i32)>, // Value is the id of the corresponding group
    pub other_group_id: Option<i32>,            // Id of the group for the pictures that do not match any filter
}
impl FilterGrouping {
    pub fn get_or_create_other_group_id(&mut self, conn: &mut DBConn, arrangement_id: i32) -> Result<(i32, bool), ErrorResponder> {
        if let Some(id) = self.other_group_id {
            Ok((id, false))
        } else {
            let id = Group::insert(conn, arrangement_id, "Other".to_string(), false)?.id;
            self.other_group_id = Some(id);
            Ok((id, true))
        }
    }
    pub fn is_groups_dependant(&self) -> bool {
        self.filters.iter().any(|(f, _)| f.is_groups_dependant())
    }
    pub fn is_tags_dependant(&self) -> bool {
        self.filters.iter().any(|(f, _)| f.is_tags_dependant())
    }
    pub fn is_exif_dependant(&self) -> bool {
        self.filters.iter().any(|(f, _)| f.is_exif_dependant())
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TagGrouping {
    pub tag_group_id: i32,
    pub tag_id_to_group_id: HashMap<i32, i32>,
    pub other_group_id: Option<i32>,
    pub group_names_format: String,
}
impl TagGrouping {
    pub fn get_or_create_tag_group_id(&mut self, conn: &mut DBConn, tag: &Tag, arrangement_id: i32) -> Result<(i32, bool), ErrorResponder> {
        if let Some(id) = self.tag_id_to_group_id.get(&tag.id) {
            Ok((*id, false))
        } else {
            let id = Group::insert(conn, arrangement_id, self.format_group_name(&tag), false)?.id;
            self.other_group_id = Some(id);
            Ok((id, true))
        }
    }
    pub fn get_or_create_other_group_id(&mut self, conn: &mut DBConn, arrangement_id: i32) -> Result<(i32, bool), ErrorResponder> {
        if let Some(id) = self.other_group_id {
            Ok((id, false))
        } else {
            let id = Group::insert(conn, arrangement_id, "Other".to_string(), false)?.id;
            self.other_group_id = Some(id);
            Ok((id, true))
        }
    }
    pub fn format_group_name(&self, tag: &Tag) -> String {
        // TODO: implement formatting rule with self.group_names_format
        tag.name.clone()
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ExifValuesGrouping {
    pub data_type: ExifDataTypeValue, // data vec contains the values for each group
    pub values_to_group_id: Vec<i32>, // The value at index i is the id of the group for the value at index i in the data vec
    pub group_names_format: String,
    pub other_group_id: Option<i32>,
}
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExifIntervalGrouping {
    /* ... | interval -2 | interval -1 |origin| interval 1 | interval 2 | ...
     * ... | decreasing  | decreasing  |origin| increasing | increasing | ...
     * ... | index 1     | index 0     |origin| index 0    | index 1    | ...
     */
    pub interval: ExifDataTypeValue,    // First value is origin, second is interval
    pub group_ids_increasing: Vec<i32>, // ids of groups for intervals after the origin
    pub group_ids_decreasing: Vec<i32>, // ids of groups for intervals before the origin (in reverse order)
    pub group_names_format: String,     // Datetime format or number format.
}
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LocationGrouping {
    pub clusters_ids: Vec<i32>, // Ids of the groups for each cluster
    pub is_date_ordered: bool,
    pub sharpness: u32,
}
