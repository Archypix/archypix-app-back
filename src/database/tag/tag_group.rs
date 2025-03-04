use crate::database::database::DBConn;
use crate::database::schema::*;
use crate::database::user::user::User;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use diesel::ExpressionMethods;
use diesel::QueryDsl;
use diesel::{Associations, Identifiable, Queryable, RunQueryDsl, Selectable};

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq)]
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
    /// List all user’s tag groups
    pub fn list_tag_groups(conn: &mut DBConn, user_id: u32) -> Result<Vec<TagGroup>, ErrorResponder> {
        tag_groups::table
            .filter(tag_groups::user_id.eq(user_id))
            .load(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
}
