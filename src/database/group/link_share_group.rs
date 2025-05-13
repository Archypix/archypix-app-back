use crate::database::group::group::Group;
use crate::database::schema::*;
use diesel::{Associations, Identifiable, Queryable, Selectable};

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq)]
#[diesel(primary_key(token))]
#[diesel(belongs_to(Group))]
#[diesel(table_name = link_share_groups)]
pub struct LinkShareGroups {
    pub token: Vec<u8>,
    pub group_id: i32,
    pub permissions: i16,
}
