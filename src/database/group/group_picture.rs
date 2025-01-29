use crate::database::group::group::Group;
use crate::database::picture::picture::Picture;
use crate::database::schema::*;
use diesel::{Associations, Identifiable, Queryable, Selectable};

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq)]
#[diesel(primary_key(group_id, picture_id))]
#[diesel(belongs_to(Group))]
#[diesel(belongs_to(Picture))]
#[diesel(table_name = groups_pictures)]
pub struct GroupPicture {
    pub group_id: u32,
    pub picture_id: u64,
}
