use crate::database::database::DBConn;
use crate::database::group::arrangement::Arrangement;
use crate::database::schema::*;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use diesel::prelude::*;
use diesel::{Associations, Identifiable, Queryable, Selectable};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq, Deserialize, Serialize, JsonSchema)]
#[diesel(primary_key(id))]
#[diesel(belongs_to(Arrangement))]
#[diesel(table_name = groups)]
pub struct Group {
    pub id: i32,
    pub arrangement_id: i32,
    pub share_match_conversion: bool,
    pub name: String,
}

impl Group {
    pub fn insert(conn: &mut DBConn, arrangement_id: i32, name: String, share_match_conversion: bool) -> Result<Group, ErrorResponder> {
        diesel::insert_into(groups::table)
            .values((
                groups::arrangement_id.eq(arrangement_id),
                groups::name.eq(name),
                groups::share_match_conversion.eq(share_match_conversion),
            ))
            .get_result(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }

    pub fn from_id(conn: &mut DBConn, group_id: i32) -> Result<Group, ErrorResponder> {
        groups::table
            .filter(groups::id.eq(group_id))
            .first(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }

    pub fn from_id_and_arrangement(conn: &mut DBConn, group_id: i32, arrangement_id: i32) -> Result<Group, ErrorResponder> {
        groups::table
            .filter(groups::id.eq(group_id))
            .filter(groups::arrangement_id.eq(arrangement_id))
            .first(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }

    pub fn arrangement_id(&self) -> i32 {
        self.arrangement_id
    }

    pub fn add_pictures(conn: &mut DBConn, group_id: i32, picture_ids: &Vec<i64>) -> Result<usize, ErrorResponder> {
        let values: Vec<_> = picture_ids
            .into_iter()
            .map(|pic_id| (groups_pictures::group_id.eq(group_id), groups_pictures::picture_id.eq(*pic_id)))
            .collect();

        diesel::insert_into(groups_pictures::table)
            .values(&values)
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }

    pub fn remove_pictures(&self, conn: &mut DBConn, picture_ids: &Vec<i64>) -> Result<usize, ErrorResponder> {
        diesel::delete(groups_pictures::table)
            .filter(groups_pictures::group_id.eq(self.id))
            .filter(groups_pictures::picture_id.eq_any(picture_ids))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
}
