use crate::database::schema::PictureOrientation;
use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct GroupingStrategy {
    filter: GroupingFilterStrategy,
    groupings: GroupingType,
    preserve_unicity: bool, // If true, a picture will not be able to appear in two different groups.
}

impl GroupingStrategy {
    pub fn is_groups_dependant(&self) -> bool {
        self.filter.is_groups_dependant() || self.groupings.is_groups_dependant()
    }
    pub fn is_tags_dependant(&self) -> bool {
        self.filter.is_tags_dependant() || self.groupings.is_tags_dependant()
    }
    pub fn is_exif_dependant(&self) -> bool {
        self.filter.is_exif_dependant() || self.groupings.is_exif_dependant()
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GroupingFilterStrategy {
    filters: Vec<Vec<FilterType>>, // Filters are stored as a list of filters to apply in disjunctive normal form.
}

impl GroupingFilterStrategy {
    pub(crate) fn is_groups_dependant(&self) -> bool {
        self.filters.iter().any(|filter| {
            filter.iter().any(|f| match f {
                FilterType::IncludeGroups(_) | FilterType::ExcludeGroups(_) => true,
                _ => false,
            })
        })
    }
    pub(crate) fn is_tags_dependant(&self) -> bool {
        self.filters.iter().any(|filter| {
            filter.iter().any(|f| match f {
                FilterType::IncludeTags(_) | FilterType::ExcludeTags(_) => true,
                _ => false,
            })
        })
    }
    pub(crate) fn is_exif_dependant(&self) -> bool {
        self.filters.iter().any(|filter| {
            filter.iter().any(|f| match f {
                FilterType::ExifEqualTo(_) | FilterType::ExifNotEqualTo(_) | FilterType::ExifInInterval(_) | FilterType::ExifNotInInterval(_) => true,
                _ => false,
            })
        })
    }
}
// EXIF RELATED DATA

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ExifDataTypeValue {
    CreationDate(Vec<NaiveDateTime>),
    EditionDate(Vec<NaiveDateTime>),
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
    IncludeTags(Vec<u32>),
    ExcludeTags(Vec<u32>),
    IncludeGroups(Vec<u32>),
    ExcludeGroups(Vec<u32>),
    ExifEqualTo(ExifDataTypeValue),       // Equal to any of the values
    ExifNotEqualTo(ExifDataTypeValue),    // Different from any of the values
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

impl GroupingType {
    pub(crate) fn is_groups_dependant(&self) -> bool {
        match self {
            GroupingType::GroupByFilter(f) => f.is_groups_dependant(),
            _ => false,
        }
    }
    pub(crate) fn is_tags_dependant(&self) -> bool {
        match self {
            GroupingType::GroupByFilter(f) => f.is_tags_dependant(),
            GroupingType::GroupByTags(_) => true,
            _ => false,
        }
    }
    pub(crate) fn is_exif_dependant(&self) -> bool {
        match self {
            GroupingType::GroupByFilter(f) => f.is_exif_dependant(),
            GroupingType::GroupByExifValues(_) | GroupingType::GroupByExifInterval(_) | GroupingType::GroupByLocation(_) => true,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FilterGrouping {
    filters: Vec<(GroupingFilterStrategy, u32)>, // Value is the id of the corresponding group
    other_group_id: Option<u32>,                 // Id of the group for the pictures that do not match any filter
}
impl FilterGrouping {
    pub(crate) fn is_groups_dependant(&self) -> bool {
        self.filters.iter().any(|(f, _)| f.is_groups_dependant())
    }
    pub(crate) fn is_tags_dependant(&self) -> bool {
        self.filters.iter().any(|(f, _)| f.is_tags_dependant())
    }
    pub(crate) fn is_exif_dependant(&self) -> bool {
        self.filters.iter().any(|(f, _)| f.is_exif_dependant())
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TagGrouping {
    tag_group_id: u32,
    tag_id_to_group_id: HashMap<u32, u32>,
    others_group_id: Option<u32>,
    group_names_format: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ExifValuesGrouping {
    data_type: ExifDataTypeValue, // data vec contains the values for each group
    values_to_group_id: Vec<u32>, // The value at index i is the id of the group for the value at index i in the data vec
    group_names_format: String,
    other_group_id: Option<u32>,
}
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExifIntervalGrouping {
    /* ... | interval -2 | interval -1 |origin| interval 1 | interval 2 | ...
     * ... | decreasing  | decreasing  |origin| increasing | increasing | ...
     * ... | index 1     | index 0     |origin| index 0    | index 1    | ...
     */
    interval: ExifDataTypeValue,    // First value is origin, second is interval
    group_ids_increasing: Vec<u32>, // ids of groups for intervals after the origin
    group_ids_decreasing: Vec<u32>, // ids of groups for intervals before the origin (in reverse order)
    other_group_id: Option<u32>,    // id of the group for the pictures that do not match any interval (if any)
    group_names_format: String,     // Datetime format or number format.
}
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LocationGrouping {
    clusters_ids: Vec<u64>,
    is_date_ordered: bool,
    sharpness: u32,
}
