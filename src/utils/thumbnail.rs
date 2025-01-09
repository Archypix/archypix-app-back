use crate::utils::errors_catcher::{ErrorResponder, ErrorResponse, ErrorType};
use magick_rust::{magick_wand_genesis, MagickWand};
use rand::{random, Rng};
use std::path::{Path, PathBuf};
use strum_macros::EnumIter;

#[derive(Debug, PartialEq, Clone, Copy, EnumIter)]
pub enum PictureThumbnail {
    Original = 0,
    Small = 1,
    Medium = 2,
    Large = 3,
}

pub fn generate_thumbnail(thumbnail_type: PictureThumbnail, source_file: &Path, dest_dir: &Path) -> Result<PathBuf, ErrorResponder> {
    // Initialize the Magick Wand environment
    magick_wand_genesis();

    let mut wand = MagickWand::new();
    if let Err(e) = wand.read_image(source_file.to_str().expect("Source file path is not valid UTF-8")) {
        return ErrorType::UnableToCreateThumbnail(String::from("Unable to read image")).res_err();
    }

    let size = get_thumbnail_size(thumbnail_type);
    if let Err(e) = wand.fit(size, size) {
        return ErrorType::UnableToCreateThumbnail(String::from("Unable to resize image")).res_err();
    }

    if let Err(e) = wand.set_image_format("webp") {
        return ErrorType::UnableToCreateThumbnail(String::from("Unable to set image format")).res_err();
    }

    let random_name: u64 = random();
    let dest_file_name = dest_dir.join(format!("{}.webp", random_name));
    let dest_file = dest_dir.join(dest_file_name.to_str().expect("Destination file path is not valid UTF-8"));
    let dest_file_path = dest_file.to_str().expect("Destination file path is not valid UTF-8");

    if let Err(e) = wand.write_image(dest_file_path) {
        return ErrorType::UnableToCreateThumbnail(String::from("Unable to write image")).res_err();
    }

    Ok(dest_file)
}

fn get_thumbnail_size(thumbnail_type: PictureThumbnail) -> (usize) {
    match thumbnail_type {
        PictureThumbnail::Original => None,
        PictureThumbnail::Small => Some(100),
        PictureThumbnail::Medium => Some(500),
        PictureThumbnail::Large => Some(1000),
    }.expect("Invalid thumbnail type")
}
