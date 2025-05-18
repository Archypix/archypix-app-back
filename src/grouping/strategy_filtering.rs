use crate::database::database::DBConn;
use crate::database::schema::{pictures_tags, PictureOrientation};
use crate::grouping::arrangement_strategy::ExifDataTypeValue;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use diesel::dsl::{exists, not};
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::sql_types::Bool;
use diesel::QueryDsl;
use diesel::{BoxableExpression, ExpressionMethods};
use diesel::{Queryable, Selectable};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub enum StrategyFiltering {
    Or(Box<Vec<StrategyFiltering>>),
    And(Box<Vec<StrategyFiltering>>),
    Not(Box<StrategyFiltering>),
    Filter(FilterType),
}

type BoxedExpr = Box<dyn BoxableExpression<crate::database::schema::pictures::table, Pg, SqlType = Bool>>;
impl StrategyFiltering {
    pub fn filter_pictures(&self, conn: &mut DBConn, picture_ids: Option<&Vec<i64>>) -> Result<Vec<i64>, ErrorResponder> {
        use crate::database::schema::*;
        if let Some(picture_ids) = picture_ids {
            pictures::table.filter(pictures::id.eq_any(picture_ids)).into_boxed()
        } else {
            pictures::table.into_boxed()
        }
        .filter(self.as_diesel_predicate())
        .select(pictures::id)
        .load(conn)
        .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
    pub fn as_diesel_predicate(&self) -> BoxedExpr {
        let always_true = crate::database::schema::pictures::id.is_not_null();
        match self {
            StrategyFiltering::Or(filters) => {
                let mut or_conditions: Option<BoxedExpr> = None;
                for filter in filters.iter() {
                    let predicate = filter.as_diesel_predicate();
                    or_conditions = match or_conditions {
                        Some(cond) => Some(Box::new(cond.or(predicate))),
                        None => Some(predicate),
                    };
                }
                or_conditions.unwrap_or(Box::new(always_true))
            }
            StrategyFiltering::And(filters) => {
                let mut and_conditions: Option<BoxedExpr> = None;
                for filter in filters.iter() {
                    let predicate = filter.as_diesel_predicate();
                    and_conditions = match and_conditions {
                        Some(cond) => Some(Box::new(cond.and(predicate))),
                        None => Some(predicate),
                    };
                }
                and_conditions.unwrap_or(Box::new(always_true))
            }
            StrategyFiltering::Not(filter) => Box::new(not(filter.as_diesel_predicate())),
            StrategyFiltering::Filter(filter_type) => filter_type.clone().to_diesel_predicate(),
        }
    }

    pub fn and(self, other: StrategyFiltering) -> StrategyFiltering {
        StrategyFiltering::And(Box::new(vec![self, other]))
    }
    pub fn or(self, other: StrategyFiltering) -> StrategyFiltering {
        StrategyFiltering::Or(Box::new(vec![self, other]))
    }
    pub fn not(self) -> StrategyFiltering {
        StrategyFiltering::Not(Box::new(self))
    }

    pub fn get_all_filter_types(&self) -> Vec<FilterType> {
        let mut filter_types = Vec::new();
        self.collect_filter_types(&mut filter_types);
        filter_types
    }
    pub fn collect_filter_types(&self, filter_types: &mut Vec<FilterType>) {
        match self {
            StrategyFiltering::Or(filters) | StrategyFiltering::And(filters) => {
                for filter in filters.iter() {
                    filter.collect_filter_types(filter_types);
                }
            }
            StrategyFiltering::Not(filter) => {
                filter.collect_filter_types(filter_types);
            }
            StrategyFiltering::Filter(filter_type) => {
                filter_types.push(filter_type.clone());
            }
        }
    }

    pub fn get_dependant_groups(&self) -> Vec<i32> {
        let mut dependant_arrangements = Vec::new();
        for filter in self.get_all_filter_types().iter() {
            match filter {
                FilterType::IncludeGroups(groups) => dependant_arrangements.extend(groups.iter().cloned()),
                _ => {}
            }
        }
        dependant_arrangements
    }
    pub fn is_groups_dependant(&self) -> bool {
        self.get_all_filter_types().iter().any(|f| match f {
            FilterType::IncludeGroups(_) => true,
            _ => false,
        })
    }
    pub fn is_tags_dependant(&self) -> bool {
        self.get_all_filter_types().iter().any(|f| match f {
            FilterType::IncludeTags(_) => true,
            _ => false,
        })
    }
    pub fn is_exif_dependant(&self) -> bool {
        self.get_all_filter_types().iter().any(|f| match f {
            FilterType::ExifEqualTo(_) | FilterType::ExifInInterval(_) => true,
            _ => false,
        })
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub enum FilterType {
    IncludeTags(Vec<i32>),
    IncludeGroups(Vec<i32>),
    ExifEqualTo(ExifDataTypeValue),    // Equal to any of the values
    ExifInInterval(ExifDataTypeValue), // Interval composed of two first values
}
impl FilterType {
    pub fn to_strategy(self) -> StrategyFiltering {
        StrategyFiltering::Filter(self)
    }
    pub fn to_diesel_predicate(self) -> BoxedExpr {
        use crate::database::schema::*;
        let always_true = pictures::id.is_not_null();
        let always_false = pictures::id.is_null();
        match self {
            FilterType::IncludeTags(tags) => Box::new(exists(
                pictures_tags::table.filter(pictures_tags::picture_id.eq(pictures::id).and(pictures_tags::tag_id.eq_any(tags))),
            )),
            FilterType::IncludeGroups(groups) => Box::new(exists(
                groups_pictures::table.filter(groups_pictures::picture_id.eq(pictures::id).and(groups_pictures::group_id.eq_any(groups))),
            )),
            FilterType::ExifEqualTo(exif) => match exif {
                ExifDataTypeValue::CreationDate(dates) => Box::new(pictures::creation_date.eq_any(dates)),
                ExifDataTypeValue::EditionDate(dates) => Box::new(pictures::edition_date.eq_any(dates)),
                ExifDataTypeValue::Latitude(latitudes) => Box::new(
                    pictures::latitude
                        .is_not_null()
                        .and(pictures::latitude.assume_not_null().eq_any(latitudes)),
                ),
                ExifDataTypeValue::Longitude(longitudes) => Box::new(
                    pictures::longitude
                        .is_not_null()
                        .and(pictures::longitude.assume_not_null().eq_any(longitudes)),
                ),
                ExifDataTypeValue::Altitude(altitudes) => Box::new(
                    pictures::altitude
                        .is_not_null()
                        .and(pictures::altitude.assume_not_null().eq_any(altitudes)),
                ),
                ExifDataTypeValue::Orientation(orientations) => Box::new(pictures::orientation.eq_any(orientations)),
                ExifDataTypeValue::Width(widths) => Box::new(pictures::width.eq_any(widths)),
                ExifDataTypeValue::Height(heights) => Box::new(pictures::height.eq_any(heights)),
                ExifDataTypeValue::CameraBrand(brands) => Box::new(
                    pictures::camera_brand
                        .is_not_null()
                        .and(pictures::camera_brand.assume_not_null().eq_any(brands)),
                ),
                ExifDataTypeValue::CameraModel(models) => Box::new(
                    pictures::camera_model
                        .is_not_null()
                        .and(pictures::camera_model.assume_not_null().eq_any(models)),
                ),
                ExifDataTypeValue::FocalLength(focal_lengths) => Box::new(
                    pictures::focal_length
                        .is_not_null()
                        .and(pictures::focal_length.assume_not_null().eq_any(focal_lengths)),
                ),
                ExifDataTypeValue::ExposureTime(exposure_times) => {
                    let mut or_conditions: BoxedExpr = Box::new(always_false.clone());
                    for (num, den) in exposure_times {
                        let predicate = pictures::exposure_time_num
                            .eq(num)
                            .and(pictures::exposure_time_den.eq(den))
                            .assume_not_null();
                        or_conditions = Box::new(or_conditions.or(predicate))
                    }
                    Box::new(
                        pictures::exposure_time_num
                            .is_not_null()
                            .and(pictures::exposure_time_den.is_not_null())
                            .and(or_conditions.assume_not_null()),
                    )
                }
                ExifDataTypeValue::IsoSpeed(iso_speeds) => Box::new(
                    pictures::iso_speed
                        .is_not_null()
                        .and(pictures::iso_speed.assume_not_null().eq_any(iso_speeds)),
                ),
                ExifDataTypeValue::FNumber(f_numbers) => Box::new(
                    pictures::f_number
                        .is_not_null()
                        .and(pictures::f_number.assume_not_null().eq_any(f_numbers)),
                ),
            },
            _ => Box::new(always_true),
        }
    }
}
