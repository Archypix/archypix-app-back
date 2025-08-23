use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use image::GenericImageView;
use magick_rust::{magick_wand_genesis, MagickWand};
use rocket::request::FromParam;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};

#[derive(Display, Debug, PartialEq, Clone, Copy, EnumIter, Deserialize, Serialize, JsonSchema)]
pub enum PictureThumbnail {
    Original = 0,
    Small = 1,
    Medium = 2,
    Large = 3,
}
impl PictureThumbnail {
    pub fn get_thumbnail_height(&self) -> Option<usize> {
        match self {
            PictureThumbnail::Original => None,
            PictureThumbnail::Small => Some(100),
            PictureThumbnail::Medium => Some(500),
            PictureThumbnail::Large => Some(1000),
        }
    }
}
impl FromParam<'_> for PictureThumbnail {
    type Error = ErrorResponder;
    fn from_param(param: &str) -> Result<Self, Self::Error> {
        // Check if param is an integer
        if let Ok(index) = param.parse::<usize>() {
            return match index {
                0 => Ok(PictureThumbnail::Original),
                1 => Ok(PictureThumbnail::Small),
                2 => Ok(PictureThumbnail::Medium),
                3 => Ok(PictureThumbnail::Large),
                _ => ErrorType::NotFound(String::from("Invalid thumbnail index")).res_err_no_rollback(),
            };
        }
        match param {
            "original" => Ok(PictureThumbnail::Original),
            "small" => Ok(PictureThumbnail::Small),
            "medium" => Ok(PictureThumbnail::Medium),
            "large" => Ok(PictureThumbnail::Large),
            _ => ErrorType::NotFound(String::from("Invalid thumbnail type")).res_err_no_rollback(),
        }
    }
}
pub const ORIGINAL_TEMP_DIR: &str = "./picture-temp/original";
pub const THUMBS_TEMP_DIR: &str = "./picture-temp/thumbs";

pub fn create_temp_directories() {
    if !Path::new(ORIGINAL_TEMP_DIR).exists() {
        std::fs::create_dir_all(ORIGINAL_TEMP_DIR).expect("Unable to create temp directory");
    }
    if !Path::new(THUMBS_TEMP_DIR).exists() {
        std::fs::create_dir_all(THUMBS_TEMP_DIR).expect("Unable to create temp directory");
    }
}

/// Generate a thumbnail from a source file and stores it in THUMBS_TEMP_DIR/source_file_name
pub fn generate_thumbnail(thumbnail_type: PictureThumbnail, source_file: &Path) -> Result<PathBuf, ErrorResponder> {
    // Initialize the Magick Wand environment
    magick_wand_genesis();

    let mut wand = MagickWand::new();
    if let Err(e) = wand.read_image(source_file.to_str().unwrap()) {
        warn!("{:?}", e);
        return ErrorType::UnableToCreateThumbnail(String::from("Unable to read image")).res_err_no_rollback();
    }

    let height = thumbnail_type.get_thumbnail_height();
    if height.is_none() {
        panic!("Thumbnail size can’t be None: \"Original\" thumbnail type should not be used to generate thumbnails");
    }
    let height = height.unwrap();
    let width = height * wand.get_image_width() / wand.get_image_height();
    wand.thumbnail_image(width, height)
        .map_err(|e| ErrorType::UnableToCreateThumbnail(format!("Unable to resize: {}", e.to_string())).res_no_rollback())?;

    if let Err(e) = wand.set_image_format("webp") {
        warn!("{:?}", e);
        return ErrorType::UnableToCreateThumbnail(String::from("Unable to set image format")).res_err_no_rollback();
    }

    let dest_file = Path::new(THUMBS_TEMP_DIR).join(source_file.file_name().unwrap().to_str().unwrap());
    let dest_file_path = dest_file.to_str().unwrap();

    if let Err(e) = wand.write_image(dest_file_path) {
        warn!("{:?}", e);
        return ErrorType::UnableToCreateThumbnail(String::from("Unable to write image")).res_err_no_rollback();
    }

    Ok(dest_file)
}

pub fn generate_blurhash(source_file: &Path) -> Result<String, ErrorResponder> {
    magick_wand_genesis();

    let mut wand = MagickWand::new();
    if let Err(e) = wand.read_image(source_file.to_str().unwrap()) {
        warn!("{:?}", e);
        return ErrorType::UnableToCreateBlurhash(format!("Unable to read image: {}", e.to_string())).res_err_no_rollback();
    }

    let size = if wand.get_image_width() > wand.get_image_height() {
        (4, 3)
    } else if wand.get_image_width() == wand.get_image_height() {
        (3, 3)
    } else {
        (3, 4)
    };

    let in_size = (wand.get_image_width(), wand.get_image_height());

    wand.thumbnail_image(in_size.0, in_size.1)
        .map_err(|e| ErrorType::UnableToCreateBlurhash(format!("Unable to resize: {}", e.to_string())).res_no_rollback())?;

    let raw_data = wand
        .export_image_pixels(0, 0, in_size.0, in_size.1, "RGBA")
        .ok_or(ErrorType::UnableToCreateBlurhash("Unable to export image pixels".to_string()).res_no_rollback())?;

    blurhash::encode(size.0 as u32, size.1 as u32, in_size.0 as u32, in_size.1 as u32, raw_data.as_slice())
        .map_err(|e| ErrorType::UnableToCreateBlurhash(format!("Can’t encode: {}", e.to_string())).res_no_rollback())
}
