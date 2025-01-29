use crate::database::schema::*;
use crate::database::tag::tag_group::TagGroup;
use diesel::{Associations, Identifiable, Insertable, Queryable, Selectable};

#[derive(Queryable, Selectable, Identifiable, Insertable, Associations, Debug, PartialEq)]
#[diesel(primary_key(id))]
#[diesel(belongs_to(TagGroup))]
#[diesel(table_name = tags)]
pub struct Tag {
    pub id: u32,
    pub tag_group_id: u32,
    pub name: String,
    pub color: Vec<u8>,
    pub is_default: bool,
}
