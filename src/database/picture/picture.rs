use crate::api::picture::ListPictureData;
use crate::api::query_pictures::{PictureFilter, PictureSort, PicturesQuery};
use crate::database::database::DBConn;
use crate::database::schema::PictureOrientation;
use crate::database::schema::*;
use crate::database::user::user::User;
use crate::database::utils::get_last_inserted_id;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use diesel::dsl::{insert_into, not, Filter, Nullable};
use diesel::helper_types::{IntoBoxed, LeftJoin, LeftJoinOn, LeftJoinQuerySource, Or};
use diesel::internal::table_macro::{BoxedSelectStatement, FromClause, Join, JoinOn, LeftOuter, SelectStatement};
use diesel::mysql::Mysql;
use diesel::query_builder::QueryFragment;
use diesel::sql_types::{BigInt, Binary, Bool, Datetime, Decimal, Integer, SmallInt, Text, TinyInt, Unsigned, VarChar, Varchar};
use diesel::JoinOnDsl;
use diesel::QueryDsl;
use diesel::{Associations, Identifiable, Queryable, RunQueryDsl, Selectable};
use diesel::{BoolExpressionMethods, ExpressionMethods};
use diesel_derives::Insertable;
use rocket::serde::json::Json;
use rocket_okapi::JsonSchema;
use serde::Serialize;

#[derive(Queryable, Selectable, Identifiable, Associations, Insertable, JsonSchema, Serialize, Debug, PartialEq, Clone)]
#[diesel(primary_key(id))]
#[diesel(belongs_to(User, foreign_key = owner_id))]
#[diesel(table_name = pictures)]
pub struct Picture {
    pub id: u64,
    pub name: String,
    pub comment: String,
    pub owner_id: u32,
    pub author_id: u32,
    pub deleted_date: Option<NaiveDateTime>,
    pub copied: bool,
    pub creation_date: NaiveDateTime,
    pub edition_date: NaiveDateTime,
    /// 6 decimals, maximum 100.000000°
    pub latitude: Option<BigDecimal>,
    /// 6 decimals, maximum 1000.000000°
    pub longitude: Option<BigDecimal>,
    pub altitude: Option<u16>,
    pub orientation: PictureOrientation,
    pub width: u16,
    pub height: u16,
    pub camera_brand: Option<String>,
    pub camera_model: Option<String>,
    /// 2 decimals, maximum 10000.00mm (10 m)
    pub focal_length: Option<BigDecimal>,
    pub exposure_time_num: Option<u32>,
    pub exposure_time_den: Option<u32>,
    pub iso_speed: Option<u32>,
    /// 1 decimal, maximum 1000.0
    pub f_number: Option<BigDecimal>,
}

