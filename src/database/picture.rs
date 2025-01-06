use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::NaiveDateTime;
use diesel::{Associations, Identifiable, Queryable, RunQueryDsl, Selectable};
use diesel::dsl::insert_into;
use rocket::http::ext::IntoCollection;
use crate::database::auth_token::Confirmation;
use crate::database::database::DBConn;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use crate::database::schema::PictureOrientation;
use crate::database::schema::*;
use crate::database::user::User;
use crate::database::utils::is_error_duplicate_key;

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq)]
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
    fn gps_val_to_big_decimal(gps_val: Option<f64>, angle_max: i32, decimals: i64) -> Option<BigDecimal> {
        if let Some(gps_val) = gps_val {
            let bd = BigDecimal::from_f64(gps_val);
            if let Some(bd) = bd {
                // Apply a modulo between -angle_max and angle_max
                return Some(((bd + BigDecimal::from(angle_max)) % BigDecimal::from(angle_max * 2) - BigDecimal::from(angle_max)).with_scale(decimals));
            }
        }
        None

    }
    pub fn insert(conn: &mut DBConn, user_id: u32, name: String, metadata: rexiv2::Metadata) -> Result<Picture, ErrorResponder> {
        let exposure_time = metadata.get_tag_rational("Exif.Photo.ExposureIndex");
        let creation_date = metadata.get_tag_string("Exif.Image.DateTimeOriginal").map(|s| s.as_str()).unwrap_or("");
        let edition_date = metadata.get_tag_string("Exif.Image.DateTime").map(|s| s.as_str()).unwrap_or("");

        let gps_info = metadata.get_gps_info();
        let latitude = Self::gps_val_to_big_decimal(gps_info.map(|g| g.latitude), 90, 6);
        let longitude = Self::gps_val_to_big_decimal(gps_info.map(|g| g.longitude), 180, 6);
        let altitude = gps_info.map(|g| g.altitude as u16);

        let latitude_ref = metadata.get_tag_string("Exif.GPSInfo.GPSLatitudeRef").unwrap_or("N".to_string()).eq("N");
        let longitude_ref = metadata.get_tag_string("Exif.GPSInfo.GPSLongitudeRef").unwrap_or("E".to_string()).eq("E");

        let latitude = metadata.get_tag_rational("Exif.GPSInfo.GPSLatitude").map(|r| {
            BigDecimal::from(if latitude_ref { 1 } else { -1 } * r)
        });
        let longitude = metadata.get_tag_rational("Exif.GPSInfo.GPSLongitude").map(|r| {
            BigDecimal::from(if longitude_ref { 1 } else { -1 } * r)
        });
        let altitude = metadata.get_tag_rational("Exif.GPSInfo.GPSAltitude").map(|r| {
            (r.numer() / r.denom()) as u16
        });

        let picture = Picture {
            id: 0,
            name,
            comment: metadata.get_tag_string("Exif.Image.ImageDescription").unwrap_or("".to_string()),
            owner_id: user_id,
            author_id: user_id,
            deleted_date: None,
            copied: false,
            creation_date: NaiveDateTime::parse_from_str(edition_date, "yyyy:MM:dd HH:mm:ss").unwrap_or(NaiveDateTime::default()),
            edition_date: NaiveDateTime::parse_from_str(edition_date, "yyyy:MM:dd HH:mm:ss").unwrap_or(NaiveDateTime::default()),
            latitude,
            longitude,
            altitude,
            orientation: PictureOrientation::Normal,
            width: metadata.get_pixel_width() as u16,
            height: metadata.get_pixel_height() as u16,
            camera_brand: metadata.get_tag_string("Exif.Photo.Make").ok(),
            camera_model: metadata.get_tag_string("Exif.Photo.Model").ok(),
            focal_length: metadata.get_tag_rational("Exif.Photo.FocalLengthIn35mmFilm").map(|f| BigDecimal::from(f)),
            exposure_time_num: exposure_time.map(|r| *r.numer() as u32),
            exposure_time_den: exposure_time.map(|r| *r.denom() as u32),
            iso_speed: Some(metadata.get_tag_numeric("Exif.Photo.ISOSpeed") as u32),
            f_number: metadata.get_tag_rational("Exif.Photo.FNumber").map(|f| BigDecimal::from(f)),
        };

        let inserted_count = insert_into(pictures::table)
            .values(&picture)
            .values((
                pictures::dsl::id.eq(None),
                ))
            .execute(conn)
            .or_else(|e| {
                ErrorType::DatabaseError("Failed to insert confirmation".to_string(), e).res_err_rollback()
            })?;

    }
}


#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq)]
#[diesel(primary_key(user_id, picture_id))]
#[diesel(belongs_to(User))]
#[diesel(belongs_to(Picture))]
#[diesel(table_name = ratings)]
pub struct Rating {
    pub user_id: u32,
    pub picture_id: u64,
    pub rating: i8,
}
