use crate::database::database::DBConn;
use crate::database::group::arrangement::ArrangementDetails;
use crate::database::group::group::Group;
use crate::database::tag::tag::Tag;
use crate::grouping::arrangement_strategy::ExifDataTypeValue;
use crate::grouping::group_by_exif_interval::ExifIntervalGrouping;
use crate::grouping::group_by_exif_value::ExifValuesGrouping;
use crate::grouping::group_by_filter::FilterGrouping;
use crate::grouping::group_by_location::LocationGrouping;
use crate::grouping::group_by_tag::TagGrouping;
use crate::grouping::strategy_filtering::StrategyFiltering;
use crate::utils::errors_catcher::ErrorResponder;
use itertools::Itertools;
use rocket::serde::{Deserialize, Serialize};
use rocket_okapi::JsonSchema;
use std::collections::{HashMap, HashSet};

/// Requirements:
/// - Give a list of pictures to group and group them
/// - Detect any picture that is in a group and should not be in it
pub trait StrategyGroupingTrait {
    /// Returns the list of existing groups ids from strategy. In most cases, it should match the list of groups in the database.
    fn get_groups(&self) -> Vec<i32>;

    /// Returns true if the strategy has been edited.
    /// Set ungroup to true if the pictures from picture_ids that do not match a group should be checked for removal.
    fn group_pictures(
        &mut self,
        conn: &mut DBConn,
        arrangement_id: i32,
        preserve_unicity: bool,
        ungroup_record: &mut UngroupRecord,
        picture_ids: &HashSet<i64>,
    ) -> Result<bool, ErrorResponder>;
}

/// Stores all pictures to ungroup, allowing to ungroup them only at the end.
pub struct UngroupRecord {
    pub enable: bool,
    pub map: HashMap<i32, HashSet<i64>>,
}
impl UngroupRecord {
    pub fn new(enable: bool) -> Self {
        UngroupRecord { enable, map: HashMap::new() }
    }
    pub fn add(&mut self, group_id: i32, picture_ids: HashSet<i64>) {
        if self.map.contains_key(&group_id) {
            self.map.get_mut(&group_id).unwrap().extend(picture_ids);
        } else {
            self.map.insert(group_id, picture_ids);
        }
    }
}

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
            StrategyGrouping::GroupByFilter(sg) => sg.get_groups(),
            StrategyGrouping::GroupByTags(sg) => sg.get_groups(),
            StrategyGrouping::GroupByExifValues(sg) => todo!(),
            StrategyGrouping::GroupByExifInterval(sg) => todo!(),
            StrategyGrouping::GroupByLocation(sg) => todo!(),
        }
    }
    pub fn get_dependant_groups(&self) -> Vec<i32> {
        let mut set = Vec::new();
        match self {
            StrategyGrouping::GroupByFilter(f) => {
                for filter in f.filters.values().into_iter() {
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
