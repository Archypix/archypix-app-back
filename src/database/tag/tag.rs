use crate::database::database::DBConn;
use crate::database::schema::*;
use crate::database::tag::tag_group::TagGroup;
use crate::database::utils::get_last_inserted_id;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use diesel::query_dsl::InternalJoinDsl;
use diesel::{
    Associations, ExpressionMethods, Identifiable, Insertable, JoinOnDsl, OptionalExtension, QueryDsl, Queryable, RunQueryDsl, Selectable, Table,
};
use rocket::yansi::Paint;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Identifiable, Insertable, Associations, Serialize, Deserialize, JsonSchema, Debug, PartialEq)]
#[diesel(primary_key(id))]
#[diesel(belongs_to(TagGroup))]
#[diesel(table_name = tags)]
pub struct Tag {
    pub id: u32,
    pub tag_group_id: u32,
    pub name: String,
    pub color: Vec<u8>,
    pub is_default: bool,
}

impl Tag {
    pub fn insert(conn: &mut DBConn, mut tag: Tag) -> Result<Tag, ErrorResponder> {
        let _ = diesel::insert_into(tags::table)
            .values((
                tags::tag_group_id.eq(tag.tag_group_id),
                tags::name.eq(&tag.name.clone()),
                tags::color.eq(tag.color.clone()),
                tags::is_default.eq(tag.is_default),
            ))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())?;
        tag.id = get_last_inserted_id(conn)? as u32;
        Ok(tag)
    }
    // Edit a tag name, color, and default
    pub fn patch(conn: &mut DBConn, tag: Tag) -> Result<Tag, ErrorResponder> {
        let _ = diesel::update(tags::table.find(tag.id))
            .set((tags::name.eq(&tag.name), tags::color.eq(&tag.color), tags::is_default.eq(tag.is_default)))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())?;
        Ok(tag)
    }

    /// List all TagGroup's tags
    pub fn list_tags(conn: &mut DBConn, tag_group_id: u32) -> Result<Vec<Tag>, ErrorResponder> {
        tags::table
            .filter(tags::tag_group_id.eq(tag_group_id))
            .load(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
    pub fn from_id_with_tag_group(conn: &mut DBConn, tag_id: u32) -> Result<(Tag, TagGroup), ErrorResponder> {
        tags::table
            .inner_join(tag_groups::table.on(tags::tag_group_id.eq(tag_groups::id)))
            .filter(tags::id.eq(tag_id))
            .select((tags::all_columns, tag_groups::all_columns))
            .first(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
    pub fn from_id(conn: &mut DBConn, tag_id: u32) -> Result<Tag, ErrorResponder> {
        Self::from_id_opt(conn, tag_id)?.ok_or_else(|| ErrorType::TagNotFound.res())
    }
    pub fn from_id_opt(conn: &mut DBConn, tag_id: u32) -> Result<Option<Tag>, ErrorResponder> {
        tags::table
            .find(tag_id)
            .first(conn)
            .optional()
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
    pub fn delete(conn: &mut DBConn, id: u32) -> Result<usize, ErrorResponder> {
        diesel::delete(tags::table.filter(tags::id.eq(id)))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
}
