use crate::database::database::DBConn;
use crate::database::schema::*;
use crate::database::tag::tag_group::TagGroup;
use crate::database::utils::get_last_inserted_id;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use diesel::{Associations, ExpressionMethods, Identifiable, Insertable, QueryDsl, Queryable, RunQueryDsl, Selectable};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Identifiable, Insertable, Associations, Serialize, JsonSchema, Debug, PartialEq)]
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
    /// List all TagGroup's tags
    pub fn list_tags(conn: &mut DBConn, tag_group_id: u32) -> Result<Vec<Tag>, ErrorResponder> {
        tags::table
            .filter(tags::tag_group_id.eq(tag_group_id))
            .load(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }

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
}
