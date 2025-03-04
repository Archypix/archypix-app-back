use crate::database::picture::picture::Picture;
use crate::database::schema::PictureOrientation;
use crate::grouping::grouping_filter_strategy::GroupingFilterStrategy;
use crate::grouping::grouping_type::GroupingType;
use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct GroupingStrategy {
    pub filter: GroupingFilterStrategy,
    pub groupings: GroupingType,
    pub preserve_unicity: bool, // If true, a picture will not be able to appear in two different groups.
}

impl GroupingStrategy {
    pub fn get_dependant_arrangements(&self) -> Vec<u32> {
        let mut dependant_arrangements = self.filter.get_dependant_arrangements();
        dependant_arrangements.extend(self.groupings.get_dependant_arrangements());
        dependant_arrangements.into_iter().collect()
    }
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

// EXIF RELATED DATA

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ExifDataTypeValue {
    CreationDate(Vec<NaiveDateTime>),
    EditionDate(Vec<NaiveDateTime>),
    Latitude(Vec<BigDecimal>),
    Longitude(Vec<BigDecimal>),
    Altitude(Vec<u16>),
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
