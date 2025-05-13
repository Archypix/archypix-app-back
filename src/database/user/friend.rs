use crate::database::schema::*;
use crate::database::user::user::User;
use diesel::{Associations, Identifiable, Queryable, Selectable};

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq)]
#[diesel(primary_key(user_id_1, user_id_2))]
#[diesel(belongs_to(User, foreign_key = user_id_1, foreign_key = user_id_2))]
#[diesel(table_name = friends)]
pub struct Friends {
    pub user_id_1: i32,
    pub user_id_2: i32,
}
