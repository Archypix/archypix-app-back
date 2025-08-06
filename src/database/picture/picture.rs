use crate::api::picture::ListPictureData;
use crate::api::query_pictures::{PictureFilter, PictureSort, PicturesQuery};
use crate::database::database::DBConn;
use crate::database::picture::picture_tag::PictureTag;
use crate::database::picture::rating::Rating;
use crate::database::schema::PictureOrientation;
use crate::database::schema::*;
use crate::database::tag::tag::Tag;
use crate::database::user::user::User;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use diesel::dsl::{exists, insert_into, not, Filter, Nullable};
use diesel::helper_types::{IntoBoxed, LeftJoin, LeftJoinOn, LeftJoinQuerySource, Or};
use diesel::internal::table_macro::{BoxedSelectStatement, FromClause, Join, JoinOn, LeftOuter, SelectStatement};
use diesel::query_builder::QueryFragment;
use diesel::query_dsl::InternalJoinDsl;
use diesel::sql_types::{BigInt, Binary, Bool, Decimal, Integer, SmallInt, Text, TinyInt, VarChar, Varchar};
use diesel::QueryDsl;
use diesel::{Associations, Identifiable, Queryable, RunQueryDsl, Selectable};
use diesel::{BoolExpressionMethods, ExpressionMethods};
use diesel::{JoinOnDsl, NullableExpressionMethods, OptionalExtension, SelectableHelper};
use diesel_derives::Insertable;
use rocket::serde::json::Json;
use rocket_okapi::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Identifiable, Associations, Insertable, JsonSchema, Serialize, Debug, PartialEq, Clone)]
#[diesel(primary_key(id))]
#[diesel(belongs_to(User, foreign_key = owner_id))]
#[diesel(table_name = pictures)]
pub struct Picture {
    pub id: i64,
    pub name: String,
    pub comment: String,
    pub owner_id: i32,
    pub author_id: i32,
    pub deleted_date: Option<NaiveDateTime>,
    pub copied: bool,
    pub creation_date: NaiveDateTime,
    pub edition_date: NaiveDateTime,
    /// 6 decimals, maximum 100.000000°
    pub latitude: Option<BigDecimal>,
    /// 6 decimals, maximum 1000.000000°
    pub longitude: Option<BigDecimal>,
    pub altitude: Option<i16>,
    pub orientation: PictureOrientation,
    pub width: i16,
    pub height: i16,
    pub camera_brand: Option<String>,
    pub camera_model: Option<String>,
    /// 2 decimals, maximum 10000.00mm (10 m)
    pub focal_length: Option<BigDecimal>,
    pub exposure_time_num: Option<i32>,
    pub exposure_time_den: Option<i32>,
    pub iso_speed: Option<i32>,
    /// 1 decimal, maximum 1000.0
    pub f_number: Option<BigDecimal>,
    pub size_ko: i32,
}
#[derive(Debug, PartialEq, JsonSchema, Serialize)]
pub struct PictureDetails {
    pub picture: Picture,
    pub tags_ids: Vec<i32>,
    pub ratings: Vec<Rating>,
}
/// The first Option is None if value is mixed
#[derive(Debug, PartialEq, JsonSchema, Serialize)]
pub struct MixedPicture {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_date: Option<Option<NaiveDateTime>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copied: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creation_date: Option<NaiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edition_date: Option<NaiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<Option<BigDecimal>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<Option<BigDecimal>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub altitude: Option<Option<i16>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orientation: Option<PictureOrientation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera_brand: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera_model: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focal_length: Option<Option<BigDecimal>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exposure_time_num: Option<Option<i32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exposure_time_den: Option<Option<i32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iso_speed: Option<Option<i32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub f_number: Option<Option<BigDecimal>>,
    pub total_size_ko: i32,
}
#[derive(Debug, PartialEq, JsonSchema, Serialize)]
pub struct MixedPictureDetails {
    pub pictures: MixedPicture,
    pub common_tags_ids: Vec<i32>,          // Tags that all pictures have
    pub mixed_tags_ids: Vec<i32>,           // Tags that some but not all pictures have
    pub average_user_rating: Option<i16>,   // Average ratings of the user, or None if no rating exists
    pub average_global_rating: Option<i16>, // Average ratings of the user and its friends, or None if no rating exists
    pub rating_users: Vec<i32>,             // List of friends user IDs that rated the picture
}

