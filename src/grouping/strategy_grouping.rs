use crate::api::groups::arrangement;
use crate::database::database::DBConn;
use crate::database::group::arrangement::{Arrangement, ArrangementDetails};
use crate::grouping::group_by_exif_interval::ExifIntervalGrouping;
use crate::grouping::group_by_exif_value::ExifValuesGrouping;
use crate::grouping::group_by_filter::{FilterGrouping, FilterGroupingRequest};
use crate::grouping::group_by_location::LocationGrouping;
use crate::grouping::group_by_tag::{TagGrouping, TagGroupingRequest};
use crate::utils::errors_catcher::ErrorResponder;
use enum_kinds::EnumKind;
use rocket::serde::{Deserialize, Serialize};
use rocket_okapi::JsonSchema;
use std::collections::{HashMap, HashSet};

/// Requirements:
/// - Give a list of pictures to group and group them
/// - Detect any picture that is in a group and should not be in it
pub trait StrategyGroupingTrait {
    type Request: Serialize + Deserialize<'static> + JsonSchema;

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

    /// Create a new strategy grouping from the request, creating the required groups if needed.
    fn create(conn: &mut DBConn, arrangement_id: i32, request: &Self::Request) -> Result<Box<Self>, ErrorResponder>;
    /// Edit the strategy grouping, marking groups of the old strategy that can’t match any group of the new strategy as "to be deleted".
    fn edit(&mut self, conn: &mut DBConn, arrangement_id: i32, request: &Self::Request) -> Result<(), ErrorResponder>;
    /// Mark all groups as "to be deleted" in the database, allowing the strategy to be deleted (and replaced by another one).
    fn delete(&self, conn: &mut DBConn, arrangement_id: i32) -> Result<(), ErrorResponder>;
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
        if !self.enable {
            return;
        }
        if self.map.contains_key(&group_id) {
            self.map.get_mut(&group_id).unwrap().extend(picture_ids);
        } else {
            self.map.insert(group_id, picture_ids);
        }
    }
}

#[derive(EnumKind, Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[enum_kind(StrategyGroupingKind, derive(Serialize, Deserialize, JsonSchema))]
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

    pub fn delete(&self, conn: &mut DBConn, arrangement_id: i32) -> Result<(), ErrorResponder> {
        match self {
            StrategyGrouping::GroupByFilter(f) => f.delete(conn, arrangement_id),
            StrategyGrouping::GroupByTags(t) => t.delete(conn, arrangement_id),
            StrategyGrouping::GroupByExifValues(_) | StrategyGrouping::GroupByExifInterval(_) | StrategyGrouping::GroupByLocation(_) => todo!(),
        }
    }

    pub fn edit_strategy_grouping(
        &self,
        conn: &mut DBConn,
        arrangement_id: i32,
        new_grouping: &StrategyGroupingRequest,
    ) -> Result<StrategyGrouping, ErrorResponder> {
        match (self, new_grouping) {
            (StrategyGrouping::GroupByFilter(old), StrategyGroupingRequest::GroupByFilter(req)) => {
                let mut new = old.clone();
                new.edit(conn, arrangement_id, req)?;
                Ok(StrategyGrouping::GroupByFilter(new))
            }
            (StrategyGrouping::GroupByTags(old), StrategyGroupingRequest::GroupByTags(req)) => {
                let mut new = old.clone();
                new.edit(conn, arrangement_id, req)?;
                Ok(StrategyGrouping::GroupByTags(new))
            }
            _ => {
                // Different types - delete old and create new
                self.delete(conn, arrangement_id)?;
                new_grouping.create_strategy_grouping(conn, arrangement_id)
            }
        }
    }
}

#[derive(EnumKind, Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[enum_kind(StrategyGroupingRequestKind, derive(Serialize, Deserialize, JsonSchema))]
pub enum StrategyGroupingRequest {
    GroupByFilter(FilterGroupingRequest),
    GroupByTags(TagGroupingRequest),
}

impl StrategyGroupingRequest {
    pub fn create_strategy_grouping(&self, conn: &mut DBConn, arrangement_id: i32) -> Result<StrategyGrouping, ErrorResponder> {
        match self {
            StrategyGroupingRequest::GroupByFilter(request) => {
                let grouping = FilterGrouping::create(conn, arrangement_id, request)?;
                Ok(StrategyGrouping::GroupByFilter(*grouping))
            }
            StrategyGroupingRequest::GroupByTags(request) => {
                let grouping = TagGrouping::create(conn, arrangement_id, request)?;
                Ok(StrategyGrouping::GroupByTags(*grouping))
            }
        }
    }
}
