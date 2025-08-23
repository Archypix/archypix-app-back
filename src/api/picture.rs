use crate::api::query_pictures::{PictureFilter, PictureSort, PicturesQuery};
use crate::database::database::{DBConn, DBPool};
use crate::database::picture::picture::{MixedPictureDetails, Picture, PictureDetails};
use crate::database::picture::picture_tag::PictureTag;
use crate::database::user::user::User;
use crate::grouping::grouping_process::group_pictures;
use crate::utils::errors_catcher::{err_transaction, ErrorResponder, ErrorResponse, ErrorType};
use crate::utils::s3::PictureStorer;
use crate::utils::thumbnail::{generate_blurhash, generate_thumbnail, PictureThumbnail, ORIGINAL_TEMP_DIR, THUMBS_TEMP_DIR};
use aws_smithy_types::byte_stream::ByteStream;
use chrono::NaiveDateTime;
use diesel::dsl::update;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
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
use schemars::{
    gen::SchemaGenerator,
    schema::{InstanceType, Schema, SchemaObject, StringValidation},
};
use serde::Deserialize;
use serde_with::base64::Base64;
use serde_with::serde_as;
use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;
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
            error!("{:?}", e);
            return ErrorType::InternalError(format!("Unable to save file to {}", ORIGINAL_TEMP_DIR)).res_err();
        }
        let path = upload.file.path().unwrap();

        // Calculate file size (Rounding up)
        let file_size_o = path
            .metadata()
            .map_err(|e| ErrorType::InternalError(format!("Unable to get file metadata: {}", e.to_string())).res())?
            .size();
        let mut file_size_ko = ((file_size_o + 1023) / 1024) as i32;
        if file_size_ko > 10_000_000 {
            return ErrorType::InvalidInput(format!("File size is too big: {} Ko", file_size_ko)).res_err();
        }
        if file_size_ko == 0 {
            file_size_ko = 1;
        }
        if user.storage_count_ko + (file_size_ko as i64) > user.storage_limit_ko {
            return ErrorType::InvalidInput(format!("File size is too big: {} Ko", file_size_ko)).res_err();
        }

        // Read EXIF metadata
        let meta = rexiv2::Metadata::new_from_path(path).ok();

        // Generating thumbnails
        let mut thumbnail_error = None;
        let mut blurhash = None;
        let mut thumbnails = HashMap::new();
        for thumbnail_type in PictureThumbnail::iter() {
            if thumbnail_type == PictureThumbnail::Original {
                continue;
            }
            let thumbnail_path = generate_thumbnail(thumbnail_type, &path);

            match thumbnail_path {
                Ok(thumbnail_path) => {
                    thumbnails.insert(thumbnail_type as usize, thumbnail_path.clone());
                    // Generating tiny thumbnail
                    if thumbnail_type == PictureThumbnail::Small {
                        match generate_blurhash(&thumbnail_path) {
                            Ok(tiny_thumb) => {
                                blurhash = Some(tiny_thumb);
                            }
                            Err(e) => {
                                thumbnail_error = Some(ErrorResponse::from(e));
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    thumbnail_error = Some(ErrorResponse::from(e));
                    break;
                }
            }
        }

        // Database operations
        let picture = err_transaction(conn, |conn| {
            let picture = Picture::insert(conn, user.id, file_name.clone(), meta, file_size_ko, blurhash)?;
            let pictures = vec![picture.id];
            // Adding default tags
            PictureTag::add_default_tags(conn, user.id, &pictures)?;
            // Grouping pictures
            group_pictures(conn, user.id, Some(&pictures), None, None, false).map_err(|e| e.with_rollback(true))?;

            // Upload file to S3
            task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    picture_storer
                        .store_picture_from_file(PictureThumbnail::Original as usize, picture.id, &path)
                        .await
                })
            })?;

            Ok(picture)
        })?;

        // Uploading thumbnails to S3
        for (thumbnail_type, thumbnail_path) in thumbnails {
            let res = picture_storer.store_picture_from_file(thumbnail_type, picture.id, &thumbnail_path).await;
            if let Err(e) = res {
                thumbnail_error = Some(ErrorResponse::from(e));
                break;
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
    picture_id: i64,
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
    picture_id: i64,
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
        return Err(ErrorType::Unauthorized.res_no_rollback());
    }

    let picture_stream = picture_storer.get_picture(format, picture_id).await?;
    Ok(PictureStream { picture_id, picture_stream })
}

#[derive(JsonSchema, Serialize, Debug)]
pub struct ListPictureData {
    pub(crate) id: i64,
    pub(crate) name: String,
    pub(crate) width: i16,
    pub(crate) height: i16,
    pub(crate) creation_date: NaiveDateTime,
    pub(crate) edition_date: NaiveDateTime,
    pub(crate) blurhash: Option<String>,
}

#[derive(JsonSchema, Deserialize, Debug)]
pub struct PicturesDetailsQuery {
    picture_ids: Vec<i64>,
}
/// Get pictures details as a MixedPictureDetails object.
/// It includes the common fields containing the common data, and mixed data as None (serialized as nothing)
/// Common and mixed tags are also calculated, and statistics about ratings.
#[openapi(tag = "Picture")]
#[post("/pictures_details", data = "<data>")]
pub async fn get_pictures_details(
    db: &State<DBPool>,
    user: User,
    data: Json<PicturesDetailsQuery>,
) -> Result<Json<MixedPictureDetails>, ErrorResponder> {
    let conn: &mut DBConn = &mut db.get().unwrap();
    Ok(Json(Picture::get_mixed_picture_details(conn, user.id, &data.picture_ids)?))
}

/// Get picture details, includes tags and ratings
#[openapi(tag = "Picture")]
#[get("/picture_details/<picture_id>")]
pub async fn get_picture_details(db: &State<DBPool>, user: User, picture_id: i64) -> Result<Json<PictureDetails>, ErrorResponder> {
    let conn: &mut DBConn = &mut db.get().unwrap();

    let picture = Picture::get_picture_details(conn, user.id, picture_id)?;
    Ok(Json(picture))
}