impl Picture {
    pub fn list_all(conn: &mut DBConn, user_id: i32, deleted: bool, shared: Option<bool>) -> Result<Vec<ListPictureData>, ErrorResponder> {
        let include_owned = !shared.unwrap_or(false);
        let include_shared = shared.unwrap_or(true);

        let mut pictures: Vec<ListPictureData> = Vec::new();

        if include_owned {
            pictures = pictures::table
                .filter(pictures::dsl::owner_id.eq(user_id))
                .filter(pictures::dsl::deleted_date.is_null().eq(!deleted))
                .select((
                    pictures::id,
                    pictures::name,
                    pictures::width,
                    pictures::height,
                    pictures::creation_date,
                    pictures::edition_date,
                ))
                .load::<(i64, String, i16, i16, NaiveDateTime, NaiveDateTime)>(conn)
                .map(|vec| {
                    vec.into_iter()
                        .map(|(id, name, width, height, creation_date, edition_date)| ListPictureData {
                            id,
                            name,
                            width,
                            height,
                            creation_date,
                            edition_date,
                        })
                        .collect()
                })
                .map_err(|e| ErrorType::DatabaseError("Failed to get pictures".to_string(), e).res())?;
        }
        if include_shared {
            pictures.append(
                &mut pictures::table
                    .inner_join(groups_pictures::table.on(groups_pictures::dsl::picture_id.eq(pictures::dsl::id)))
                    .inner_join(shared_groups::table.on(shared_groups::dsl::group_id.eq(groups_pictures::dsl::group_id)))
                    .filter(shared_groups::dsl::user_id.eq(user_id))
                    .filter(pictures::dsl::deleted_date.is_null().eq(!deleted))
                    .select((
                        pictures::id,
                        pictures::name,
                        pictures::width,
                        pictures::height,
                        pictures::creation_date,
                        pictures::edition_date,
                    ))
                    .load::<(i64, String, i16, i16, NaiveDateTime, NaiveDateTime)>(conn)
                    .map(|vec| {
                        vec.into_iter()
                            .map(|(id, name, width, height, creation_date, edition_date)| ListPictureData {
                                id,
                                name,
                                width,
                                height,
                                creation_date,
                                edition_date,
                            })
                            .collect()
                    })
                    .map_err(|e| ErrorType::DatabaseError("Failed to get pictures".to_string(), e).res())?,
            );
        }
        Ok(pictures)
    }

