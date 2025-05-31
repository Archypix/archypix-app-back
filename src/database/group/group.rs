use crate::database::database::DBConn;
use crate::database::group::arrangement;
use crate::database::group::arrangement::Arrangement;
use crate::database::schema::*;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use diesel::prelude::*;
use diesel::{Associations, Identifiable, Queryable, Selectable};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq, Clone, Deserialize, Serialize, JsonSchema)]
#[diesel(primary_key(id))]
#[diesel(belongs_to(Arrangement))]
#[diesel(table_name = groups)]
pub struct Group {
    pub id: i32,
    pub arrangement_id: i32,
    pub share_match_conversion: bool,
    pub name: String,
    pub to_be_deleted: bool,
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

    /// Retrieves all groups for a given arrangement, including those marked for deletion.
    pub fn from_arrangement_all(conn: &mut DBConn, arrangement_id: i32) -> Result<Vec<Group>, ErrorResponder> {
        groups::table
            .filter(groups::arrangement_id.eq(arrangement_id))
            .get_results(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
    pub fn from_id_and_arrangement(conn: &mut DBConn, group_id: i32, arrangement_id: i32) -> Result<Group, ErrorResponder> {
        groups::table
            .filter(groups::id.eq(group_id))
            .filter(groups::arrangement_id.eq(arrangement_id))
            .first(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
    pub fn from_user_id(conn: &mut DBConn, user_id: i32) -> Result<Vec<Group>, ErrorResponder> {
        groups::table
            .inner_join(arrangements::table.on(groups::arrangement_id.eq(arrangements::id)))
            .filter(arrangements::user_id.eq(user_id))
            .select(Group::as_select())
            .load(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }

    // Adds a picture to the group and returns the vec of added picture ids (the ones that were not already in the group)
    pub fn add_pictures(conn: &mut DBConn, group_id: i32, picture_ids: &Vec<i64>) -> Result<Vec<i64>, ErrorResponder> {
        let values: Vec<_> = picture_ids
            .into_iter()
            .map(|pic_id| (groups_pictures::group_id.eq(group_id), groups_pictures::picture_id.eq(*pic_id)))
            .collect();

        diesel::insert_into(groups_pictures::table)
            .values(&values)
            .on_conflict_do_nothing()
            .returning(groups_pictures::picture_id)
            .get_results(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }

    pub fn remove_pictures(conn: &mut DBConn, group_id: i32, picture_ids: &Vec<i64>) -> Result<Vec<i64>, ErrorResponder> {
        diesel::delete(groups_pictures::table)
            .filter(groups_pictures::group_id.eq(group_id))
            .filter(groups_pictures::picture_id.eq_any(picture_ids))
            .returning(groups_pictures::picture_id)
            .get_results(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
    pub fn clear_and_get_pictures(conn: &mut DBConn, group_id: i32) -> Result<Vec<i64>, ErrorResponder> {
        diesel::delete(groups_pictures::table)
            .filter(groups_pictures::group_id.eq(group_id))
            .returning(groups_pictures::picture_id)
            .get_results(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
    pub fn delete_by_arrangement_id(conn: &mut DBConn, arrangement_id: i32) -> Result<(), ErrorResponder> {
        diesel::delete(groups::table.filter(groups::arrangement_id.eq(arrangement_id)))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())?;
        Ok(())
    }
    /// Marks all groups for a given arrangement as to be deleted.
    pub fn mark_all_as_to_be_deleted(conn: &mut DBConn, arrangement_id: i32) -> Result<(), ErrorResponder> {
        diesel::update(groups::table.filter(groups::arrangement_id.eq(arrangement_id)))
            .set(groups::to_be_deleted.eq(true))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())?;
        Ok(())
    }
}
