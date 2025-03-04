use crate::database::database::DBConn;
use crate::database::picture::picture::Picture;
use crate::database::schema::*;
use crate::database::tag::tag::Tag;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use diesel::{Associations, ExpressionMethods, Identifiable, QueryDsl, Queryable, RunQueryDsl, Selectable};

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq)]
#[diesel(primary_key(picture_id, tag_id))]
#[diesel(belongs_to(Picture))]
#[diesel(belongs_to(Tag))]
#[diesel(table_name = pictures_tags)]
pub struct PictureTag {
    pub picture_id: u64,
    pub tag_id: u32,
}

impl PictureTag {
    /// Filter a pictures list against a tag
    pub fn get_tag_pictures(conn: &mut DBConn, tag_id: u32, picture_ids: &Vec<u64>) -> Result<Vec<u64>, ErrorResponder> {
        pictures_tags::table
            .filter(pictures_tags::tag_id.eq(tag_id))
            .filter(pictures_tags::picture_id.eq_any(picture_ids))
            .select((pictures_tags::picture_id))
            .load(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
}