    /// Get a list of pictures based on the query. This function guaranties that the user has the right to access the requested pictures.
    pub fn query(conn: &mut DBConn, user_id: i32, query: PicturesQuery, page_size: i64) -> Result<Vec<ListPictureData>, ErrorResponder> {
        assert_ne!(query.page, 0, "Page number must be greater than 0");

        // Initial request that returns all the pictures the user can see
        let mut dsl_query = pictures::table
            .left_join(groups_pictures::table.on(groups_pictures::dsl::picture_id.eq(pictures::dsl::id)))
            .left_join(shared_groups::table.on(shared_groups::dsl::group_id.eq(groups_pictures::dsl::group_id)))
            .filter(
                pictures::dsl::owner_id
                    .eq(user_id) // Owned picture
                    .or(shared_groups::dsl::user_id.eq(user_id)), // Shared picture
            )
            .select(Picture::as_select())
            .distinct()
            .into_boxed();

        // Applying filters
        for filter in query.filters {
            dsl_query = match filter.clone() {
                PictureFilter::Owned { invert } => {
                    if !invert {
                        dsl_query.filter(pictures::dsl::owner_id.eq(user_id))
                    } else {
                        dsl_query.filter(not(pictures::dsl::owner_id.eq(user_id)))
                    }
                }
                PictureFilter::Deleted { invert } => dsl_query.filter(pictures::dsl::deleted_date.is_null().eq(invert)),
                PictureFilter::Arrangement { invert, ids } => {
                    let gp_alias = diesel::alias!(groups_pictures as gp_alias);
                    let subquery = exists(
                        gp_alias
                            .inner_join(groups::table.on(groups::id.eq(gp_alias.field(groups_pictures::group_id))))
                            .filter(gp_alias.field(groups_pictures::picture_id).eq(pictures::id))
                            .filter(groups::arrangement_id.eq_any(ids)),
                    );
                    if !invert {
                        dsl_query.filter(subquery)
                    } else {
                        dsl_query.filter(not(subquery))
                    }
                }
                PictureFilter::Group { invert, ids } => {
                    let gp_alias = diesel::alias!(groups_pictures as gp_alias);
                    let subquery = exists(
                        gp_alias
                            .filter(gp_alias.field(groups_pictures::picture_id).eq(pictures::id))
                            .filter(gp_alias.field(groups_pictures::group_id).eq_any(ids)),
                    );
                    if !invert {
                        dsl_query.filter(subquery)
                    } else {
                        dsl_query.filter(not(subquery))
                    }
                }
                PictureFilter::TagGroup { invert, ids } => {
                    let subquery = exists(
                        pictures_tags::table
                            .inner_join(tags::table.on(tags::id.eq(pictures_tags::tag_id)))
                            .filter(pictures_tags::picture_id.eq(pictures::id))
                            .filter(tags::tag_group_id.eq_any(ids)),
                    );
                    if !invert {
                        dsl_query.filter(subquery)
                    } else {
                        dsl_query.filter(not(subquery))
                    }
                }
                PictureFilter::Tag { invert, ids } => {
                    let subquery = exists(
                        pictures_tags::table
                            .filter(pictures_tags::picture_id.eq(pictures::id))
                            .filter(pictures_tags::tag_id.eq_any(ids)),
                    );
                    if !invert {
                        dsl_query.filter(subquery)
                    } else {
                        dsl_query.filter(not(subquery))
                    }
                }
            }
        }

        // Applying sorting
        for sort in query.sorts {
            dsl_query = match sort {
                PictureSort::CreationDate { ascend } => {
                    if ascend {
                        dsl_query.order(pictures::dsl::creation_date.asc())
                    } else {
                        dsl_query.order(pictures::dsl::creation_date.desc())
                    }
                }
                PictureSort::EditionDate { ascend } => {
                    if ascend {
                        dsl_query.order(pictures::dsl::edition_date.asc())
                    } else {
                        dsl_query.order(pictures::dsl::edition_date.desc())
                    }
                }
            }
        }

        // Applying pagination
        dsl_query = dsl_query.limit(page_size).offset((query.page - 1) as i64 * page_size);

        // Fetching the pictures
        let pictures: Vec<ListPictureData> = dsl_query
            .select((
                pictures::id,
                pictures::name,
                pictures::width,
                pictures::height,
                pictures::creation_date,
                pictures::edition_date,
            ))
            .distinct()
            .load::<(i64, String, i16, i16, NaiveDateTime, NaiveDateTime)>(conn)
            .map(|vec| {
                vec.into_iter()
                    .map(|(id, name, width, height, creation_date, edition_date)| ListPictureData {
                        id,
                        name,
                        width,
                        height,
                        creation_date,
                        edition_date,
                    })
                    .collect()
            })
            .map_err(|e| ErrorType::DatabaseError("Failed to get pictures".to_string(), e).res())?;

        Ok(pictures)
    }

