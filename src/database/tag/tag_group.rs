use crate::database::database::DBConn;
use crate::database::picture::picture_tag::PictureTag;
use crate::database::schema::*;
use crate::database::tag::tag::Tag;
use crate::database::user::user::User;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use diesel::dsl::{exists, not};
use diesel::{Associations, Identifiable, Queryable, RunQueryDsl, Selectable};
use diesel::{BoolExpressionMethods, JoinOnDsl};
use diesel::{EqAll, QueryDsl};
use diesel::{ExpressionMethods, OptionalExtension};
use itertools::Itertools;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Queryable, Selectable, Identifiable, Associations, Serialize, Deserialize, JsonSchema, Debug, PartialEq, Eq, Hash, Clone)]
#[diesel(primary_key(id))]
#[diesel(belongs_to(User, foreign_key = user_id))]
#[diesel(table_name = tag_groups)]
pub struct TagGroup {
    #[diesel(deserialize_as = i32, serialize_as = i32)]
    pub id: Option<i32>,
    pub user_id: i32,
    pub name: String,
    pub multiple: bool,
    pub required: bool,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct TagGroupWithTags {
    pub tag_group: TagGroup,
    pub tags: Vec<Tag>,
}

impl TagGroup {
    pub fn insert(conn: &mut DBConn, mut tag_group: TagGroup) -> Result<TagGroup, ErrorResponder> {
        diesel::insert_into(tag_groups::table)
            .values((
                tag_groups::user_id.eq(tag_group.user_id),
                tag_groups::name.eq(&tag_group.name.clone()),
                tag_groups::multiple.eq(tag_group.multiple),
                tag_groups::required.eq(tag_group.required),
            ))
            .get_result(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
    // Edit a tag group name, multiple, and required, works only if the user owns the tag group
    pub fn patch(conn: &mut DBConn, tag_group: TagGroup, user_id: i32) -> Result<TagGroup, ErrorResponder> {
        let _ = diesel::update(tag_groups::table.find(tag_group.id.unwrap()).filter(tag_groups::user_id.eq(user_id)))
            .set((
                tag_groups::name.eq(&tag_group.name),
                tag_groups::multiple.eq(tag_group.multiple),
                tag_groups::required.eq(tag_group.required),
            ))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())?;
        Ok(tag_group)
    }

    /// List all user's tag groups
    pub fn list_tag_groups(conn: &mut DBConn, user_id: i32) -> Result<Vec<TagGroup>, ErrorResponder> {
        tag_groups::table
            .filter(tag_groups::user_id.eq(user_id))
            .load(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
    /// List all user's tag groups and associated tags
    pub fn list_all_tags(conn: &mut DBConn, user_id: i32) -> Result<Vec<(TagGroup, Tag)>, ErrorResponder> {
        tag_groups::table
            .inner_join(tags::table)
            .filter(tag_groups::user_id.eq(user_id))
            .load(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
    /// List all user's tag groups and associated tags, as a Vec<TagGroupWithTags>
    pub fn list_all_tags_as_tag_group_with_tags(conn: &mut DBConn, user_id: i32) -> Result<Vec<TagGroupWithTags>, ErrorResponder> {
        let tags = TagGroup::list_all_tags(conn, user_id)?;
        // Create first a HashMap, and then map it to TagGroupWithTags.
        let mut map: HashMap<TagGroup, Vec<Tag>> = HashMap::new();
        for (a, b) in tags {
            map.entry(a).or_insert_with(Vec::new).push(b);
        }
        Ok(map.into_iter().map(|(tag_group, tags)| TagGroupWithTags { tag_group, tags }).collect())
    }

    pub fn from_id(conn: &mut DBConn, id: i32) -> Result<TagGroup, ErrorResponder> {
        Self::from_id_opt(conn, id)?.ok_or_else(|| ErrorType::TagNotFound.res())
    }
    pub fn from_id_opt(conn: &mut DBConn, id: i32) -> Result<Option<TagGroup>, ErrorResponder> {
        tag_groups::table
            .find(id)
            .first(conn)
            .optional()
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
    pub fn delete(conn: &mut DBConn, id: i32) -> Result<usize, ErrorResponder> {
        let deleted = diesel::delete(tag_groups::table.filter(tag_groups::id.eq(id)))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())?;
        diesel::delete(tags::table.filter(tags::tag_group_id.eq(id)))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())?;
        Ok(deleted)
    }

    /// Add a default tag to all pictures that don't have any tag from this tag group
    pub fn add_tags_to_pictures_without_tag_from_user(
        conn: &mut DBConn,
        tag_ids: &Vec<i32>,
        tag_group_id: i32,
        user_id: i32,
    ) -> Result<usize, ErrorResponder> {
        // Get all pictures accessible by the user that don't have any tag from this tag group
        let pictures_without_tag = pictures::table
            // Join with shared pictures
            .left_join(
                groups_pictures::table
                    .inner_join(shared_groups::table.on(shared_groups::dsl::group_id.eq(groups_pictures::dsl::group_id)))
                    .on(groups_pictures::dsl::picture_id.eq(pictures::dsl::id)),
            )
            // Filter allowed pictures
            .filter(shared_groups::dsl::user_id.eq(user_id).or(pictures::dsl::owner_id.eq(user_id)))
            // Filter pictures that have no tag group
            .filter(not(exists(
                pictures_tags::table
                    .inner_join(tags::table.on(tags::id.eq(pictures_tags::tag_id)))
                    .filter(pictures_tags::picture_id.eq(pictures::id))
                    .filter(tags::tag_group_id.eq(tag_group_id)),
            )))
            .select(pictures::id)
            .distinct()
            .load::<i64>(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())?;

        PictureTag::add_pictures_batch(conn, &tag_ids, &pictures_without_tag)
    }
    /// Add a default tag to all pictures that don't have any tag from this tag group along a vec of pictures
    pub fn add_default_tag_to_pictures_without_tag_from_list(
        conn: &mut DBConn,
        default_tag_id: i32,
        tag_group_id: i32,
        picture_ids: &Vec<i64>,
    ) -> Result<(), ErrorResponder> {
        // Get all pictures in that vec that don't have any tag from this tag group
        let pictures_without_tag = pictures::table
            .filter(pictures::id.eq_any(picture_ids))
            .filter(not(exists(
                pictures_tags::table
                    .inner_join(tags::table.on(tags::id.eq(pictures_tags::tag_id)))
                    .filter(pictures_tags::picture_id.eq(pictures::id))
                    .filter(tags::tag_group_id.eq(tag_group_id)),
            )))
            .select(pictures::id)
            .load::<i64>(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())?;

        PictureTag::add_pictures(conn, default_tag_id, &pictures_without_tag)?;
        Ok(())
    }

    /// Remove tags of this tag group from pictures
    pub fn remove_pictures(&self, conn: &mut DBConn, picture_ids: &Vec<i64>) -> Result<usize, ErrorResponder> {
        let tag_ids = Tag::list_tags(conn, self.id.unwrap())?.iter().map(|tag| tag.id).collect::<Vec<i32>>();
        diesel::delete(pictures_tags::table)
            .filter(pictures_tags::tag_id.eq_any(tag_ids))
            .filter(pictures_tags::picture_id.eq_any(picture_ids))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
}
