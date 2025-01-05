use rocket::{Data, State};
use rocket::data::ToByteUnit;
use rocket::form::Form;
use rocket::fs::{NamedFile, TempFile};
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use rocket::serde::json::Json;
use rocket::serde::Serialize;
use rocket_okapi::{openapi, JsonSchema};
use rocket_okapi::gen::OpenApiGenerator;
use rocket_okapi::okapi::openapi3;
use schemars::gen::SchemaGenerator;
use schemars::schema::{Schema, SchemaObject};
use serde::Deserialize;
use crate::database::user::User;
use crate::picture_storer::picture_file_storer::PictureFileStorer;
use crate::picture_storer::picture_storer::{PictureStorer};

#[derive(JsonSchema, Serialize, Debug)]
pub struct UploadPictureResponse {
    pub(crate) name: String,
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

/// Get the account information of the authenticated user.
/// If the credentials are invalid or match an unconfirmed or banned user, it returns an error from
/// the User Request Guard.
#[openapi(tag = "Picture")]
#[post("/picture", data = "<upload>")]
pub async fn add_picture(mut upload: Form<UploadPictureData<'_>>, user: User, picture_storer: &State<PictureStorer>) -> Result<Json<UploadPictureResponse>, ErrorResponder> {
    let file_name = upload.name.clone();
    // TODO: read the file exif data

    // TODO: save the file
    picture_storer.store_picture(0, upload.into_inner()).await.or(ErrorType::UnprocessableEntity.res_err())?;

    // TODO: add the picture to the database

    // TODO: add the picture to its matching groups.

    Ok(Json(UploadPictureResponse {
        name: "test".to_string(),
    }))
}


#[openapi(tag = "Picture")]
#[get("/picture/<picture_id>")]
pub async fn get_picture(picture_id: u64, user: Option<User>, picture_storer: &State<PictureStorer>) -> Result<NamedFile, ErrorResponder> {
    // TODO : check if the user has access to the picture

    picture_storer.get_picture(picture_id).await.or(ErrorType::UnprocessableEntity.res_err())
}