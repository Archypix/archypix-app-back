use crate::api::picture::UploadPictureData;
use crate::picture_storer::picture_file_storer::PictureFileStorer;
use crate::picture_storer::picture_s3_storer::PictureS3Storer;
use crate::utils::errors_catcher::ErrorResponder;
use rocket::fs::NamedFile;

pub enum PictureStorerType {
    FILE,
    S3,
}

pub struct PictureStorer {
    picture_storer_type: PictureStorerType,
    picture_file_storer: Option<PictureFileStorer>,
    picture_s3_storer: Option<PictureS3Storer>,
}
impl PictureStorer {
    pub async fn store_picture(
        &self,
        picture_id: u64,
        upload: UploadPictureData<'_>,
    ) -> Result<(), ErrorResponder> {
        match self.picture_storer_type {
            PictureStorerType::FILE => self
                .picture_file_storer
                .as_ref()
                .unwrap()
                .store_picture(picture_id, upload).await,
            PictureStorerType::S3 => self
                .picture_s3_storer
                .as_ref()
                .unwrap()
                .store_picture(picture_id, upload).await,
        }
    }
    pub async fn get_picture(&self, picture_id: u64) -> Result<NamedFile, ErrorResponder> {
        match self.picture_storer_type {
            PictureStorerType::FILE => self
                .picture_file_storer
                .as_ref()
                .unwrap()
                .retrieve_picture(picture_id).await,
            PictureStorerType::S3 => self
                .picture_s3_storer
                .as_ref()
                .unwrap()
                .retrieve_picture(picture_id).await,
        }
    }
}

impl From<PictureFileStorer> for PictureStorer {
    fn from(picture_file_storer: PictureFileStorer) -> Self {
        PictureStorer {
            picture_storer_type: PictureStorerType::FILE,
            picture_file_storer: Some(picture_file_storer),
            picture_s3_storer: None,
        }
    }
}
impl From<PictureS3Storer> for PictureStorer {
    fn from(picture_s3_storer: PictureS3Storer) -> Self {
        PictureStorer {
            picture_storer_type: PictureStorerType::S3,
            picture_file_storer: None,
            picture_s3_storer: Some(picture_s3_storer),
        }
    }
}

pub trait PictureStorerTrait {
    async fn store_picture(&self, picture_id: u64, upload: UploadPictureData) -> Result<(), ErrorResponder>;
    fn delete_picture(&self, picture_id: u64) -> Result<(), ErrorResponder>;
    async fn retrieve_picture(&self, picture_id: u64) -> Result<NamedFile, ErrorResponder>;
}
