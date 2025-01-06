use std::path::Path;
use crate::database::database::{DBConn, DBPool};
use crate::database::picture::Picture;
use crate::database::user::User;
use crate::picture_storer::picture_file_storer::PictureFileStorer;
use crate::picture_storer::picture_storer::PictureStorer;
use crate::utils::errors_catcher::{err_transaction, ErrorResponder, ErrorType};
use rocket::data::ToByteUnit;
use rocket::form::Form;
use rocket::fs::{NamedFile, TempFile};
use rocket::serde::json::Json;
use rocket::serde::Serialize;
use rocket::{Data, State};
use rocket::outcome::IntoOutcome;
use rocket_okapi::gen::OpenApiGenerator;
use rocket_okapi::okapi::openapi3;
use rocket_okapi::{openapi, JsonSchema};
use schemars::gen::SchemaGenerator;
use schemars::schema::{Schema, SchemaObject};
use serde::Deserialize;
use tokio::io::AsyncReadExt;

#[derive(JsonSchema, Serialize, Debug)]
pub struct UploadPictureResponse {
    pub(crate) name: String,
    pub(crate) picture: Picture,
}

#[derive(FromForm, Debug)]
pub struct UploadPictureData<'r> {
    pub(crate) name: String,
    pub(crate) file: TempFile<'r>,
}

impl JsonSchema for UploadPictureData<'_> {
    fn schema_name() -> String {
        String::from("Upload")
    }
    fn json_schema(gen: &mut SchemaGenerator) -> Schema {
        let schema = SchemaObject::default();

        Schema::Object(schema)
    }
}

/// Upload a picture using multipart form upload
/// TODO : Implement S3 direct upload
/// TODO : Implement chunked upload
#[openapi(tag = "Picture")]
#[post("/picture", data = "<upload>")]
pub async fn add_picture(
    mut upload: Form<UploadPictureData<'_>>,
    db: &State<DBPool>,
    picture_storer: &State<PictureStorer>,
    user: User,
) -> Result<Json<UploadPictureResponse>, ErrorResponder> {
    let conn: &mut DBConn = &mut db.get().unwrap();
    let file_name = upload.name.clone();
    let path = upload.file.path().ok_or(ErrorType::UnableToSaveFile.res())?;

    println!("Uploaded picture: {} as temp file to {:?}", file_name, path);

    // EXIF data
    let meta = rexiv2::Metadata::new_from_path(path).map_err(|e| ErrorType::UnableToLoadExifMetadata(e).res())?;

    let picture = err_transaction(conn, |conn| {
        let picture = Picture::insert(conn, user.id, file_name, meta)?;

        // TODO: request to add the picture to its matching groups

        Ok(picture)
    })?;

    // Saving the file
    picture_storer.store_picture(picture.id, path).await?;

    Ok(Json(UploadPictureResponse { name: String::from("tets"), picture }))
}

#[openapi(tag = "Picture")]
#[get("/picture/<picture_id>")]
pub async fn get_picture(picture_id: u64, user: Option<User>, picture_storer: &State<PictureStorer>) -> Result<NamedFile, ErrorResponder> {
    // TODO : check if the user has access to the picture

    picture_storer.get_picture(picture_id).await.or(ErrorType::UnprocessableEntity.res_err())
}
