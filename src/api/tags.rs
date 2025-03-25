use crate::api::groups::arrangement::CreateArrangementResponse;
use crate::api::picture::ListPictureData;
use crate::api::query_pictures::PicturesQuery;
use crate::database::database::{DBConn, DBPool};
use crate::database::picture::picture::Picture;
use crate::database::tag::tag::Tag;
use crate::database::tag::tag_group::TagGroup;
use crate::database::user::user::User;
use crate::utils::errors_catcher::{err_transaction, ErrorResponder, ErrorType};
use diesel::GroupedBy;
use log::Level::Error;
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::State;
use rocket_cors::AllOrSome::All;
use rocket_okapi::{openapi, JsonSchema};
use std::collections::HashMap;

#[derive(Debug, Serialize, JsonSchema)]
struct AllTagsResponse {
    pub tag_groups: Vec<TagGroupWithTags>,
}
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct TagGroupWithTags {
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

    let tag_groups = map.into_iter().map(|(tag_group, tags)| TagGroupWithTags { tag_group, tags }).collect();
    Ok(Json(AllTagsResponse { tag_groups }))
}

/// Creates a new tag group with tags
#[openapi(tag = "Tags")]
#[post("/tag_group", data = "<data>")]
pub async fn new_tag_group(mut data: Json<TagGroupWithTags>, db: &State<DBPool>, user: User) -> Result<Json<TagGroupWithTags>, ErrorResponder> {
    let conn: &mut DBConn = &mut db.get().unwrap();

    // TODO: prevent creation if the tag group is required and there are no default tag.

    data.tag_group.user_id = user.id;
    let tag_group = TagGroup::insert(conn, data.tag_group.clone())?;
    let mut tags = Vec::new();

    for mut tag in data.into_inner().tags {
        tag.tag_group_id = tag_group.id.unwrap();
        tags.push(Tag::insert(conn, tag)?);

        // TODO: apply changes to all pictures if the tag group is required and a default tag exists
    }

    Ok(Json(TagGroupWithTags { tag_group, tags }))
}
/// Add a new tag to an existing tag group
#[openapi(tag = "Tags")]
#[post("/tag", data = "<data>")]
pub async fn new_tag(data: Json<Tag>, db: &State<DBPool>, user: User) -> Result<Json<Tag>, ErrorResponder> {
    let mut conn: &mut DBConn = &mut db.get().unwrap();

    // Check that the user is the owner of the wanted tag group for this new tag
    let tag_group = TagGroup::from_id(conn, data.tag_group_id)?;
    if tag_group.user_id != user.id {
        return ErrorType::Unauthorized.res_err();
    }

    err_transaction(&mut conn, |conn| {
        let new_tag = Tag::insert(conn, data.into_inner())?;
        // Nothing to do here, the tag is new, and the default characteristic of a tag only applies on new pictures
        Ok(Json(new_tag))
    })
}
/// Edit an existing tag group
#[openapi(tag = "Tags")]
#[patch("/tag_group", data = "<data>")]
pub async fn edit_tag_group(data: Json<TagGroup>, db: &State<DBPool>, user: User) -> Result<Json<TagGroup>, ErrorResponder> {
    let mut conn: &mut DBConn = &mut db.get().unwrap();

    if data.id.is_none() {
        return ErrorType::UnprocessableEntity.res_err();
    }

    // Check that the user is the owner of the tag group
    let old_tag_group = TagGroup::from_id(conn, data.id.unwrap())?;
    if old_tag_group.user_id != user.id {
        return ErrorType::Unauthorized.res_err();
    }

    err_transaction(&mut conn, |conn| {
        let new_tag_group = TagGroup::patch(conn, data.into_inner(), user.id)?;

        // TODO: apply changes to all pictures and strategies depending on the differences between the old and new tag group
        // TODO: prevent changes if the tag group becomes required and there are no default tag.

        Ok(Json(new_tag_group))
    })
}
/// Edit an existing tag
#[openapi(tag = "Tags")]
#[patch("/tag", data = "<data>")]
pub async fn edit_tag(data: Json<Tag>, db: &State<DBPool>, user: User) -> Result<Json<Tag>, ErrorResponder> {
    let mut conn: &mut DBConn = &mut db.get().unwrap();

    // Check that the user is the owner of the tag group of this tag
    let (old_tag, tag_group) = Tag::from_id_with_tag_group(conn, data.id)?;
    if tag_group.user_id != user.id {
        return ErrorType::Unauthorized.res_err();
    }

    err_transaction(&mut conn, |conn| {
        let new_tag = Tag::patch(conn, data.into_inner())?;

        // TODO: apply changes to all pictures and strategies depending on the differences between the old and new tag
        // TODO: prevent changes if the tag becomes not default and the tag group is required and there are no default tag.

        Ok(Json(new_tag))
    })
}
#[derive(Debug, Deserialize, JsonSchema)]
struct IDOnly {
    pub id: u32,
}

/// Delete an existing tag
#[openapi(tag = "Tags")]
#[delete("/tag", data = "<data>")]
pub async fn delete_tag(data: Json<IDOnly>, db: &State<DBPool>, user: User) -> Result<(), ErrorResponder> {
    let mut conn: &mut DBConn = &mut db.get().unwrap();

    // Check that the user is the owner of the tag group of this tag
    let (tag, tag_group) = Tag::from_id_with_tag_group(conn, data.id)?;
    if tag_group.user_id != user.id {
        return ErrorType::Unauthorized.res_err();
    }

    err_transaction(&mut conn, |conn| {
        let deleted = Tag::delete(conn, data.id)?;
        if deleted == 0 {
            return ErrorType::InternalError("Tag group has not been deleted".to_string()).res_err();
        }

        // TODO: apply deletion to all pictures and strategies
        // TODO: prevent changes if the tag was default and the tag group is required and there are no any other default tag.

        Ok(())
    })
}

/// Delete an existing tag group
#[openapi(tag = "Tags")]
#[delete("/tag_group", data = "<data>")]
pub async fn delete_tag_group(data: Json<IDOnly>, db: &State<DBPool>, user: User) -> Result<(), ErrorResponder> {
    let mut conn: &mut DBConn = &mut db.get().unwrap();

    // Check that the user is the owner of the tag group
    let tag_group = TagGroup::from_id(conn, data.id)?;
    if tag_group.user_id != user.id {
        return ErrorType::Unauthorized.res_err();
    }

    err_transaction(&mut conn, |conn| {
        let deleted = TagGroup::delete(conn, data.id)?;
        if deleted == 0 {
            return ErrorType::InternalError("Tag group has not been deleted".to_string()).res_err();
        }

        // TODO: apply deletion to all pictures and strategies

        Ok(())
    })
}
