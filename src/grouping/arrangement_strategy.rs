use crate::database::database::DBConn;
use crate::database::group::arrangement::Arrangement;
use crate::database::picture::picture::Picture;
use crate::database::schema::PictureOrientation;
use crate::grouping::strategy_filtering::StrategyFiltering;
use crate::grouping::strategy_grouping::StrategyGrouping;
use crate::utils::errors_catcher::ErrorResponder;
use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ArrangementStrategy {
    pub filter: StrategyFiltering,
    pub groupings: StrategyGrouping,
    pub preserve_unicity: bool, // If true, a picture will not be able to appear in two different groups.
}

impl ArrangementStrategy {
    pub fn get_dependant_arrangements(&self, conn: &mut DBConn) -> Result<Vec<u32>, ErrorResponder> {
        let mut dependant_groups = self.filter.get_dependant_groups();
        dependant_groups.extend(self.groupings.get_dependant_groups());
        Arrangement::get_arrangements_from_groups_ids(conn, dependant_groups).map(|arrangements| arrangements.iter().map(|a| a.id).collect())
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
