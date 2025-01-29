use diesel::{Associations, Identifiable, Queryable, Selectable};

use crate::database::picture::picture::Picture;
use crate::database::schema::*;
use crate::database::tag::tag::Tag;

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq)]
#[diesel(primary_key(picture_id, tag_id))]
#[diesel(belongs_to(Picture))]
#[diesel(belongs_to(Tag))]
#[diesel(table_name = pictures_tags)]
pub struct PictureTag {
    pub picture_id: u64,
    pub tag_id: u32,
}
