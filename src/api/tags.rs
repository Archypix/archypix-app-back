use crate::api::groups::arrangement::CreateArrangementResponse;
use crate::api::picture::ListPictureData;
use crate::api::query_pictures::PicturesQuery;
use crate::database::database::{DBConn, DBPool};
use crate::database::picture::picture::Picture;
use crate::database::tag::tag::Tag;
use crate::database::tag::tag_group::{TagGroup, TagGroupWithTags};
use crate::database::user::user::User;
use crate::utils::errors_catcher::{err_transaction, ErrorResponder, ErrorType};
use diesel::dsl::{exists, not};
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
struct PatchTagGroupRequest {
    pub edited_tag_group: TagGroup,
    pub new_tags: Vec<Tag>,
    pub edited_tags: Vec<Tag>,
    pub deleted_tags_ids: Vec<u32>,
}

/// Get all tags groups and all tags of the user
#[openapi(tag = "Tags")]
#[get("/tags")]
pub async fn get_tags(db: &State<DBPool>, user: User) -> Result<Json<AllTagsResponse>, ErrorResponder> {
    let conn: &mut DBConn = &mut db.get().unwrap();
    let tag_groups = TagGroup::list_all_tags_as_tag_group_with_tags(conn, user.id)?;
    Ok(Json(AllTagsResponse { tag_groups }))
}

/// Creates a new tag group with tags
#[openapi(tag = "Tags")]
#[post("/tag_group", data = "<data>")]
pub async fn new_tag_group(mut data: Json<TagGroupWithTags>, db: &State<DBPool>, user: User) -> Result<Json<TagGroupWithTags>, ErrorResponder> {
    let conn: &mut DBConn = &mut db.get().unwrap();

    // Check requirements:
    //  - If the group is required, there must be at least one default tag.
    //  - If the group is not multiple, there can't be more than one default tag.
    let mut default_tag_for_required_group = None;
    if data.tag_group.required {
        let default_tags_count = data.tags.iter().find(|tag| tag.is_default);
        if let Some(default_tag) = default_tags_count {
            default_tag_for_required_group = Some((*default_tag).clone());
        } else {
            return ErrorType::UnprocessableEntity.res_err();
        }
    }
    if !data.tag_group.multiple {
        let default_tags_count = data.tags.iter().filter(|tag| tag.is_default).count();
        if default_tags_count > 1 {
            return ErrorType::UnprocessableEntity.res_err();
        }
    }
    // Insert the group and tags
    let mut to_insert_tag_group = data.tag_group.clone();
    to_insert_tag_group.user_id = user.id;
    let inserted_tag_group = TagGroup::insert(conn, to_insert_tag_group)?;
    let inserted_tag_group_id = inserted_tag_group.id.unwrap();
    let mut inserted_tags = Vec::new();

    for mut tag in data.into_inner().tags {
        tag.tag_group_id = inserted_tag_group_id;
        inserted_tags.push(Tag::insert(conn, tag)?);
    }

    // If the group is required, add the first default tag to all pictures that don't have any tag from this tag group
    if let Some(default_tag) = default_tag_for_required_group {
        TagGroup::add_default_tag_to_pictures_without_tag(conn, default_tag.id, inserted_tag_group_id, user.id)?;
    }

    Ok(Json(TagGroupWithTags {
        tag_group: inserted_tag_group,
        tags: inserted_tags,
    }))
}

/// Patch a tag group and its tags (create, edit, delete)
#[openapi(tag = "Tags")]
#[patch("/tag_group", data = "<data>")]
pub async fn patch_tag_group(data: Json<PatchTagGroupRequest>, db: &State<DBPool>, user: User) -> Result<Json<TagGroupWithTags>, ErrorResponder> {
    let mut conn: &mut DBConn = &mut db.get().unwrap();

    // Check that the user is the owner of the tag group
    let old_tag_group = TagGroup::from_id(conn, data.edited_tag_group.id.unwrap())?;
    if old_tag_group.user_id != user.id {
        return ErrorType::Unauthorized.res_err();
    }
    let old_tag_group_tags = Tag::list_tags(conn, old_tag_group.id.unwrap())?;

    // Check requirements for the updated tag group:
    //  - If the group is required, there must be at least one default tag.
    //  - If the group is not multiple, there can't be more than one default tag.
    if data.edited_tag_group.required {
        let default_tags_count =
            data.edited_tags.iter().filter(|tag| tag.is_default).count() + data.new_tags.iter().filter(|tag| tag.is_default).count();
        if default_tags_count == 0 {
            return ErrorType::UnprocessableEntity.res_err();
        }
    }
    if !data.edited_tag_group.multiple {
        let default_tags_count =
            data.edited_tags.iter().filter(|tag| tag.is_default).count() + data.new_tags.iter().filter(|tag| tag.is_default).count();
        if default_tags_count > 1 {
            return ErrorType::UnprocessableEntity.res_err();
        }
    }

    err_transaction(&mut conn, |conn| {
        // 1. Edit the tag group
        let updated_tag_group = TagGroup::patch(conn, data.edited_tag_group.clone(), user.id)?;

        // 2. Delete tags
        for tag_id in &data.deleted_tags_ids {
            let tag = old_tag_group_tags
                .iter()
                .find(|t| t.id == *tag_id)
                .ok_or_else(|| ErrorType::TagNotFound.res())?;
            Tag::delete(conn, *tag_id)?;
        }

        // 3. Edit existing tags
        let mut updated_tags = Vec::new();
        for tag in data.edited_tags.clone() {
            let old_tag = old_tag_group_tags
                .iter()
                .find(|t| t.id == tag.id)
                .ok_or_else(|| ErrorType::TagNotFound.res())?;
            updated_tags.push(Tag::patch(conn, tag)?);
        }

        // 4. Create new tags
        for mut tag in data.new_tags.clone() {
            tag.tag_group_id = updated_tag_group.id.unwrap();
            updated_tags.push(Tag::insert(conn, tag)?);
        }

        // 5. If the group is required, add the first default tag to all pictures that don't have any tag from this tag group
        if updated_tag_group.required {
            if let Some(default_tag) = updated_tags.iter().find(|tag| tag.is_default) {
                TagGroup::add_default_tag_to_pictures_without_tag(conn, default_tag.id, updated_tag_group.id.unwrap(), user.id)?;
            }
        }

        Ok(Json(TagGroupWithTags {
            tag_group: updated_tag_group,
            tags: updated_tags,
        }))
    })
}

#[derive(Debug, Deserialize, JsonSchema)]
struct IDOnly {
    pub id: u32,
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