impl Picture {
    pub fn list_all(conn: &mut DBConn, user_id: u32, deleted: bool, shared: Option<bool>) -> Result<Vec<ListPictureData>, ErrorResponder> {
        let include_owned = !shared.unwrap_or(false);
        let include_shared = shared.unwrap_or(true);

        let mut pictures: Vec<ListPictureData> = Vec::new();

        if include_owned {
            pictures = pictures::table
                .filter(pictures::dsl::owner_id.eq(user_id))
                .filter(pictures::dsl::deleted_date.is_null().eq(!deleted))
                .select((pictures::dsl::id, pictures::dsl::name, pictures::dsl::width, pictures::dsl::height))
                .load::<(u64, String, u16, u16)>(conn)
                .map(|vec| {
                    vec.into_iter()
                        .map(|(id, name, width, height)| ListPictureData { id, name, width, height })
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
                    .select((pictures::dsl::id, pictures::dsl::name, pictures::dsl::width, pictures::dsl::height))
                    .load::<(u64, String, u16, u16)>(conn)
                    .map(|vec| {
                        vec.into_iter()
                            .map(|(id, name, width, height)| ListPictureData { id, name, width, height })
                            .collect()
                    })
                    .map_err(|e| ErrorType::DatabaseError("Failed to get pictures".to_string(), e).res())?,
            );
        }
        Ok(pictures)
    }

    /// Get a list of pictures based on the query. This function guaranties that the user has the right to access the requested pictures.
    pub fn query(conn: &mut DBConn, user_id: u32, query: PicturesQuery) -> Result<Vec<ListPictureData>, ErrorResponder> {
        assert_ne!(query.page, 0, "Page number must be greater than 0");
        let page_size: u32 = 100;

        // Making an initial big join request with any table that might be needed.
        // The SQL optimizer will remove any unnecessary joins.
        let mut dsl_query = pictures::table
            // Join with groups_pictures & shared_groups (for shared pictures)
            .left_join(groups_pictures::table.on(groups_pictures::dsl::picture_id.eq(pictures::dsl::id)))
            .left_join(shared_groups::table.on(shared_groups::dsl::group_id.eq(groups_pictures::dsl::group_id)))
            // Join with groups_pictures (already done) and groups (for group and arrangement queries)
            .left_join(groups::table.on(groups::dsl::id.eq(groups_pictures::dsl::group_id)))
            // Join with picture_tags and tags (for tags and tag_groups queries)
            .left_join(pictures_tags::table.on(pictures_tags::dsl::picture_id.eq(pictures::dsl::id)))
            .left_join(tags::table.on(tags::dsl::id.eq(pictures_tags::dsl::tag_id)))
            // Filter only pictures that the user can see :
            .filter(
                pictures::dsl::owner_id
                    .eq(user_id) // Owned picture
                    .or(shared_groups::dsl::user_id.eq(user_id)), // Shared picture
            )
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
                    if !invert {
                        dsl_query.filter(groups::dsl::arrangement_id.eq_any(ids))
                    } else {
                        dsl_query.filter(not(groups::dsl::arrangement_id.eq_any(ids)))
                    }
                }
                PictureFilter::Group { invert, ids } => {
                    if !invert {
                        dsl_query.filter(groups_pictures::dsl::group_id.eq_any(ids))
                    } else {
                        dsl_query.filter(not(groups_pictures::dsl::group_id.eq_any(ids)))
                    }
                }
                PictureFilter::TagGroup { invert, ids } => {
                    if !invert {
                        dsl_query.filter(tags::dsl::tag_group_id.eq_any(ids))
                    } else {
                        dsl_query.filter(not(tags::dsl::tag_group_id.eq_any(ids)))
                    }
                }
                PictureFilter::Tag { invert, ids } => {
                    if !invert {
                        dsl_query.filter(pictures_tags::dsl::tag_id.eq_any(ids))
                    } else {
                        dsl_query.filter(not(pictures_tags::dsl::tag_id.eq_any(ids)))
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
        dsl_query = dsl_query.limit(page_size as i64).offset(((query.page - 1) * page_size) as i64);

        // Fetching the pictures
        let pictures: Vec<ListPictureData> = dsl_query
            .select((pictures::dsl::id, pictures::dsl::name, pictures::dsl::width, pictures::dsl::height))
            .distinct()
            .load::<(u64, String, u16, u16)>(conn)
            .map(|vec| {
                vec.into_iter()
                    .map(|(id, name, width, height)| ListPictureData { id, name, width, height })
                    .collect()
            })
            .map_err(|e| ErrorType::DatabaseError("Failed to get pictures".to_string(), e).res())?;

        Ok(pictures)
    }

    /// Returns Ok(true) if the user is the owner of the picture or the picture is in a group shared with the user
    pub fn can_user_access_picture(conn: &mut DBConn, picture_id: u64, user_id: u32) -> Result<bool, ErrorResponder> {
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
    pub fn is_picture_publicly_shared(conn: &mut DBConn, picture_id: u64) -> Result<bool, ErrorResponder> {
        let shared_count = groups_pictures::table
            .inner_join(link_share_groups::table.on(link_share_groups::dsl::group_id.eq(groups_pictures::dsl::group_id)))
            .filter(groups_pictures::dsl::picture_id.eq(picture_id))
            .count()
            .get_result::<i64>(conn)
            .map_err(|e| ErrorType::DatabaseError("Failed to get picture".to_string(), e).res())?;

        Ok(shared_count > 0)
    }

    pub fn insert(conn: &mut DBConn, user_id: u32, name: String, metadata: Option<rexiv2::Metadata>) -> Result<Picture, ErrorResponder> {
        let mut picture = Picture::from(metadata);
        picture.owner_id = user_id;
        picture.author_id = user_id;
        picture.name = name;

        let p = picture.clone();
        let _ = insert_into(pictures::table)
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
            ))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError("Failed to insert user".to_string(), e).res_rollback())?;

        picture.id = get_last_inserted_id(conn)?;

        Ok(picture)
    }
}
