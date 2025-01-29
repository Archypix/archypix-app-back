use crate::database::picture::picture::Picture;
use crate::database::schema::PictureOrientation;
use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use chrono::{Local, NaiveDateTime};
use num_rational::Ratio;
use rexiv2::Metadata;

impl From<Metadata> for Picture {
    /// Creates a Picture from a rexiv2 Metadata
    /// The picture id, name, owner_id and author_id are set to 0 or empty String.
    fn from(metadata: Metadata) -> Self {
        let creation_tags = ["Exif.Image.DateTime", "Exif.Image.DateTimeOriginal", "Exif.Image.DateTimeDigitized"];
        let edition_tags = ["Exif.Photo.DateTimeOriginal", "Exif.Photo.DateTimeDigitized"];

        let creation_date = extract_first_tag(&metadata, &creation_tags).unwrap_or(String::new());
        let edition_date = extract_first_tag(&metadata, &edition_tags).unwrap_or(String::new());

        let gps_info = metadata.get_gps_info();
        let latitude = gps_val_to_big_decimal(gps_info.map(|g| g.latitude), 90, 6);
        let longitude = gps_val_to_big_decimal(gps_info.map(|g| g.longitude), 180, 6);
        let altitude = gps_info.map(|g| g.altitude as u16);

        let exposure_time = metadata.get_tag_rational("Exif.Photo.ExposureTime");

        let orientation = match metadata.get_tag_numeric("Exif.Image.Orientation") {
            1 => PictureOrientation::Normal,
            2 => PictureOrientation::HorizontalFlip,
            3 => PictureOrientation::Rotate180,
            4 => PictureOrientation::VerticalFlip,
            5 => PictureOrientation::Rotate90HorizontalFlip,
            6 => PictureOrientation::Rotate90,
            7 => PictureOrientation::Rotate90VerticalFlip,
            8 => PictureOrientation::Rotate270,
            _ => PictureOrientation::Unspecified,
        };

        Picture {
            id: 0,
            name: "".to_string(),
            comment: metadata.get_tag_string("Exif.Image.ImageDescription").unwrap_or("".to_string()),
            owner_id: 0,
            author_id: 0,
            deleted_date: None,
            copied: false,
            creation_date: NaiveDateTime::parse_from_str(creation_date.as_str(), "%Y:%m:%d %H:%M:%S").unwrap_or(NaiveDateTime::default()),
            edition_date: NaiveDateTime::parse_from_str(edition_date.as_str(), "%Y:%m:%d %H:%M:%S").unwrap_or(Local::now().naive_utc()),
            latitude,
            longitude,
            altitude,
            orientation,
            width: metadata.get_pixel_width() as u16,
            height: metadata.get_pixel_height() as u16,
            camera_brand: metadata.get_tag_string("Exif.Image.Make").ok(),
            camera_model: metadata.get_tag_string("Exif.Image.Model").ok(),
            focal_length: rational_to_big_decimal(metadata.get_tag_rational("Exif.Photo.FocalLengthIn35mmFilm"), 2),
            exposure_time_num: exposure_time.map(|r| *r.numer() as u32),
            exposure_time_den: exposure_time.map(|r| *r.denom() as u32),
            iso_speed: extract_iso(&metadata),
            f_number: rational_to_big_decimal(metadata.get_tag_rational("Exif.Photo.FNumber"), 1),
        }
    }
}

impl From<Option<Metadata>> for Picture {
    /// Creates a Picture from a rexiv2 Metadata
    /// The picture id, name, owner_id and author_id are set to 0 or empty String.
    /// If the metadata is None, the picture is created with default values.
    fn from(metadata: Option<Metadata>) -> Self {
        if let Some(metadata) = metadata {
            return metadata.into();
        }
        Picture {
            id: 0,
            name: String::default(),
            comment: String::default(),
            owner_id: 0,
            author_id: 0,
            deleted_date: None,
            copied: false,
            creation_date: NaiveDateTime::default(),
            edition_date: Local::now().naive_utc(),
            latitude: None,
            longitude: None,
            altitude: None,
            orientation: PictureOrientation::Unspecified,
            width: 0,
            height: 0,
            camera_brand: None,
            camera_model: None,
            focal_length: None,
            exposure_time_num: None,
            exposure_time_den: None,
            iso_speed: None,
            f_number: None,
        }
    }
}

/// Converts a GPS value to a big decimal with a given number of decimals
/// and a modulo between -angle_max and angle_max
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
/// Converts a rational to a big decimal with a given number of decimals
fn rational_to_big_decimal(rational: Option<Ratio<i32>>, decimals: i64) -> Option<BigDecimal> {
    rational
        .map(|r| r.to_f64())
        .flatten()
        .map(|f| BigDecimal::from_f64(f))
        .flatten()
        .map(|bd| bd.with_scale_round(decimals, bigdecimal::RoundingMode::HalfUp))
}

fn extract_first_tag(metadata: &Metadata, tags: &[&str]) -> Option<String> {
    for tag in tags {
        if let Some(value) = metadata.get_tag_string(tag).ok() {
            println!("Found valid tag `{}` with value `{}`", tag, value);
            return Some(value);
        }
    }
    None
}

fn extract_iso(metadata: &Metadata) -> Option<u32> {
    let iso_tags = [
        "Exif.Photo.ISOSpeedRatings",
        "Exif.Photo.PhotographicSensitivity",
        "Xmp.exifEX.PhotographicSensitivity",
    ];

    for tag in &iso_tags {
        let value = metadata.get_tag_numeric(tag);
        if value != 0 {
            return Some(value as u32);
        }
    }
    None
}
