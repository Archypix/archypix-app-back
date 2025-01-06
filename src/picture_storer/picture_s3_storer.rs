use crate::picture_storer::picture_storer::PictureStorerTrait;
use crate::utils::errors_catcher::ErrorResponder;
use rocket::fs::NamedFile;
use std::path::Path;

pub struct PictureS3Storer {
    bucket: String,
}

impl PictureStorerTrait for PictureS3Storer {
    async fn store_picture(&self, picture_id: u64, temp_path: &Path) -> Result<(), ErrorResponder> {
        todo!()
    }

    fn delete_picture(&self, picture_id: u64) -> Result<(), ErrorResponder> {
        todo!()
    }

    async fn retrieve_picture(&self, picture_id: u64) -> Result<NamedFile, ErrorResponder> {
        todo!()
    }
}