    /// Returns Ok(true) if the user is the owner of the picture or the picture is in a group shared with the user
    pub fn can_user_access_picture(conn: &mut DBConn, picture_id: i64, user_id: i32) -> Result<bool, ErrorResponder> {
        let owned_count = pictures::table
            .filter(pictures::dsl::id.eq(picture_id))
            .filter(pictures::dsl::owner_id.eq(user_id))
            .count()
            .get_result::<i64>(conn)
            .map_err(|e| ErrorType::DatabaseError("Failed to get picture".to_string(), e).res())?;

        if owned_count > 0 {
            return Ok(true);
        }

        let shared_count = groups_pictures::table
            .inner_join(shared_groups::table.on(shared_groups::dsl::group_id.eq(groups_pictures::dsl::group_id)))
            .filter(shared_groups::dsl::user_id.eq(user_id))
            .filter(groups_pictures::dsl::picture_id.eq(picture_id))
            .count()
            .get_result::<i64>(conn)
            .map_err(|e| ErrorType::DatabaseError("Failed to get picture".to_string(), e).res())?;

        Ok(shared_count > 0)
    }
    pub fn filter_user_accessible_pictures(conn: &mut DBConn, user_id: i32, picture_ids: &Vec<i64>) -> Result<Vec<i64>, ErrorResponder> {
        pictures::table
            // Join with shared pictures
            .left_join(
                groups_pictures::table
                    .inner_join(shared_groups::table.on(shared_groups::dsl::group_id.eq(groups_pictures::dsl::group_id)))
                    .on(groups_pictures::dsl::picture_id.eq(pictures::dsl::id)),
            )
            // Filter allowed pictures
            .filter(shared_groups::dsl::user_id.eq(user_id).or(pictures::dsl::owner_id.eq(user_id)))
            // Filter requested pictures
            .filter(pictures::dsl::id.eq_any(picture_ids))
            .select(pictures::dsl::id)
            .distinct()
            .load(conn)
            .map_err(|e| ErrorType::DatabaseError("Failed to get accessible pictures".to_string(), e).res())
    }
    pub fn filter_user_unaccessible_pictures(conn: &mut DBConn, user_id: i32, picture_ids: &Vec<i64>) -> Result<Vec<i64>, ErrorResponder> {
        pictures::table
            // Join with shared pictures
            .left_join(
                groups_pictures::table
                    .inner_join(shared_groups::table.on(shared_groups::dsl::group_id.eq(groups_pictures::dsl::group_id)))
                    .on(groups_pictures::dsl::picture_id.eq(pictures::dsl::id)),
            )
            // Filter disallowed pictures
            .filter(not(shared_groups::dsl::user_id.eq(user_id).and(pictures::dsl::owner_id.eq(user_id))))
            // Filter requested pictures
            .filter(pictures::dsl::id.eq_any(picture_ids))
            .select(pictures::dsl::id)
            .distinct()
            .load(conn)
            .map_err(|e| ErrorType::DatabaseError("Failed to get accessible pictures".to_string(), e).res())
    }
    pub fn is_picture_publicly_shared(conn: &mut DBConn, picture_id: i64) -> Result<bool, ErrorResponder> {
        let shared_count = groups_pictures::table
            .inner_join(link_share_groups::table.on(link_share_groups::dsl::group_id.eq(groups_pictures::dsl::group_id)))
            .filter(groups_pictures::dsl::picture_id.eq(picture_id))
            .count()
            .get_result::<i64>(conn)
            .map_err(|e| ErrorType::DatabaseError("Failed to get picture".to_string(), e).res())?;

        Ok(shared_count > 0)
    }

    pub fn insert(
        conn: &mut DBConn,
        user_id: i32,
        name: String,
        metadata: Option<rexiv2::Metadata>,
        size_ko: i32,
    ) -> Result<Picture, ErrorResponder> {
        let mut p = Picture::from(metadata);
        p.owner_id = user_id;
        p.author_id = user_id;
        p.name = name;
        p.size_ko = size_ko;

        insert_into(pictures::table)
            .values((
                pictures::dsl::name.eq::<String>(p.name),
                pictures::dsl::comment.eq::<String>(p.comment),
                pictures::dsl::owner_id.eq(p.owner_id),
                pictures::dsl::author_id.eq(p.author_id),
                pictures::dsl::deleted_date.eq(p.deleted_date),
                pictures::dsl::copied.eq(p.copied),
                pictures::dsl::creation_date.eq(p.creation_date),
                pictures::dsl::edition_date.eq(p.edition_date),
                pictures::dsl::latitude.eq(p.latitude),
                pictures::dsl::longitude.eq(p.longitude),
                pictures::dsl::altitude.eq(p.altitude),
                pictures::dsl::orientation.eq(p.orientation),
                pictures::dsl::width.eq(p.width),
                pictures::dsl::height.eq(p.height),
                pictures::dsl::camera_brand.eq(p.camera_brand),
                pictures::dsl::camera_model.eq(p.camera_model),
                pictures::dsl::focal_length.eq(p.focal_length),
                pictures::dsl::exposure_time_num.eq(p.exposure_time_num),
                pictures::dsl::exposure_time_den.eq(p.exposure_time_den),
                pictures::dsl::iso_speed.eq(p.iso_speed),
                pictures::dsl::f_number.eq(p.f_number),
                pictures::dsl::size_ko.eq(p.size_ko),
            ))
            .get_result(conn)
            .map_err(|e| ErrorType::DatabaseError("Failed to insert user".to_string(), e).res())
    }

