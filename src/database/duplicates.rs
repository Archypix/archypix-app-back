use crate::database::picture::Picture;
use crate::database::schema::*;
use crate::database::user::User;
use diesel::{Associations, Identifiable, Queryable, Selectable};

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq)]
#[diesel(primary_key(id))]
#[diesel(belongs_to(User, foreign_key = user_id))]
#[diesel(table_name = duplicate_groups)]
pub struct DuplicateGroup {
    pub id: u32,
    pub user_id: u32,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq)]
#[diesel(primary_key(group_id, picture_id))]
#[diesel(belongs_to(DuplicateGroup, foreign_key = group_id))]
#[diesel(belongs_to(Picture, foreign_key = picture_id))]
#[diesel(table_name = duplicates)]
pub struct Duplicate {
    pub group_id: u32,
    pub picture_id: u64,
}


