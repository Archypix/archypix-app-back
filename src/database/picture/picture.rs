use crate::database::database::DBConn;
use crate::database::schema::PictureOrientation;
use crate::database::schema::*;
use crate::database::user::user::User;
use crate::database::utils::get_last_inserted_id;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use diesel::dsl::insert_into;
use diesel::ExpressionMethods;
use diesel::JoinOnDsl;
use diesel::QueryDsl;
use diesel::{select, Associations, Identifiable, Queryable, RunQueryDsl, Selectable};
use diesel_derives::Insertable;
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
    /// Returns Unauthorized if the user is not the owner of the picture and the picture is not in a group shared with the user
    pub(crate) fn can_user_access_picture(conn: &mut DBConn, picture_id: u64, user_id: u32) -> Result<bool, ErrorResponder> {
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
            .filter(groups_pictures::dsl::picture_id.eq(picture_id))
            .filter(shared_groups::dsl::user_id.eq(user_id))
            .count()
            .get_result::<i64>(conn)
            .map_err(|e| ErrorType::DatabaseError("Failed to get picture".to_string(), e).res())?;

        Ok(shared_count > 0)
    }
    pub(crate) fn is_picture_publicly_shared(conn: &mut DBConn, picture_id: u64) -> Result<bool, ErrorResponder> {
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
