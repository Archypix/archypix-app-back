use crate::api::query_pictures::PicturesQuery;
use crate::database::database::{DBConn, DBPool};
use crate::database::group::arrangement::ArrangementDependencyType;
use crate::database::picture::picture::Picture;
use crate::database::picture::picture_tag::PictureTag;
use crate::database::tag::tag::Tag;
use crate::database::tag::tag_group::{TagGroup, TagGroupWithTags};
use crate::database::user::user::User;
use crate::grouping::grouping_process::group_pictures;
use crate::utils::errors_catcher::{err_transaction, ErrorResponder, ErrorType};
use itertools::Itertools;
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
    pub deleted_tags_ids: Vec<i32>,
}

/// Get all tags groups and all tags of the user
#[openapi(tag = "Tags")]
#[get("/tags")]
pub async fn list_tags(db: &State<DBPool>, user: User) -> Result<Json<AllTagsResponse>, ErrorResponder> {
    let conn: &mut DBConn = &mut db.get().unwrap();
    let tag_groups = TagGroup::list_all_tags_as_tag_group_with_tags(conn, user.id)?;
    Ok(Json(AllTagsResponse { tag_groups }))
}

/// Creates a new tag group with tags
#[openapi(tag = "Tags")]
#[post("/tag_group", data = "<data>")]
pub async fn create_tag_group(data: Json<TagGroupWithTags>, db: &State<DBPool>, user: User) -> Result<Json<TagGroupWithTags>, ErrorResponder> {
    let mut conn: &mut DBConn = &mut db.get().unwrap();

    err_transaction(&mut conn, |conn| {
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

        let default_tag_ids = inserted_tags.iter().filter(|tag| tag.is_default).map(|tag| tag.id).collect_vec();

        // Check requirements (on inserted_tags to have the correct ids):
        //  - If the group is required, there must be at least one default tag.
        //  - If the group is not multiple, there can't be more than one default tag.
        if inserted_tag_group.required && default_tag_ids.len() == 0 {
            return ErrorType::UnprocessableEntity("Required tag group must have at least one default tag".to_string()).res_err();
        }
        if !inserted_tag_group.multiple && default_tag_ids.len() > 1 {
            return ErrorType::UnprocessableEntity("Multiple tag group can't have more than one default tag".to_string()).res_err();
        }

        // Add all default tags to all pictures
        let mut query = PicturesQuery::from_page(1);
        let mut pictures = Picture::query(conn, user.id, query.clone(), 1000)?;
        while pictures.len() > 0 {
            let ids = pictures.into_iter().map(|picture| picture.id).collect_vec();
            PictureTag::add_pictures_batch(conn, &default_tag_ids, &ids)?;
            query.page += 1;
            if ids.len() < 1000 {
                break;
            }
            pictures = Picture::query(conn, user.id, query.clone(), 1000)?;
        }

        Ok(Json(TagGroupWithTags {
            tag_group: inserted_tag_group,
            tags: inserted_tags,
        }))
    })
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

    let unedited_tags: Vec<Tag> = old_tag_group_tags
        .iter()
        .filter(|tag| !data.edited_tags.iter().any(|edited_tag| edited_tag.id == tag.id) && !data.deleted_tags_ids.contains(&tag.id))
        .cloned()
        .collect();

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

        // 5. Check requirements for the updated tag group:
        //  - If the group is required, there must be at least one default tag.
        //  - If the group is not multiple, there can't be more than one default tag.
        let default_tag_ids = updated_or_new_tags
            .iter()
            .chain(unedited_tags.iter())
            .filter(|tag| tag.is_default)
            .map(|tag| tag.id)
            .collect_vec();
        if data.edited_tag_group.required && default_tag_ids.len() == 0 {
            return ErrorType::UnprocessableEntity("Required tag group must have at least one default tag".to_string()).res_err();
        }
        if !data.edited_tag_group.multiple && default_tag_ids.len() > 1 {
            return ErrorType::UnprocessableEntity("Multiple tag group can't have more than one default tag".to_string()).res_err();
        }

        // 6. If the group is required, add all the default tag to all pictures that don't have any tag from this tag group
        if updated_tag_group.required {
            TagGroup::add_tags_to_pictures_without_tag_from_user(conn, &default_tag_ids, updated_tag_group.id.unwrap(), user.id)?;
        }

        // 7. Gather all Tags: all old tags that are not deleted or edited, and all updated/new tags
        let mut all_tags = updated_or_new_tags.iter().chain(unedited_tags.iter()).cloned().collect::<Vec<Tag>>();

        // 7. Update arrangements strategies if needed
        // TODO: update arrangements that depends on this tag group.

        Ok(Json(TagGroupWithTags {
            tag_group: updated_tag_group,
            tags: all_tags,
        }))
    })
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IDOnly {
    pub id: i32,
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
pub struct EditPictureTagsRequest {
    pub picture_ids: Vec<i64>,
    pub add_tag_ids: Vec<i32>,
    pub remove_tag_ids: Vec<i32>,
}

/// Edit tags of a list of pictures
/// The user can edit tags of pictures he does not own as long as the tag is his own.
/// If the tag is not multiple, any picture already having a tag of the same tag group will lose the old tag in favor of the new one.
/// If the tag is required, the picture will be tagged with the default tag of the tag group.
#[openapi(tag = "Tags")]
#[patch("/picture_tags", data = "<data>")]
pub async fn edit_picture_tags(db: &State<DBPool>, user: User, data: Json<EditPictureTagsRequest>) -> Result<Json<Vec<i32>>, ErrorResponder> {
    let mut conn: &mut DBConn = &mut db.get().unwrap();
    if data.picture_ids.len() == 0 {
        return ErrorType::UnprocessableEntity("No picture ids on which to edit tags".to_string()).res_err();
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
        PictureTag::remove_pictures_batch(conn, &data.remove_tag_ids, &data.picture_ids)?;

        // Remove all tags for multiple tag groups before adding new tags
        for tgwt in add_tgwt {
            if !tgwt.tag_group.multiple {
                tgwt.tag_group.remove_pictures(conn, &data.picture_ids)?;
            }
        }
        // Add tags
        PictureTag::add_pictures_batch(conn, &data.add_tag_ids, &data.picture_ids)?;

        // Add default tags for required tag groups
        for tgwt in remove_tgwt {
            if tgwt.tag_group.required {
                // Get the default tag of the group
                let default_tag = Tag::list_tags(conn, tgwt.tag_group.id.unwrap())?
                    .into_iter()
                    .find(|tag| tag.is_default)
                    .ok_or_else(|| ErrorType::InternalError("There is a required tag group without any default tag".to_string()).res())?;
                // Add the default tag to the pictures
                TagGroup::add_default_tag_to_pictures_without_tag_from_list(conn, default_tag.id, tgwt.tag_group.id.unwrap(), &data.picture_ids)?;
            }
        }

        // Regroup the pictures
        group_pictures(
            conn,
            user.id,
            Some(&data.picture_ids),
            None,
            Some(&ArrangementDependencyType::new_tags_dependant()),
            true,
        )?;

        Ok(Json(PictureTag::get_picture_tags(conn, data.picture_ids[0], user.id)?))
    })
}
