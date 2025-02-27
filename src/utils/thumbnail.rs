use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use magick_rust::{magick_wand_genesis, MagickWand};
use rocket::request::FromParam;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use strum_macros::{Display, EnumIter};

#[derive(Display, Debug, PartialEq, Clone, Copy, EnumIter, Deserialize, Serialize, JsonSchema)]
pub enum PictureThumbnail {
    Original = 0,
    Small = 1,
    Medium = 2,
    Large = 3,
}
impl FromParam<'_> for PictureThumbnail {
    type Error = ErrorResponder;
    fn from_param(param: &str) -> Result<Self, Self::Error> {
        match param {
            "original" => Ok(PictureThumbnail::Original),
            "small" => Ok(PictureThumbnail::Small),
            "medium" => Ok(PictureThumbnail::Medium),
            "large" => Ok(PictureThumbnail::Large),
            _ => ErrorType::NotFound(String::from("Invalid thumbnail type")).res_err(),
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
/// Generate a thumbnail from a source file and stores it in temp_dir/<thumbnail_type>/original_name.webp

pub fn generate_thumbnail(thumbnail_type: PictureThumbnail, source_file: &Path) -> Result<PathBuf, ErrorResponder> {
    // Initialize the Magick Wand environment
    magick_wand_genesis();

    let mut wand = MagickWand::new();
    if let Err(e) = wand.read_image(source_file.to_str().unwrap()) {
        println!("{:?}", e);
        return ErrorType::UnableToCreateThumbnail(String::from("Unable to read image")).res_err();
    }

    let size = get_thumbnail_size(thumbnail_type);
    wand.fit(size, size);

    if let Err(e) = wand.set_image_format("webp") {
        println!("{:?}", e);
        return ErrorType::UnableToCreateThumbnail(String::from("Unable to set image format")).res_err();
    }

    let dest_file = Path::new(THUMBS_TEMP_DIR).join(source_file.file_name().unwrap().to_str().unwrap());
    let dest_file_path = dest_file.to_str().unwrap();

    if let Err(e) = wand.write_image(dest_file_path) {
        println!("{:?}", e);
        return ErrorType::UnableToCreateThumbnail(String::from("Unable to write image")).res_err();
    }

    Ok(dest_file)
}

fn get_thumbnail_size(thumbnail_type: PictureThumbnail) -> usize {
    match thumbnail_type {
        PictureThumbnail::Original => None,
        PictureThumbnail::Small => Some(100),
        PictureThumbnail::Medium => Some(500),
        PictureThumbnail::Large => Some(1000),
    }
    .expect("Invalid thumbnail type")
}
