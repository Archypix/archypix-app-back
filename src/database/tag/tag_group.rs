use diesel::{Associations, Identifiable, Queryable, Selectable};

use crate::database::schema::*;
use crate::database::user::user::User;

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq)]
#[diesel(primary_key(id))]
#[diesel(belongs_to(User, foreign_key = user_id))]
#[diesel(table_name = tag_groups)]
pub struct TagGroup {
    pub id: u32,
    pub user_id: u32,
    pub name: String,
    pub multiple: bool,
    pub required: bool,
}
