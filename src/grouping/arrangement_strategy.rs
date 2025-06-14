use crate::database::database::DBConn;
use crate::database::group::arrangement::{Arrangement, ArrangementDependencyType};
use crate::database::schema::PictureOrientation;
use crate::grouping::strategy_filtering::StrategyFiltering;
use crate::grouping::strategy_grouping::{StrategyGrouping, StrategyGroupingRequest};
use crate::utils::errors_catcher::ErrorResponder;
use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ArrangementStrategy {
    pub filter: StrategyFiltering,
    pub groupings: StrategyGrouping,
    pub preserve_unicity: bool, // If true, a picture will not be able to appear in two different groups.
}

impl ArrangementStrategy {
    pub fn get_dependant_arrangements(&self, conn: &mut DBConn) -> Result<Vec<i32>, ErrorResponder> {
        Arrangement::get_arrangements_from_groups_ids(conn, self.get_dependant_groups())
            .map(|arrangements| arrangements.iter().map(|a| a.id).collect())
    }
    /// Get the groups ids on which the strategy depends.
    pub fn get_dependant_groups(&self) -> Vec<i32> {
        let mut dependant_groups = self.filter.get_dependant_groups();
        dependant_groups.extend(self.groupings.get_dependant_groups());
        dependant_groups
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

    pub fn get_dependency_type(&self) -> ArrangementDependencyType {
        ArrangementDependencyType {
            groups_dependant: self.is_groups_dependant(),
            tags_dependant: self.is_tags_dependant(),
            exif_dependant: self.is_exif_dependant(),
        }
    }
}

// EXIF RELATED DATA

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub enum ExifDataTypeValue {
    CreationDate(Vec<NaiveDateTime>),
    EditionDate(Vec<NaiveDateTime>),
    Latitude(Vec<BigDecimal>),
    Longitude(Vec<BigDecimal>),
    Altitude(Vec<i16>),
    Orientation(Vec<PictureOrientation>),
    Width(Vec<i16>),
    Height(Vec<i16>),
    CameraBrand(Vec<String>),
    CameraModel(Vec<String>),
    FocalLength(Vec<BigDecimal>),
    ExposureTime(Vec<(i32, i32)>),
    IsoSpeed(Vec<i32>),
    FNumber(Vec<BigDecimal>),
}

// Requests

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ArrangementStrategyRequest {
    pub filter: StrategyFiltering,
    pub groupings: StrategyGroupingRequest,
    pub preserve_unicity: bool,
}

impl ArrangementStrategyRequest {
    pub fn create(&self, conn: &mut DBConn, arrangement_id: i32) -> Result<ArrangementStrategy, ErrorResponder> {
        let groupings = self.groupings.create_strategy_grouping(conn, arrangement_id)?;
        Ok(ArrangementStrategy {
            filter: self.filter.clone(),
            groupings,
            preserve_unicity: self.preserve_unicity,
        })
    }
    pub fn edit(&self, conn: &mut DBConn, arrangement_id: i32, old_strategy: ArrangementStrategy) -> Result<ArrangementStrategy, ErrorResponder> {
        let groupings = old_strategy.groupings.edit_strategy_grouping(conn, arrangement_id, &self.groupings)?;
        Ok(ArrangementStrategy {
            filter: self.filter.clone(),
            groupings,
            preserve_unicity: self.preserve_unicity,
        })
    }
}
