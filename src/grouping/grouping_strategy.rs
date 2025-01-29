use crate::database::schema::PictureOrientation;
use bigdecimal::BigDecimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct GroupingStrategy {
    is_manual_grouping: bool,       // If true, the user will be able to create groups manually
    filter: GroupingFilterStrategy, // Apply in disjunctive normal form
    groupings: Vec<GroupingType>,   // Create intersection between all groups generated by the GroupingType
    preserve_unicity: bool,         // If true, a picture will not be able to appear in two different groups.
}

impl GroupingStrategy {
    pub fn is_manual(&self) -> bool {
        self.is_manual_grouping
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GroupingFilterStrategy {
    filters: Vec<Vec<FilterType>>, // Filters are stored as a list of filters to apply in disjunctive normal form.
}

// EXIF RELATED DATA

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ExifDataTypeValue {
    // CreationDate(Vec<NaiveDateTime>),
    // EditionDate(Vec<NaiveDateTime>),
    Latitude(Vec<BigDecimal>),
    Longitude(Vec<BigDecimal>),
    Altitude(Vec<BigDecimal>),
    Orientation(Vec<PictureOrientation>),
    Width(Vec<u16>),
    Height(Vec<u16>),
    CameraBrand(Vec<String>),
    CameraModel(Vec<String>),
    FocalLength(Vec<BigDecimal>),
    ExposureTime(Vec<(u32, u32)>),
    IsoSpeed(Vec<u32>),
    FNumber(Vec<BigDecimal>),
}

// FILTERING
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub enum FilterType {
    All,
    IncludeTags(Vec<[u8; 16]>),
    ExcludeTags(Vec<[u8; 16]>),
    IncludeSubgroups(Vec<[u8; 16]>),
    ExcludeSubgroups(Vec<[u8; 16]>),
    ExifEqualTo(ExifDataTypeValue),       // Equal to any of the values
    ExifNotEqualTo(ExifDataTypeValue),    // Not equal to all the values
    ExifInInterval(ExifDataTypeValue),    // Interval composed of two first values
    ExifNotInInterval(ExifDataTypeValue), // Interval composed of two first values
}

// GROUPING
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub enum GroupingType {
    GroupByFilter(FilterGrouping),
    GroupByTags(TagGrouping),
    GroupByExifValues(ExifValuesGrouping),
    GroupByExifInterval(ExifIntervalGrouping),
    GroupByLocation(LocationGrouping),
}
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FilterGrouping {
    filters: Vec<(GroupingFilterStrategy, u64)>, // Value is the id of the corresponding subgroup
}
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TagGrouping {
    tag_group_id: u64,
    tag_id_to_subgroup_id: HashMap<u64, u64>,
    subgroup_names_format: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ExifValuesGrouping {
    data_type: ExifDataTypeValue,    // data vec contains the values for each group
    values_to_subgroup_id: Vec<u32>, // The value at index i is the id of the group for the value at index i in the data vec
    subgroup_names_format: String,
}
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExifIntervalGrouping {
    interval: ExifDataTypeValue,   // First value is origin, second is interval
    subgroup_names_format: String, // Datetime format or number format.
}
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LocationGrouping {
    clusters_ids: Vec<u64>,
    is_date_ordered: bool,
    sharpness: u32,
}
