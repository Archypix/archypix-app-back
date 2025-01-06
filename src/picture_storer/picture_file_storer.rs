use crate::picture_storer::picture_storer::PictureStorerTrait;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use diesel::BoolExpressionMethods;
use rocket::fs::NamedFile;
use std::path::Path;

pub struct PictureFileStorer {
    save_path: String,
}

impl PictureFileStorer {
    pub fn new(save_path: String) -> Self {
        PictureFileStorer { save_path }
    }
}

impl PictureStorerTrait for PictureFileStorer {
    async fn store_picture(&self, picture_id: u64, temp_path: &Path) -> Result<(), ErrorResponder> {
        let path = Path::new(self.save_path.as_str()).join(picture_id.to_string());
        std::fs::copy(temp_path, path).or(ErrorType::UnableToSaveFile.res_err())?;
        Ok(())
    }

    fn delete_picture(&self, picture_id: u64) -> Result<(), ErrorResponder> {
        std::fs::remove_file(Path::new(self.save_path.as_str()).join(picture_id.to_string()))
            .or(ErrorType::InternalError("Failed to delete the file".to_string()).res_err())?;
        Ok(())
    }

    async fn retrieve_picture(&self, picture_id: u64) -> Result<NamedFile, ErrorResponder> {
        NamedFile::open(Path::new(self.save_path.as_str()).join(picture_id.to_string()))
            .await
            .or(ErrorType::InternalError("Failed to retrieve the file".to_string()).res_err())
    }
}
