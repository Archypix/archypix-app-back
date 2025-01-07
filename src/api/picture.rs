use crate::database::database::{DBConn, DBPool};
use crate::database::picture::Picture;
use crate::database::user::User;
use crate::utils::errors_catcher::{err_transaction, ErrorResponder, ErrorType};
use crate::utils::s3::PictureStorer;
use aws_smithy_types::byte_stream::ByteStream;
use rocket::data::ToByteUnit;
use rocket::form::Form;
use rocket::fs::{NamedFile, TempFile};
use rocket::outcome::IntoOutcome;
use rocket::response::Responder;
use rocket::serde::json::Json;
use rocket::serde::Serialize;
use rocket::{response, Data, Request, Response, State};
use rocket_okapi::gen::OpenApiGenerator;
use rocket_okapi::okapi::openapi3;
use rocket_okapi::okapi::openapi3::Responses;
use rocket_okapi::response::OpenApiResponderInner;
use rocket_okapi::{openapi, JsonSchema};
use schemars::gen::SchemaGenerator;
use schemars::schema::{Schema, SchemaObject};
use serde::Deserialize;
use std::path::Path;
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

    // EXIF data
    let meta = rexiv2::Metadata::new_from_path(path).ok();


    let picture = err_transaction(conn, |conn| {
        let picture = Picture::insert(conn, user.id, file_name.clone(), meta)?;

        // TODO: request to add the picture to its matching groups

        Ok(picture)
    })?;

    // Saving the file
    picture_storer.store_picture_from_file(picture.id, &path).await?;

    Ok(Json(UploadPictureResponse {
        name: file_name,
        picture,
    }))
}

struct PictureStream {
    picture_id: u64,
    picture_stream: ByteStream,
}

impl<'a> Responder<'a, 'a> for PictureStream {
    fn respond_to(self, _: &Request) -> response::Result<'a> {
        Response::build()
            .header(rocket::http::ContentType::JPEG)
            .streamed_body(self.picture_stream.into_async_read())
            .ok()
    }
}
impl OpenApiResponderInner for PictureStream {
    fn responses(_: &mut OpenApiGenerator) -> rocket_okapi::Result<Responses> {
        Ok(Responses::default())
    }
}

/// Get a picture by its id
/// If the user is logged in, the picture is only accessible if  owned by the user or in a shared group with the user,
/// If the user is not logged in, the picture is only accessible if it is in a publicly shared group.
/// Otherwise, Unauthorized is returned
#[openapi(tag = "Picture")]
#[get("/picture/<picture_id>")]
pub async fn get_picture(
    db: &State<DBPool>,
    picture_id: u64,
    user: Option<User>,
    picture_storer: &State<PictureStorer>,
) -> Result<PictureStream, ErrorResponder> {
    let conn: &mut DBConn = &mut db.get().unwrap();

    let access_allowed = if let Some(user) = user {
        Picture::can_user_access_picture(conn, picture_id, user.id)?
    } else {
        Picture::is_picture_publicly_shared(conn, picture_id)?
    };
    if !access_allowed {
        return Err(ErrorType::Unauthorized.res());
    }

    let picture_stream = picture_storer.get_picture(picture_id).await?;
    Ok(PictureStream { picture_id, picture_stream })
}
