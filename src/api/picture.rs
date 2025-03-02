use crate::api::query_pictures::{PictureFilter, PictureSort, PicturesQuery};
use crate::database::database::{DBConn, DBPool};
use crate::database::picture::picture::Picture;
use crate::database::schema::pictures::width;
use crate::database::user::user::User;
use crate::utils::errors_catcher::{err_transaction, ErrorResponder, ErrorResponse, ErrorType};
use crate::utils::s3::PictureStorer;
use crate::utils::thumbnail::{generate_thumbnail, PictureThumbnail, ORIGINAL_TEMP_DIR, THUMBS_TEMP_DIR};
use aws_smithy_types::byte_stream::ByteStream;
use rand::random;
use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::response::Responder;
use rocket::serde::json::Json;
use rocket::serde::Serialize;
use rocket::{response, Request, Response, State};
use rocket_okapi::gen::OpenApiGenerator;
use rocket_okapi::okapi::openapi3::Responses;
use rocket_okapi::response::OpenApiResponderInner;
use rocket_okapi::{openapi, JsonSchema};
use schemars::gen::SchemaGenerator;
use schemars::schema::{Schema, SchemaObject};
use std::path::Path;
use strum::IntoEnumIterator;
use tokio::task;

#[derive(JsonSchema, Serialize, Debug)]
pub struct UploadPictureResponse {
    pub(crate) name: String,
    pub(crate) picture: Picture,
    pub(crate) thumbnail_error: Option<ErrorResponse>,
}

#[derive(FromForm, Debug)]
pub struct UploadPictureData<'r> {
    pub(crate) file: TempFile<'r>,
}

impl JsonSchema for UploadPictureData<'_> {
    fn schema_name() -> String {
        String::from("Upload")
    }
    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
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
    let file_name = upload.file.name().unwrap_or("unknown.jpg").to_string();

    let file_name_ascii = file_name.chars().filter(|c| c.is_ascii()).collect::<String>();
    let temp_file_name = format!("{}-{}", random::<u16>(), file_name_ascii);

    let res = {
        // Saving the file
        if let Err(e) = upload.file.persist_to(Path::new(ORIGINAL_TEMP_DIR).join(temp_file_name.clone())).await {
            println!("{:?}", e);
            return ErrorType::InternalError(format!("Unable to save file to {}", ORIGINAL_TEMP_DIR)).res_err();
        }
        let path = upload.file.path().unwrap();

        // Read EXIF metadata
        let meta = rexiv2::Metadata::new_from_path(path).ok();

        // Database operations
        let picture = err_transaction(conn, |conn| {
            let picture = Picture::insert(conn, user.id, file_name.clone(), meta)?;

            // TODO: request to add the picture to its matching groups

            // Upload file to S3
            task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    picture_storer
                        .store_picture_from_file(PictureThumbnail::Original, picture.id, &path)
                        .await
                })
            })?;

            Ok(picture)
        })?;

        // Generating thumbnails
        let mut thumbnail_error = None;
        for thumbnail_type in PictureThumbnail::iter() {
            if thumbnail_type == PictureThumbnail::Original {
                continue;
            }
            let thumbnail_path = generate_thumbnail(thumbnail_type, &path);

            let error = if let Ok(thumbnail_path) = thumbnail_path {
                picture_storer.store_picture_from_file(thumbnail_type, picture.id, &thumbnail_path).await
            } else {
                thumbnail_path.map(|_| ())
            };
            if let Err(e) = error {
                thumbnail_error = Some(ErrorResponse::from(e));
            }
        }

        Ok(Json(UploadPictureResponse {
            name: file_name,
            picture,
            thumbnail_error,
        }))
    };

    // Cleaning up files
    let _ = std::fs::remove_file(Path::new(ORIGINAL_TEMP_DIR).join(temp_file_name.clone()));
    let _ = std::fs::remove_file(Path::new(THUMBS_TEMP_DIR).join(temp_file_name));
    res
}

pub struct PictureStream {
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
/// If the user is logged in, the picture is only accessible if owned by the user or in a shared group with the user,
/// If the user is not logged in, the picture is only accessible if it is in a publicly shared group.
/// Otherwise, Unauthorized is returned
/// TODO: Implement S3 secret URL or picture secret token and remove the access check from this endpoint.
#[openapi(tag = "Picture")]
#[get("/picture/<picture_id>/<format>")]
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

#[derive(JsonSchema, Serialize, Debug)]
pub struct ListPictureData {
    pub(crate) id: u64,
    pub(crate) name: String,
    pub(crate) width: u16,
    pub(crate) height: u16,
}

/// Query pictures using custom query filters and sorting parameters.
/// Does not change any state, but using post to have a request body.
#[openapi(tag = "Picture")]
#[post("/pictures", data = "<query>")]
pub async fn query_pictures(db: &State<DBPool>, user: User, query: Json<PicturesQuery>) -> Result<Json<Vec<ListPictureData>>, ErrorResponder> {
    let conn: &mut DBConn = &mut db.get().unwrap();
    let pictures = Picture::query(conn, user.id, query.into_inner())?;
    Ok(Json(pictures))
}

/// List all pictures
#[openapi(tag = "Picture")]
#[get("/pictures?<deleted>")]
pub async fn list_pictures(db: &State<DBPool>, user: User, deleted: bool) -> Result<Json<Vec<ListPictureData>>, ErrorResponder> {
    let conn: &mut DBConn = &mut db.get().unwrap();

    let query = PicturesQuery {
        filters: vec![PictureFilter::Deleted { invert: !deleted }],
        sorts: vec![PictureSort::CreationDate { ascend: true }],
        page: 2,
    };

    let pictures = Picture::query(conn, user.id, query)?;
    Ok(Json(pictures))
}
