use crate::database::picture::picture::Picture;
use crate::database::schema::*;
use crate::database::user::user::User;
use diesel_derives::{Associations, Identifiable, Queryable, Selectable};

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq)]
#[diesel(primary_key(user_id, picture_id))]
#[diesel(belongs_to(User))]
#[diesel(belongs_to(Picture))]
#[diesel(table_name = ratings)]
pub struct Rating {
    pub user_id: u32,
    pub picture_id: u64,
    pub rating: i8,
}
