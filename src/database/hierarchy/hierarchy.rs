use diesel::{Associations, Identifiable, Queryable, Selectable};

use crate::database::schema::*;
use crate::database::user::user::User;

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq)]
#[diesel(primary_key(id))]
#[diesel(belongs_to(User))]
#[diesel(table_name = hierarchies)]
pub struct Hierarchy {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
}
impl Hierarchy {}
