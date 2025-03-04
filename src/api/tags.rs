use crate::api::picture::ListPictureData;
use crate::api::query_pictures::PicturesQuery;
use crate::database::database::{DBConn, DBPool};
use crate::database::picture::picture::Picture;
use crate::database::tag::tag::Tag;
use crate::database::tag::tag_group::TagGroup;
use crate::database::user::user::User;
use crate::utils::errors_catcher::ErrorResponder;
use diesel::GroupedBy;
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::State;
use rocket_cors::AllOrSome::All;
use rocket_okapi::{openapi, JsonSchema};
use std::collections::HashMap;

#[derive(Debug, Serialize, JsonSchema)]
struct AllTagsResponse {
    pub tag_groups: Vec<TagGroupWithTagsResponse>,
}
#[derive(Debug, Serialize, JsonSchema)]
struct TagGroupWithTagsResponse {
    pub tag_group: TagGroup,
    pub tags: Vec<Tag>,
}

/// Get all tags groups and all tags of the user
#[openapi(tag = "Tags")]
#[get("/tags")]
pub async fn get_tags(db: &State<DBPool>, user: User) -> Result<Json<AllTagsResponse>, ErrorResponder> {
    let conn: &mut DBConn = &mut db.get().unwrap();
    let tags = TagGroup::list_all_tags(conn, user.id)?;

    let mut map: HashMap<TagGroup, Vec<Tag>> = HashMap::new();

    for (a, b) in tags {
        map.entry(a).or_insert_with(Vec::new).push(b);
    }

    let tag_groups = map
        .into_iter()
        .map(|(tag_group, tags)| TagGroupWithTagsResponse { tag_group, tags })
        .collect();
    Ok(Json(AllTagsResponse { tag_groups }))
}

#[derive(Debug, Deserialize, JsonSchema)]
struct NewTagGroupRequest {
    pub name: String,
    pub multiple: bool,
    pub default_tag_id: Option<u32>,
    pub required: bool,
    pub tags: Vec<NewTagRequest>,
}
#[derive(Debug, Deserialize, JsonSchema)]
struct NewTagRequest {
    pub name: String,
    pub color: Vec<u8>,
    pub is_default: bool,
}
/// Creates a new tag group
#[openapi(tag = "Tags")]
#[post("/tag_group", data = "<data>")]
pub async fn new_tag_group(data: Json<NewTagGroupRequest>, db: &State<DBPool>, user: User) -> Result<Json<TagGroupWithTagsResponse>, ErrorResponder> {
    let conn: &mut DBConn = &mut db.get().unwrap();

    let tag_group = TagGroup::insert(
        conn,
        TagGroup {
            id: 0,
            user_id: user.id,
            name: data.name.clone(),
            multiple: data.multiple,
            default_tag_id: data.default_tag_id,
            required: data.required,
        },
    )?;
    let mut tags = Vec::new();

    for tag in &data.tags {
        Tag::insert(
            conn,
            Tag {
                id: 0,
                tag_group_id: tag_group.id,
                name: tag.name.clone(),
                color: tag.color.clone(),
                is_default: tag.is_default,
            },
        )?;
    }

    Ok(Json(TagGroupWithTagsResponse { tag_group, tags }))
}
