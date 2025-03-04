use crate::database::database::DBConn;
use crate::database::group::arrangement::Arrangement;
use crate::database::picture::picture::Picture;
use crate::database::schema::pictures::dsl::pictures;
use crate::database::schema::*;
use crate::database::schema::{groups_pictures, pictures_tags, PictureOrientation};
use crate::grouping::grouping_strategy::ExifDataTypeValue;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use diesel::dsl::{exists, not};
use diesel::mysql::Mysql;
use diesel::prelude::*;
use diesel::sql_types::Bool;
use diesel::QueryDsl;
use diesel::{Associations, Identifiable, Queryable, Selectable};
use diesel::{BoxableExpression, ExpressionMethods};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GroupingFilterStrategy {
    pub filters: Vec<Vec<FilterType>>, // Filters are stored as a list of filters to apply in disjunctive normal form.
}

impl GroupingFilterStrategy {
    pub fn filter_pictures(&self, conn: &mut DBConn, picture_ids: &Vec<u64>) -> Result<Vec<u64>, ErrorResponder> {
        use crate::database::schema::*;
        let mut req = pictures::table.filter(pictures::id.eq_any(picture_ids)).into_boxed();

        type BoxedExpr = Box<dyn BoxableExpression<pictures::table, Mysql, SqlType = Bool>>;

        // Apply with OR and then AND (in DNF)
        let mut or_conditions: Option<BoxedExpr> = None;
        for filters in self.filters.clone() {
            let mut and_conditions: Option<BoxedExpr> = None;
            for filter in filters {
                let predicate: BoxedExpr = filter.get_filter_dsl_predicate();
                and_conditions = match and_conditions {
                    Some(cond) => Some(Box::new(cond.and(predicate))),
                    None => Some(predicate),
                };
            }
            if let Some(predicate) = and_conditions {
                or_conditions = match or_conditions {
                    Some(cond) => Some(Box::new(cond.or(predicate))),
                    None => Some(predicate),
                };
            }
        }
        if let Some(or_cond) = or_conditions {
            req = req.filter(or_cond);
        }

        req.select((pictures::id))
            .load(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
    pub fn get_dependant_arrangements(&self) -> HashSet<u32> {
        let mut dependant_arrangements = HashSet::new();
        for filters in self.filters.iter() {
            for filter in filters.iter() {
                match filter {
                    FilterType::IncludeGroups(groups) | FilterType::ExcludeGroups(groups) => dependant_arrangements.extend(groups.iter().cloned()),
                    _ => {}
                }
            }
        }
        dependant_arrangements
    }
    pub fn is_groups_dependant(&self) -> bool {
        self.filters.iter().any(|filter| {
            filter.iter().any(|f| match f {
                FilterType::IncludeGroups(_) | FilterType::ExcludeGroups(_) => true,
                _ => false,
            })
        })
    }
    pub fn is_tags_dependant(&self) -> bool {
        self.filters.iter().any(|filter| {
            filter.iter().any(|f| match f {
                FilterType::IncludeTags(_) | FilterType::ExcludeTags(_) => true,
                _ => false,
            })
        })
    }
    pub fn is_exif_dependant(&self) -> bool {
        self.filters.iter().any(|filter| {
            filter.iter().any(|f| match f {
                FilterType::ExifEqualTo(_) | FilterType::ExifNotEqualTo(_) | FilterType::ExifInInterval(_) | FilterType::ExifNotInInterval(_) => true,
                _ => false,
            })
        })
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub enum FilterType {
    IncludeTags(Vec<u32>),
    ExcludeTags(Vec<u32>),
    IncludeGroups(Vec<u32>),
    ExcludeGroups(Vec<u32>),
    ExifEqualTo(ExifDataTypeValue),       // Equal to any of the values
    ExifNotEqualTo(ExifDataTypeValue),    // Different from any of the values
    ExifInInterval(ExifDataTypeValue),    // Interval composed of two first values
    ExifNotInInterval(ExifDataTypeValue), // Interval composed of two first values
}
type PicturesBoxedExpr = Box<dyn BoxableExpression<crate::database::schema::pictures::table, Mysql, SqlType = Bool>>;
impl FilterType {
    pub fn get_filter_dsl_predicate(self) -> PicturesBoxedExpr {
        use crate::database::schema::*;
        let always_true = pictures::id.is_not_null();
        let always_false = pictures::id.is_null();
        match self {
            FilterType::IncludeTags(tags) => Box::new(exists(
                pictures_tags::table.filter(pictures_tags::picture_id.eq(pictures::id).and(pictures_tags::tag_id.eq_any(tags))),
            )),
            FilterType::ExcludeTags(tags) => Box::new(not(exists(
                pictures_tags::table.filter(pictures_tags::picture_id.eq(pictures::id).and(pictures_tags::tag_id.eq_any(tags))),
            ))),
            FilterType::IncludeGroups(groups) => Box::new(exists(
                groups_pictures::table.filter(groups_pictures::picture_id.eq(pictures::id).and(groups_pictures::group_id.eq_any(groups))),
            )),
            FilterType::ExcludeGroups(groups) => Box::new(not(exists(
                groups_pictures::table.filter(groups_pictures::picture_id.eq(pictures::id).and(groups_pictures::group_id.eq_any(groups))),
            ))),
            FilterType::ExifEqualTo(exif) => match exif {
                ExifDataTypeValue::CreationDate(dates) => Box::new(pictures::creation_date.eq_any(dates)),
                ExifDataTypeValue::EditionDate(dates) => Box::new(pictures::edition_date.eq_any(dates)),
                ExifDataTypeValue::Latitude(latitudes) => Box::new(pictures::latitude.eq_any(latitudes)),
                ExifDataTypeValue::Longitude(longitudes) => Box::new(pictures::longitude.eq_any(longitudes)),
                ExifDataTypeValue::Altitude(altitudes) => Box::new(pictures::altitude.eq_any(altitudes)),
                ExifDataTypeValue::Orientation(orientations) => Box::new(pictures::orientation.eq_any(orientations)),
                ExifDataTypeValue::Width(widths) => Box::new(pictures::width.eq_any(widths)),
                ExifDataTypeValue::Height(heights) => Box::new(pictures::height.eq_any(heights)),
                ExifDataTypeValue::CameraBrand(brands) => Box::new(pictures::camera_brand.eq_any(brands)),
                ExifDataTypeValue::CameraModel(models) => Box::new(pictures::camera_model.eq_any(models)),
                ExifDataTypeValue::FocalLength(focal_lengths) => Box::new(pictures::focal_length.eq_any(focal_lengths)),
                ExifDataTypeValue::ExposureTime(exposure_times) => {
                    let mut or_conditions: PicturesBoxedExpr = Box::new(always_false.clone());
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
                            .and(or_conditions),
                    )
                }
                ExifDataTypeValue::IsoSpeed(iso_speeds) => Box::new(pictures::iso_speed.eq_any(iso_speeds)),
                ExifDataTypeValue::FNumber(f_numbers) => Box::new(pictures::f_number.eq_any(f_numbers)),
            },
            FilterType::ExifNotEqualTo(exif) => match exif {
                ExifDataTypeValue::CreationDate(dates) => Box::new(not(pictures::creation_date.eq_any(dates))),
                ExifDataTypeValue::EditionDate(dates) => Box::new(not(pictures::edition_date.eq_any(dates))),
                ExifDataTypeValue::Latitude(latitudes) => Box::new(not(pictures::latitude.eq_any(latitudes))),
                ExifDataTypeValue::Longitude(longitudes) => Box::new(not(pictures::longitude.eq_any(longitudes))),
                ExifDataTypeValue::Altitude(altitudes) => Box::new(not(pictures::altitude.eq_any(altitudes))),
                ExifDataTypeValue::Orientation(orientations) => Box::new(not(pictures::orientation.eq_any(orientations))),
                ExifDataTypeValue::Width(widths) => Box::new(not(pictures::width.eq_any(widths))),
                ExifDataTypeValue::Height(heights) => Box::new(not(pictures::height.eq_any(heights))),
                ExifDataTypeValue::CameraBrand(brands) => Box::new(not(pictures::camera_brand.eq_any(brands))),
                ExifDataTypeValue::CameraModel(models) => Box::new(not(pictures::camera_model.eq_any(models))),
                ExifDataTypeValue::FocalLength(focal_lengths) => Box::new(not(pictures::focal_length.eq_any(focal_lengths))),
                ExifDataTypeValue::ExposureTime(exposure_times) => {
                    let mut and_conditions: PicturesBoxedExpr =
                        Box::new(pictures::exposure_time_num.is_not_null().and(pictures::exposure_time_den.is_not_null()));
                    for (num, den) in exposure_times {
                        let predicate = not(pictures::exposure_time_num
                            .eq(num)
                            .and(pictures::exposure_time_den.eq(den))
                            .assume_not_null());
                        and_conditions = Box::new(and_conditions.and(predicate))
                    }
                    and_conditions
                }
                ExifDataTypeValue::IsoSpeed(iso_speeds) => Box::new(not(pictures::iso_speed.eq_any(iso_speeds))),
                ExifDataTypeValue::FNumber(f_numbers) => Box::new(not(pictures::f_number.eq_any(f_numbers))),
            },
            _ => Box::new(always_true),
        }
    }
}
