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
    pub user_id: u32,
    pub picture_id: u64,
    pub rating: u8,
}

impl Rating {
    pub fn from_picture_id(conn: &mut DBConn, picture_id: u64, user_id: u32) -> Result<Option<Rating>, ErrorResponder> {
        ratings::table
            .filter(ratings::dsl::picture_id.eq(picture_id))
            .filter(ratings::dsl::user_id.eq(user_id))
            .first::<Rating>(conn)
            .optional()
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }

    pub fn from_picture_id_including_friends(conn: &mut DBConn, picture_id: u64, user_id: u32) -> Result<Vec<Rating>, ErrorResponder> {
        ratings::table
            .filter(ratings::dsl::picture_id.eq(picture_id))
            .filter(
                ratings::dsl::user_id
                    .eq(user_id)
                    .or(ratings::dsl::user_id.eq_any(friends::table.filter(friends::dsl::user_id_1.eq(user_id)).select(friends::dsl::user_id_2)))
                    .or(ratings::dsl::user_id.eq_any(friends::table.filter(friends::dsl::user_id_2.eq(user_id)).select(friends::dsl::user_id_1))),
            )
            .load(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
}
