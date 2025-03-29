use crate::database::database::DBConn;
use crate::database::schema::*;
use crate::database::tag::tag::Tag;
use crate::database::user::user::User;
use crate::database::utils::get_last_inserted_id;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use diesel::QueryDsl;
use diesel::{Associations, Identifiable, Queryable, RunQueryDsl, Selectable};
use diesel::{ExpressionMethods, OptionalExtension};
use rocket::serde::json::Json;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Identifiable, Associations, Serialize, Deserialize, JsonSchema, Debug, PartialEq, Eq, Hash, Clone)]
#[diesel(primary_key(id))]
#[diesel(belongs_to(User, foreign_key = user_id))]
#[diesel(table_name = tag_groups)]
pub struct TagGroup {
    #[diesel(deserialize_as = u32, serialize_as = u32)]
    pub id: Option<u32>,
    pub user_id: u32,
    pub name: String,
    pub multiple: bool,
    pub required: bool,
}

impl TagGroup {
    pub fn insert(conn: &mut DBConn, mut tag_group: TagGroup) -> Result<TagGroup, ErrorResponder> {
        let _ = diesel::insert_into(tag_groups::table)
            .values((
                tag_groups::user_id.eq(tag_group.user_id),
                tag_groups::name.eq(&tag_group.name.clone()),
                tag_groups::multiple.eq(tag_group.multiple),
                tag_groups::required.eq(tag_group.required),
            ))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())?;
        tag_group.id = Some(get_last_inserted_id(conn)? as u32);
        Ok(tag_group)
    }
    // Edit a tag group name, multiple, default tag, and required, works only if the tag group is owned by the user
    pub fn patch(conn: &mut DBConn, tag_group: TagGroup, user_id: u32) -> Result<TagGroup, ErrorResponder> {
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

    /// List all user’s tag groups
    pub fn list_tag_groups(conn: &mut DBConn, user_id: u32) -> Result<Vec<TagGroup>, ErrorResponder> {
        tag_groups::table
            .filter(tag_groups::user_id.eq(user_id))
            .load(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
    /// List all user’s tag groups and associated tags
    pub fn list_all_tags(conn: &mut DBConn, user_id: u32) -> Result<Vec<(TagGroup, Tag)>, ErrorResponder> {
        tag_groups::table
            .inner_join(tags::table)
            .filter(tag_groups::user_id.eq(user_id))
            .load(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }

    pub fn from_id(conn: &mut DBConn, id: u32) -> Result<TagGroup, ErrorResponder> {
        Self::from_id_opt(conn, id)?.ok_or_else(|| ErrorType::TagNotFound.res())
    }
    pub fn from_id_opt(conn: &mut DBConn, id: u32) -> Result<Option<TagGroup>, ErrorResponder> {
        tag_groups::table
            .find(id)
            .first(conn)
            .optional()
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
    pub fn delete(conn: &mut DBConn, id: u32) -> Result<usize, ErrorResponder> {
        let deleted = diesel::delete(tag_groups::table.filter(tag_groups::id.eq(id)))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())?;
        diesel::delete(tags::table.filter(tags::tag_group_id.eq(id)))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())?;
        Ok(deleted)
    }
}
