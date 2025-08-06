use crate::database::database::DBConn;
use crate::database::picture::picture::Picture;
use crate::database::schema::*;
use crate::database::user::user::User;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use diesel::prelude::*;
use diesel::query_dsl::InternalJoinDsl;
use diesel::{Associations, Identifiable, Queryable, Selectable};
use diesel::{BoolExpressionMethods, ExpressionMethods, OptionalExtension, RunQueryDsl};
use diesel::{JoinOnDsl, NullableExpressionMethods, SelectableHelper};
use rocket::serde::Deserialize;
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Queryable, Selectable, Identifiable, Associations, Serialize, Deserialize, JsonSchema, Debug, PartialEq, Clone)]
#[diesel(primary_key(user_id, picture_id))]
#[diesel(belongs_to(User))]
#[diesel(belongs_to(Picture))]
#[diesel(table_name = ratings)]
pub struct Rating {
    pub user_id: i32,
    pub picture_id: i64,
    pub rating: i16,
}

impl Rating {
    pub fn from_picture_id(conn: &mut DBConn, picture_id: i64, user_id: i32) -> Result<Option<Rating>, ErrorResponder> {
        ratings::table
            .filter(ratings::dsl::picture_id.eq(picture_id))
            .filter(ratings::dsl::user_id.eq(user_id))
            .first::<Rating>(conn)
            .optional()
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }

    pub fn from_picture_id_including_friends(conn: &mut DBConn, picture_id: i64, user_id: i32) -> Result<Vec<Rating>, ErrorResponder> {
        Self::from_picture_ids_including_friends(conn, user_id, &[picture_id])
    }

    /// Gets ratings for a slice of pictures for a user and its friends
    pub fn from_picture_ids_including_friends(conn: &mut DBConn, user_id: i32, picture_ids: &[i64]) -> Result<Vec<Rating>, ErrorResponder> {
        ratings::table
            .filter(ratings::dsl::picture_id.eq_any(picture_ids))
            .filter(
                ratings::dsl::user_id
                    .eq(user_id)
                    .or(ratings::dsl::user_id.eq_any(friends::table.filter(friends::dsl::user_id_1.eq(user_id)).select(friends::dsl::user_id_2)))
                    .or(ratings::dsl::user_id.eq_any(friends::table.filter(friends::dsl::user_id_2.eq(user_id)).select(friends::dsl::user_id_1))),
            )
            .load(conn)
            .map_err(|e| ErrorType::DatabaseError("Failed to get ratings".to_string(), e).res())
    }

    /// Get rating statistics for a slice of pictures.
    /// Returned tuple contains: (average_user_rating, average_global_rating, friends user ids that have ratings for at least one picture)
    pub fn get_mixed_pictures_ratings(
        conn: &mut DBConn,
        user_id: i32,
        pictures_ids: &[i64],
    ) -> Result<(Option<i16>, Option<i16>, Vec<i32>), ErrorResponder> {
        let all_ratings = Self::from_picture_ids_including_friends(conn, user_id, pictures_ids)?;
        if all_ratings.is_empty() {
            return Ok((None, None, Vec::new()));
        }
        let user_ratings: Vec<&Rating> = all_ratings.iter().filter(|r| r.user_id == user_id).collect();

        let average_user_rating = Self::average_ratings_value(&user_ratings);
        let average_global_rating = Self::average_ratings_value(&all_ratings);

        let mut rating_users: Vec<i32> = all_ratings.iter().map(|r| r.user_id).filter(|uid| *uid != user_id).collect();
        rating_users.sort();
        rating_users.dedup();

        Ok((average_user_rating, average_global_rating, rating_users))
    }

    fn average_ratings_value<T>(ratings: &[T]) -> Option<i16>
    where
        T: std::borrow::Borrow<Rating>,
    {
        if ratings.is_empty() {
            None
        } else {
            Some((ratings.iter().map(|r| r.borrow().rating as i32).sum::<i32>() / ratings.len() as i32) as i16)
        }
    }
}
