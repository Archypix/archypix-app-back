use crate::database::database::DBConn;
use crate::database::schema::*;
use crate::database::tag::tag_group::TagGroup;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use diesel::query_dsl::InternalJoinDsl;
use diesel::{
    Associations, ExpressionMethods, Identifiable, Insertable, JoinOnDsl, OptionalExtension, QueryDsl, Queryable, RunQueryDsl, Selectable, Table,
};
use rocket::yansi::Paint;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Identifiable, Insertable, Associations, Serialize, Deserialize, JsonSchema, Debug, PartialEq, Clone)]
#[diesel(primary_key(id))]
#[diesel(belongs_to(TagGroup))]
#[diesel(table_name = tags)]
pub struct Tag {
    pub id: i32,
    pub tag_group_id: i32,
    pub name: String,
    pub color: Vec<u8>,
    pub is_default: bool,
}

impl Tag {
    pub fn insert(conn: &mut DBConn, mut tag: Tag) -> Result<Tag, ErrorResponder> {
        diesel::insert_into(tags::table)
            .values((
                tags::tag_group_id.eq(tag.tag_group_id),
                tags::name.eq(&tag.name.clone()),
                tags::color.eq(tag.color.clone()),
                tags::is_default.eq(tag.is_default),
            ))
            .get_result(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
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
    pub fn list_tags(conn: &mut DBConn, tag_group_id: i32) -> Result<Vec<Tag>, ErrorResponder> {
        tags::table
            .filter(tags::tag_group_id.eq(tag_group_id))
            .load(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
    pub fn from_id_with_tag_group(conn: &mut DBConn, tag_id: i32) -> Result<(Tag, TagGroup), ErrorResponder> {
        tags::table
            .inner_join(tag_groups::table.on(tags::tag_group_id.eq(tag_groups::id)))
            .filter(tags::id.eq(tag_id))
            .select((tags::all_columns, tag_groups::all_columns))
            .first(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
    pub fn from_id(conn: &mut DBConn, tag_id: i32) -> Result<Tag, ErrorResponder> {
        Self::from_id_opt(conn, tag_id)?.ok_or_else(|| ErrorType::TagNotFound.res())
    }
    pub fn from_id_opt(conn: &mut DBConn, tag_id: i32) -> Result<Option<Tag>, ErrorResponder> {
        tags::table
            .find(tag_id)
            .first(conn)
            .optional()
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
    pub fn from_ids(conn: &mut DBConn, tag_ids: Vec<i32>) -> Result<Vec<Tag>, ErrorResponder> {
        tags::table
            .filter(tags::id.eq_any(tag_ids))
            .load(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
    pub fn delete(conn: &mut DBConn, id: i32) -> Result<usize, ErrorResponder> {
        // Delete all pictures with this tag
        diesel::delete(pictures_tags::table.filter(pictures_tags::tag_id.eq(id)))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())?;

        diesel::delete(tags::table.filter(tags::id.eq(id)))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }

    pub fn add_pictures(conn: &mut DBConn, tag_id: i32, picture_ids: Vec<i64>) -> Result<usize, ErrorResponder> {
        let values: Vec<_> = picture_ids
            .into_iter()
            .map(|pic_id| (pictures_tags::tag_id.eq(tag_id), pictures_tags::picture_id.eq(pic_id)))
            .collect();

        diesel::insert_into(pictures_tags::table)
            .values(&values)
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
    pub fn add_pictures_batch(conn: &mut DBConn, tag_ids: Vec<i32>, picture_ids: Vec<i64>) -> Result<usize, ErrorResponder> {
        let values: Vec<_> = tag_ids
            .iter()
            .flat_map(|tag_id| {
                picture_ids
                    .iter()
                    .map(move |pic_id| (pictures_tags::tag_id.eq(tag_id), pictures_tags::picture_id.eq(pic_id)))
            })
            .collect();

        diesel::insert_into(pictures_tags::table)
            .values(&values)
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }

    pub fn remove_pictures(conn: &mut DBConn, tag_id: i32, picture_ids: Vec<i64>) -> Result<usize, ErrorResponder> {
        diesel::delete(pictures_tags::table)
            .filter(pictures_tags::tag_id.eq(tag_id))
            .filter(pictures_tags::picture_id.eq_any(picture_ids))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
    pub fn remove_pictures_batch(conn: &mut DBConn, tag_ids: Vec<i32>, picture_ids: Vec<i64>) -> Result<usize, ErrorResponder> {
        diesel::delete(pictures_tags::table)
            .filter(pictures_tags::tag_id.eq_any(tag_ids))
            .filter(pictures_tags::picture_id.eq_any(picture_ids))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
}
