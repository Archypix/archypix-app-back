use crate::database::database::DBConn;
use crate::database::schema::*;
use crate::database::tag::tag::Tag;
use crate::database::user::user::User;
use crate::database::utils::get_last_inserted_id;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use diesel::ExpressionMethods;
use diesel::QueryDsl;
use diesel::{Associations, Identifiable, Queryable, RunQueryDsl, Selectable};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Identifiable, Associations, Serialize, JsonSchema, Debug, PartialEq, Eq, Hash)]
#[diesel(primary_key(id))]
#[diesel(belongs_to(User, foreign_key = user_id))]
#[diesel(table_name = tag_groups)]
pub struct TagGroup {
    pub id: u32,
    pub user_id: u32,
    pub name: String,
    pub multiple: bool,
    pub default_tag_id: Option<u32>,
    pub required: bool,
}

impl TagGroup {
    pub fn insert(conn: &mut DBConn, mut tag_group: TagGroup) -> Result<TagGroup, ErrorResponder> {
        let _ = diesel::insert_into(tag_groups::table)
            .values((
                tag_groups::user_id.eq(tag_group.user_id),
                tag_groups::name.eq(&tag_group.name.clone()),
                tag_groups::multiple.eq(tag_group.multiple),
                tag_groups::default_tag_id.eq(tag_group.default_tag_id),
                tag_groups::required.eq(tag_group.required),
            ))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())?;
        tag_group.id = get_last_inserted_id(conn)? as u32;
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
}
