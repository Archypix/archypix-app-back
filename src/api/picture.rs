use std::env::temp_dir;
use crate::database::database::{DBConn, DBPool};
use crate::database::picture::Picture;
use crate::database::user::User;
use crate::utils::errors_catcher::{err_transaction, ErrorResponder, ErrorType};
use crate::utils::s3::PictureStorer;
use crate::utils::thumbnail::{generate_thumbnail, PictureThumbnail, PictureThumbnailIter};
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
use rand::random;
use strum::IntoEnumIterator;
use tokio::io::AsyncReadExt;
use tokio::task;
use totp_rs::qrcodegen_image::image::imageops::thumbnail;

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

    let file_name_ascii = file_name.chars().filter(|c| c.is_ascii()).collect::<String>();
    let temp_dir = Path::new("./picture-temp");
    let temp_file_name = format!("{}-{}", random::<u16>(), file_name_ascii);

    let temp_file_name_clone = temp_file_name.clone();
    let res = {
        // Saving the file
        upload.file.persist_to(temp_dir.join("original").join(temp_file_name_clone)).await.unwrap();
        let path = upload.file.path().unwrap();

        // EXIF metadata
        let meta = rexiv2::Metadata::new_from_path(path).ok();

        // Database operations
        let picture = err_transaction(conn, |conn| {
            let picture = Picture::insert(conn, user.id, file_name.clone(), meta)?;

            // TODO: request to add the picture to its matching groups

            // Saving the file
            task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    picture_storer
                        .store_picture_from_file(PictureThumbnail::Original, picture.id, &path)
                        .await
                })
            });

            Ok(picture)
        })?;

        // Generating thumbnails
        for thumbnail_type in PictureThumbnail::iter() {
            if thumbnail_type == PictureThumbnail::Original {
                continue;
            }
            picture_storer
                .store_picture_from_file(thumbnail_type, picture.id, &generate_thumbnail(thumbnail_type, &path, &temp_dir).unwrap())
                .await?
        }

        Ok(Json(UploadPictureResponse { name: file_name, picture }))
    };

    if res.is_err() {
        for thumbnail_type in PictureThumbnail::iter() {
            let _ = std::fs::remove_file(temp_dir.join(thumbnail_type.to_string().to_lowercase()).join(temp_file_name.clone()));
        }
    }
    res
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
#[get("/picture/<format>/<picture_id>")]
pub async fn get_picture(
    db: &State<DBPool>,
    format: PictureThumbnail,
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

    let picture_stream = picture_storer.get_picture(format, picture_id).await?;
    Ok(PictureStream { picture_id, picture_stream })
}
