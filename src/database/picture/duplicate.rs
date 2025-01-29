use crate::database::picture::duplicate_group::DuplicateGroup;
use crate::database::picture::picture::Picture;
use crate::database::schema::*;
use diesel::{Associations, Identifiable, Queryable, Selectable};

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq)]
#[diesel(primary_key(group_id, picture_id))]
#[diesel(belongs_to(DuplicateGroup, foreign_key = group_id))]
#[diesel(belongs_to(Picture, foreign_key = picture_id))]
#[diesel(table_name = duplicates)]
pub struct Duplicate {
    pub group_id: u32,
    pub picture_id: u64,
}
