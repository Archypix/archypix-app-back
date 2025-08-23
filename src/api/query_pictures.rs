use crate::api::picture::ListPictureData;
use crate::database::database::{DBConn, DBPool};
use crate::database::picture::picture::Picture;
use crate::database::schema::*;
use crate::database::user::user::User;
use crate::rocket::futures::StreamExt;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use crate::utils::s3::PictureStorer;
use crate::utils::thumbnail::{generate_blurhash, PictureThumbnail, THUMBS_TEMP_DIR};
use diesel::dsl::{exists, not, Filter};
use diesel::query_dsl::methods;
use diesel::QueryDsl;
use diesel::{update, ExpressionMethods, RunQueryDsl};
use rand::random;
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::State;
use rocket_okapi::{openapi, JsonSchema};
use std::cmp::Ordering;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PicturesQuery {
    pub filters: Vec<PictureFilter>, // Applies an AND between filters
    pub sorts: Vec<PictureSort>,
    pub page: i32,
}
impl PicturesQuery {
    pub fn from_page(page: i32) -> Self {
        PicturesQuery {
            filters: vec![],
            sorts: vec![],
            page,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum PictureFilter {
    Arrangement { invert: bool, ids: Vec<i32> }, // user must be the owner
    Group { invert: bool, ids: Vec<i32> },       // can be a shared group
    Deleted { invert: bool },
    Owned { invert: bool },                   // Only pictures owned by the user
    TagGroup { invert: bool, ids: Vec<i32> }, // user must be the owner
    Tag { invert: bool, ids: Vec<i32> },      // user must be the owner
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum PictureSort {
    CreationDate { ascend: bool },
    EditionDate { ascend: bool },
}

/// Query pictures using custom query filters and sorting parameters.
/// Does not change any state, but using post to have a request body.
#[openapi(tag = "Picture")]
#[post("/query_pictures", data = "<query>")]
pub async fn query_pictures(db: &State<DBPool>, user: User, query: Json<PicturesQuery>) -> Result<Json<Vec<ListPictureData>>, ErrorResponder> {
    let conn: &mut DBConn = &mut db.get().unwrap();
    let pictures = Picture::query(conn, user.id, query.into_inner(), 100)?;

    Ok(Json(pictures))
}
