use crate::database::database::DBConn;
use crate::database::picture::picture::Picture;
use crate::database::schema::*;
use crate::database::tag::tag::Tag;
use crate::database::tag::tag_group::TagGroup;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use diesel::dsl::{exists, not};
use diesel::{Associations, ExpressionMethods, Identifiable, JoinOnDsl, QueryDsl, Queryable, RunQueryDsl, Selectable};
use itertools::Itertools;
use std::collections::HashMap;

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
            .select(pictures_tags::picture_id)
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

    pub fn add_pictures(conn: &mut DBConn, tag_id: i32, picture_ids: &Vec<i64>) -> Result<usize, ErrorResponder> {
        let values: Vec<_> = picture_ids
            .into_iter()
            .map(|pic_id| (pictures_tags::tag_id.eq(tag_id), pictures_tags::picture_id.eq(pic_id)))
            .collect();

        diesel::insert_into(pictures_tags::table)
            .values(&values)
            .on_conflict_do_nothing()
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
    pub fn add_pictures_batch(conn: &mut DBConn, tag_ids: &Vec<i32>, picture_ids: &Vec<i64>) -> Result<usize, ErrorResponder> {
        let values: Vec<_> = tag_ids
            .iter()
            .flat_map(|tag_id| {
                picture_ids
                    .iter()
                    .map(move |pic_id| (pictures_tags::tag_id.eq(tag_id), pictures_tags::picture_id.eq(pic_id)))
            })
            .collect();

        diesel::insert_into(pictures_tags::table)
            .values(&values)
            .on_conflict_do_nothing()
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
    pub fn remove_pictures(conn: &mut DBConn, tag_id: i32, picture_ids: &Vec<i64>) -> Result<usize, ErrorResponder> {
        diesel::delete(pictures_tags::table)
            .filter(pictures_tags::tag_id.eq(tag_id))
            .filter(pictures_tags::picture_id.eq_any(picture_ids))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
    pub fn remove_pictures_batch(conn: &mut DBConn, tag_ids: &Vec<i32>, picture_ids: &Vec<i64>) -> Result<usize, ErrorResponder> {
        diesel::delete(pictures_tags::table)
            .filter(pictures_tags::tag_id.eq_any(tag_ids))
            .filter(pictures_tags::picture_id.eq_any(picture_ids))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }

    /// Add all the users’ default tags to a list of pictures.
    pub fn add_default_tags(conn: &mut DBConn, user_id: i32, picture_ids: &Vec<i64>) -> Result<usize, ErrorResponder> {
        let default_tags = tags::table
            .inner_join(tag_groups::table.on(tag_groups::id.eq(tags::tag_group_id)))
            .filter(tag_groups::user_id.eq(user_id))
            .filter(tags::is_default.eq(true))
            .select(tags::id)
            .load::<i32>(conn)
            .map_err(|e| ErrorType::DatabaseError("Failed to get default tags".to_string(), e).res())?;

        Self::add_pictures_batch(conn, &default_tags, picture_ids)
    }

    /// For every tag group of the user, add the defaults tags of the tag group only to provided pictures that have not any tag of this tag group.
    pub fn add_default_tags_to_pictures_without_tags(conn: &mut DBConn, user_id: i32, picture_ids: &Vec<i64>) -> Result<(), ErrorResponder> {
        TagGroup::list_all_tags_as_tag_group_with_tags(conn, user_id)?
            .iter()
            .try_for_each(|tgwt| {
                let pictures_without_tag = pictures::table
                    .filter(pictures::id.eq_any(picture_ids))
                    .filter(not(exists(
                        pictures_tags::table
                            .inner_join(tags::table.on(tags::id.eq(pictures_tags::tag_id)))
                            .filter(pictures_tags::picture_id.eq(pictures::id))
                            .filter(tags::tag_group_id.eq(tgwt.tag_group.id.unwrap())),
                    )))
                    .select(pictures::id)
                    .load::<i64>(conn)
                    .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())?;

                let default_tags = tgwt.tags.iter().filter(|t| t.is_default).map(|t| t.id).collect_vec();
                Self::add_pictures_batch(conn, &default_tags, &pictures_without_tag)?;
                Ok(())
            })
    }

    /// Get common and mixed tags from an array of pictures
    /// Returned tuple contains arrays of tag ids: (common_tags, mixed_tags)
    pub fn get_mixed_pictures_tags(conn: &mut DBConn, user_id: i32, picture_ids: &[i64]) -> Result<(Vec<i32>, Vec<i32>), ErrorResponder> {
        let all_tags: Vec<(i64, i32)> = pictures_tags::table
            .filter(pictures_tags::picture_id.eq_any(picture_ids))
            .inner_join(tags::table.on(tags::id.eq(pictures_tags::tag_id)))
            .inner_join(tag_groups::table.on(tag_groups::id.eq(tags::tag_group_id)))
            .filter(tag_groups::user_id.eq(user_id))
            .select((pictures_tags::picture_id, pictures_tags::tag_id))
            .load(conn)
            .map_err(|e| ErrorType::DatabaseError("Failed to get picture tags".to_string(), e).res())?;

        // Group tags by tag_id and count how many pictures have each tag
        let mut tag_counts: HashMap<i32, usize> = HashMap::new();
        for (_, tag_id) in all_tags {
            *tag_counts.entry(tag_id).or_insert(0) += 1;
        }

        let total_pictures = picture_ids.len();
        let mut common_tags = Vec::new();
        let mut mixed_tags = Vec::new();

        for (tag_id, count) in tag_counts {
            if count == total_pictures {
                common_tags.push(tag_id);
            } else {
                mixed_tags.push(tag_id);
            }
        }
        common_tags.sort();
        mixed_tags.sort();
        Ok((common_tags, mixed_tags))
    }
}
