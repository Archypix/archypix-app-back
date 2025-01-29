use crate::database::group::group::Group;
use crate::database::schema::*;
use crate::database::user::user::User;
use diesel::prelude::*;
use diesel::{Associations, Identifiable, Queryable, Selectable};

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq)]
#[diesel(primary_key(user_id, group_id))]
#[diesel(belongs_to(User))]
#[diesel(belongs_to(Group))]
#[diesel(table_name = shared_groups)]
pub struct SharedGroup {
    pub user_id: u32,
    pub group_id: u32,
    pub permissions: Vec<u8>,
    pub match_conversion_group_id: Option<u32>,
    pub copied: bool,
    pub confirmed: bool,
}
