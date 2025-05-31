use crate::database::database::DBConn;
use crate::database::group::group::Group;
use crate::database::schema::*;
use crate::database::user::user::User;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use diesel::ExpressionMethods;
use diesel::QueryDsl;
use diesel::{Associations, Identifiable, Queryable, RunQueryDsl, Selectable};

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq)]
#[diesel(primary_key(user_id, group_id))]
#[diesel(belongs_to(User))]
#[diesel(belongs_to(Group))]
#[diesel(table_name = shared_groups)]
pub struct SharedGroup {
    pub user_id: i32,
    pub group_id: i32,
    pub permissions: i16,
    pub match_conversion_group_id: Option<i32>,
    pub copied: bool,
    pub confirmed: bool,
}

impl SharedGroup {
    pub fn from_group_id(conn: &mut DBConn, group_id: i32) -> Result<Vec<SharedGroup>, ErrorResponder> {
        shared_groups::table
            .filter(shared_groups::group_id.eq(group_id))
            .load(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }

    pub fn delete_by_group_ids(conn: &mut DBConn, group_ids: &Vec<i32>) -> Result<(), ErrorResponder> {
        diesel::delete(shared_groups::table.filter(shared_groups::group_id.eq_any(group_ids)))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())?;
        Ok(())
    }
}