    pub fn get_pictures_details(conn: &mut DBConn, user_id: i32, picture_ids: Vec<i64>) -> Result<Vec<Picture>, ErrorResponder> {
        let pictures: Vec<Picture> = pictures::table
            // Join with shared pictures
            .left_join(
                groups_pictures::table
                    .inner_join(shared_groups::table.on(shared_groups::dsl::group_id.eq(groups_pictures::dsl::group_id)))
                    .on(groups_pictures::dsl::picture_id.eq(pictures::dsl::id)),
            )
            // Filter allowed pictures
            .filter(shared_groups::dsl::user_id.eq(user_id).or(pictures::dsl::owner_id.eq(user_id)))
            // Filter requested pictures
            .filter(pictures::dsl::id.eq_any(picture_ids))
            .select(Picture::as_select())
            .distinct()
            .load(conn)
            .map_err(|e| ErrorType::DatabaseError("Failed to get pictures details".to_string(), e).res())?;

        Ok(pictures)
    }

    pub fn get_picture_details(conn: &mut DBConn, user_id: i32, picture_id: i64) -> Result<PictureDetails, ErrorResponder> {
        let picture = Self::get_pictures_details(conn, user_id, vec![picture_id])?
            .pop()
            .ok_or_else(|| ErrorType::PictureNotFound.res())?;
        let ratings = Rating::from_picture_id_including_friends(conn, picture_id, user_id)?;
        let tags_ids = PictureTag::get_picture_tags(conn, picture_id, user_id)?;
        Ok(PictureDetails { picture, tags_ids, ratings })
    }

    /// Get mixed picture details from a vector of picture IDs
    /// This method efficiently queries the database and calculates mixed properties
    pub fn get_mixed_picture_details(conn: &mut DBConn, user_id: i32, picture_ids: &Vec<i64>) -> Result<MixedPictureDetails, ErrorResponder> {
        if picture_ids.is_empty() {
            return Err(ErrorType::UnprocessableEntity("Picture IDs list cannot be empty".to_string()).res());
        }
        // Get all pictures
        let pictures = Self::get_pictures_details(conn, user_id, picture_ids.clone())?;

        if pictures.is_empty() {
            return Err(ErrorType::PictureNotFound.res());
        }
        // Calculate the MixedPicture
        let mixed_picture = Self::calculate_mixed_picture(&pictures);

        // Tags processing
        let (common_tags_ids, mixed_tags_ids) = PictureTag::get_mixed_pictures_tags(conn, user_id, &picture_ids)?;
        // Rating processing
        let (average_user_rating, average_global_rating, rating_users) = Rating::get_mixed_pictures_ratings(conn, user_id, &picture_ids)?;

        Ok(MixedPictureDetails {
            pictures: mixed_picture,
            common_tags_ids,
            mixed_tags_ids,
            average_user_rating,
            average_global_rating,
            rating_users,
        })
    }

    /// Calculate mixed picture properties from a list of pictures
    fn calculate_mixed_picture(pictures: &[Picture]) -> MixedPicture {
        if pictures.is_empty() {
            return MixedPicture {
                name: None,
                comment: None,
                owner_id: None,
                author_id: None,
                deleted_date: None,
                copied: None,
                creation_date: None,
                edition_date: None,
                latitude: None,
                longitude: None,
                altitude: None,
                orientation: None,
                width: None,
                height: None,
                camera_brand: None,
                camera_model: None,
                focal_length: None,
                exposure_time_num: None,
                exposure_time_den: None,
                iso_speed: None,
                f_number: None,
                total_size_ko: 0,
            };
        }

        #[allow(unused)] // Used by the macro
        let first = &pictures[0];
        let total_size_ko = pictures.iter().map(|p| p.size_ko).sum();

        // Helper macro to check if all pictures have the same value for a field
        macro_rules! check_same {
            ($field:ident) => {
                if pictures.iter().all(|p| p.$field == first.$field) {
                    Some(first.$field.clone())
                } else {
                    None
                }
            };
        }

        MixedPicture {
            name: check_same!(name),
            comment: check_same!(comment),
            owner_id: check_same!(owner_id),
            author_id: check_same!(author_id),
            deleted_date: check_same!(deleted_date),
            copied: check_same!(copied),
            creation_date: check_same!(creation_date),
            edition_date: check_same!(edition_date),
            latitude: check_same!(latitude),
            longitude: check_same!(longitude),
            altitude: check_same!(altitude),
            orientation: check_same!(orientation),
            width: check_same!(width),
            height: check_same!(height),
            camera_brand: check_same!(camera_brand),
            camera_model: check_same!(camera_model),
            focal_length: check_same!(focal_length),
            exposure_time_num: check_same!(exposure_time_num),
            exposure_time_den: check_same!(exposure_time_den),
            iso_speed: check_same!(iso_speed),
            f_number: check_same!(f_number),
            total_size_ko,
        }
    }
}
