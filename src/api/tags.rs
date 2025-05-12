use crate::database::database::{DBConn, DBPool};
use crate::database::picture::picture_tag::PictureTag;
use crate::database::tag::tag::Tag;
use crate::database::tag::tag_group::{TagGroup, TagGroupWithTags};
use crate::database::user::user::User;
use crate::utils::errors_catcher::{err_transaction, ErrorResponder, ErrorType};
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::State;
use rocket_okapi::{openapi, JsonSchema};

#[derive(Debug, Serialize, JsonSchema)]
pub struct AllTagsResponse {
    pub tag_groups: Vec<TagGroupWithTags>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct PatchTagGroupRequest {
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
pub async fn new_tag_group(data: Json<TagGroupWithTags>, db: &State<DBPool>, user: User) -> Result<Json<TagGroupWithTags>, ErrorResponder> {
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
        TagGroup::add_default_tag_to_pictures_without_tag_from_user(conn, default_tag.id, inserted_tag_group_id, user.id)?;
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

        // 2. Delete tags to delete
        for tag_id in &data.deleted_tags_ids {
            if !old_tag_group_tags.iter().any(|t| t.id == *tag_id) {
                return ErrorType::TagNotFound.res_err();
            }
            Tag::delete(conn, *tag_id)?;
        }

        // 3. Edit existing tags
        let mut updated_or_new_tags = Vec::new();
        for tag in data.edited_tags.clone() {
            if !old_tag_group_tags.iter().any(|t| t.id == tag.id) {
                return ErrorType::TagNotFound.res_err();
            }
            updated_or_new_tags.push(Tag::patch(conn, tag)?);
        }

        // 4. Create new tags
        for mut tag in data.new_tags.clone() {
            tag.tag_group_id = updated_tag_group.id.unwrap();
            updated_or_new_tags.push(Tag::insert(conn, tag)?);
        }

        // 5. If the group is required, add the first default tag to all pictures that don't have any tag from this tag group
        if updated_tag_group.required {
            if let Some(default_tag) = updated_or_new_tags.iter().find(|tag| tag.is_default) {
                TagGroup::add_default_tag_to_pictures_without_tag_from_user(conn, default_tag.id, updated_tag_group.id.unwrap(), user.id)?;
            }
        }

        // 6. Gather all Tags: all old tags that are not deleted or edited, and all updated/new tags
        let mut new_tag_group_tags = old_tag_group_tags
            .into_iter()
            .filter(|tag| !data.deleted_tags_ids.contains(&tag.id) && !data.edited_tags.iter().any(|edited_tag| edited_tag.id == tag.id))
            .collect::<Vec<Tag>>();
        new_tag_group_tags.append(&mut updated_or_new_tags);

        // 7. Update arrangements strategies if needed
        // TODO: update arrangements that depends on this tag group.

        Ok(Json(TagGroupWithTags {
            tag_group: updated_tag_group,
            tags: new_tag_group_tags,
        }))
    })
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IDOnly {
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

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TagIdWithPictureIds {
    pub tag_id: u32,
    pub picture_ids: Vec<u64>,
}

/// Add a tag to a list of pictures
/// The user can add tags to pictures he does not own as long as the tag is his own.
/// If the tag is not multiple, any picture already having a tag of the same tag group will lose the old tag in favor of the new one.
#[openapi(tag = "Tags")]
#[post("/add_tag_to_picture", data = "<data>")]
pub async fn add_tag_to_pictures(db: &State<DBPool>, user: User, data: Json<TagIdWithPictureIds>) -> Result<(), ErrorResponder> {
    let mut conn: &mut DBConn = &mut db.get().unwrap();

    // Check that the user is the owner of the tag group
    let tag = Tag::from_id(conn, data.tag_id)?;
    let tag_group = TagGroup::from_id(conn, tag.tag_group_id)?;
    if tag_group.user_id != user.id {
        return ErrorType::Unauthorized.res_err();
    }

    err_transaction(&mut conn, |conn| {
        if !tag_group.multiple {
            tag_group.remove_pictures(conn, data.picture_ids.clone())?;
        }
        Tag::add_pictures(conn, tag.id, data.picture_ids.clone())?;

        // TODO: check these pictures against arrangement that depends on tag groups.

        Ok(())
    })
}

/// Remove a tag from a list of pictures
/// The user can remove tags from pictures he does not own as long as the tag is his own.
/// If the tag is required, the picture will be tagged with the default tag of the tag group.
#[openapi(tag = "Tags")]
#[delete("/remove_tag_from_picture", data = "<data>")]
pub async fn remove_tag_from_pictures(db: &State<DBPool>, user: User, data: Json<TagIdWithPictureIds>) -> Result<(), ErrorResponder> {
    let mut conn: &mut DBConn = &mut db.get().unwrap();

    // Check that the user is the owner of the tag group
    let tag = Tag::from_id(conn, data.tag_id)?;
    let tag_group = TagGroup::from_id(conn, tag.tag_group_id)?;
    if tag_group.user_id != user.id {
        return ErrorType::Unauthorized.res_err();
    }

    err_transaction(&mut conn, |conn| {
        Tag::remove_pictures(conn, tag.id, data.picture_ids.clone())?;
        if tag_group.required {
            // Get the default tag of the group
            let default_tag = Tag::list_tags(conn, tag_group.id.unwrap())?
                .into_iter()
                .find(|tag| tag.is_default)
                .ok_or_else(|| ErrorType::InternalError("Required tag group without any default tag".to_string()).res())?;
            // Add the default tag to the pictures
            TagGroup::add_default_tag_to_pictures_without_tag_from_list(conn, default_tag.id, tag_group.id.unwrap(), data.picture_ids.clone())?;
        }
        Ok(())
    })
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EditPictureTagsRequest {
    pub picture_ids: Vec<u64>,
    pub add_tag_ids: Vec<u32>,
    pub remove_tag_ids: Vec<u32>,
}

/// Edit tags of a list of pictures
/// The user can edit tags of pictures he does not own as long as the tag is his own.
/// If the tag is not multiple, any picture already having a tag of the same tag group will lose the old tag in favor of the new one.
/// If the tag is required, the picture will be tagged with the default tag of the tag group.
#[openapi(tag = "Tags")]
#[patch("/picture_tags", data = "<data>")]
pub async fn edit_picture_tags(db: &State<DBPool>, user: User, data: Json<EditPictureTagsRequest>) -> Result<Json<Vec<u32>>, ErrorResponder> {
    let mut conn: &mut DBConn = &mut db.get().unwrap();
    if data.picture_ids.len() == 0 {
        return ErrorType::UnprocessableEntity.res_err();
    }

    // Grouping tags by tag group, checking at the same time that tags exists and belong to the user
    let add_tags = Tag::from_ids(conn, data.add_tag_ids.clone())?;
    let remove_tags = Tag::from_ids(conn, data.remove_tag_ids.clone())?;
    if add_tags.len() != data.add_tag_ids.len() || remove_tags.len() != data.remove_tag_ids.len() {
        return ErrorType::TagNotFound.res_err();
    }
    let user_tag_groups = TagGroup::list_tag_groups(conn, user.id)?;

    let mut more_than_one_add_tag = false;

    let add_tgwt: Vec<TagGroupWithTags> = user_tag_groups
        .iter()
        .filter_map(|tag_group| {
            let tags = add_tags
                .iter()
                .filter(|tag| tag.tag_group_id == tag_group.id.unwrap())
                .cloned()
                .collect::<Vec<Tag>>();
            if tags.is_empty() {
                return None;
            }
            if !tag_group.multiple && tags.len() > 1 {
                more_than_one_add_tag = true;
            }
            Some(TagGroupWithTags {
                tag_group: tag_group.clone(),
                tags,
            })
        })
        .collect();

    if more_than_one_add_tag {
        return ErrorType::InvalidInput("Cannot add multiple tags to a non-multiple tag group".to_string()).res_err();
    }

    let remove_tgwt: Vec<TagGroupWithTags> = user_tag_groups
        .iter()
        .filter_map(|tag_group| {
            let tags = remove_tags
                .iter()
                .filter(|tag| tag.tag_group_id == tag_group.id.unwrap())
                .cloned()
                .collect::<Vec<Tag>>();
            if tags.is_empty() {
                return None;
            }
            Some(TagGroupWithTags {
                tag_group: tag_group.clone(),
                tags,
            })
        })
        .collect();
    if add_tgwt.iter().map(|tgwt| tgwt.tags.len()).sum::<usize>() != add_tags.len()
        || remove_tgwt.iter().map(|tgwt| tgwt.tags.len()).sum::<usize>() != remove_tags.len()
    {
        return ErrorType::TagNotFound.res_err();
    }

    err_transaction(&mut conn, |conn| {
        // Remove tags
        Tag::remove_pictures_batch(conn, data.remove_tag_ids.clone(), data.picture_ids.clone())?;

        // Remove all tags for multiple tag groups before adding new tags
        for tgwt in add_tgwt {
            if !tgwt.tag_group.multiple {
                tgwt.tag_group.remove_pictures(conn, data.picture_ids.clone())?;
            }
        }
        // Add tags
        Tag::add_pictures_batch(conn, data.add_tag_ids.clone(), data.picture_ids.clone())?;

        // Add default tags for required tag groups
        for tgwt in remove_tgwt {
            if tgwt.tag_group.required {
                // Get the default tag of the group
                let default_tag = Tag::list_tags(conn, tgwt.tag_group.id.unwrap())?
                    .into_iter()
                    .find(|tag| tag.is_default)
                    .ok_or_else(|| ErrorType::InternalError("Required tag group without any default tag".to_string()).res())?;
                // Add the default tag to the pictures
                TagGroup::add_default_tag_to_pictures_without_tag_from_list(
                    conn,
                    default_tag.id,
                    tgwt.tag_group.id.unwrap(),
                    data.picture_ids.clone(),
                )?;
            }
        }

        // TODO: check these pictures against arrangement that depends on tag groups.

        Ok(Json(PictureTag::get_picture_tags(conn, data.picture_ids[0], user.id)?))
    })
}
