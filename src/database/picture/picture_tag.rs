use crate::database::database::DBConn;
use crate::database::picture::picture::Picture;
use crate::database::schema::*;
use crate::database::tag::tag::Tag;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use diesel::{Associations, ExpressionMethods, Identifiable, JoinOnDsl, QueryDsl, Queryable, RunQueryDsl, Selectable};

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq)]
#[diesel(primary_key(picture_id, tag_id))]
#[diesel(belongs_to(Picture))]
#[diesel(belongs_to(Tag))]
#[diesel(table_name = pictures_tags)]
pub struct PictureTag {
    pub picture_id: i64,
    pub tag_id: i32,
}

impl PictureTag {
    /// Filter a pictures list against a tag
    pub fn filter_pictures_from_tag(conn: &mut DBConn, tag_id: i32, picture_ids: &Vec<i64>) -> Result<Vec<i64>, ErrorResponder> {
        pictures_tags::table
            .filter(pictures_tags::tag_id.eq(tag_id))
            .filter(pictures_tags::picture_id.eq_any(picture_ids))
            .select((pictures_tags::picture_id))
            .load(conn)
            .map_err(|e| ErrorType::DatabaseError("Failed to get tag pictures".to_string(), e).res())
    }
    /// Get all tags of a picture for a certain user
    pub fn get_picture_tags(conn: &mut DBConn, picture_id: i64, user_id: i32) -> Result<Vec<i32>, ErrorResponder> {
        pictures_tags::table
            .filter(pictures_tags::picture_id.eq(picture_id))
            // Check that the tag is owned by the owner
            .inner_join(tags::table.on(tags::id.eq(pictures_tags::tag_id)))
            .inner_join(tag_groups::table.on(tag_groups::id.eq(tags::tag_group_id)))
            .filter(tag_groups::user_id.eq(user_id))
            .select(pictures_tags::tag_id)
            .load(conn)
            .map_err(|e| ErrorType::DatabaseError("Failed to get picture tags".to_string(), e).res())
    }
}
