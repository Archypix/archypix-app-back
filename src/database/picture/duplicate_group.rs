use crate::database::schema::*;
use crate::database::user::user::User;
use diesel::{Associations, Identifiable, Queryable, Selectable};

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq)]
#[diesel(primary_key(id))]
#[diesel(belongs_to(User, foreign_key = user_id))]
#[diesel(table_name = duplicate_groups)]
pub struct DuplicateGroup {
    pub id: i32,
    pub user_id: i32,
}
