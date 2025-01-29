use crate::database::database::DBConn;
use crate::database::group::arrangement::Arrangement;
use crate::database::schema::*;
use crate::database::utils::get_last_inserted_id;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use diesel::prelude::*;
use diesel::{Associations, Identifiable, Queryable, Selectable};
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq, Serialize, JsonSchema)]
#[diesel(primary_key(id))]
#[diesel(belongs_to(Arrangement))]
#[diesel(table_name = groups)]
pub struct Group {
    pub id: u32,
    pub arrangement_id: u32,
    pub share_match_conversion: bool,
    pub name: String,
}

impl Group {
    pub fn insert(conn: &mut DBConn, arrangement_id: u32, name: String, share_match_conversion: bool) -> Result<Group, ErrorResponder> {
        let mut group = Group {
            id: 0,
            arrangement_id,
            name,
            share_match_conversion,
        };
        diesel::insert_into(groups::table)
            .values((
                groups::arrangement_id.eq(&group.arrangement_id),
                groups::name.eq(&group.name),
                groups::share_match_conversion.eq(&group.share_match_conversion),
            ))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())?;

        group.id = get_last_inserted_id(conn)? as u32;
        Ok(group)
    }

    pub fn from_id(conn: &mut DBConn, group_id: u32) -> Result<Group, ErrorResponder> {
        groups::table
            .filter(groups::id.eq(group_id))
            .first(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }

    pub fn from_id_and_arrangement(conn: &mut DBConn, group_id: u32, arrangement_id: u32) -> Result<Group, ErrorResponder> {
        groups::table
            .filter(groups::id.eq(group_id))
            .filter(groups::arrangement_id.eq(arrangement_id))
            .first(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }

    pub fn arrangement_id(&self) -> u32 {
        self.arrangement_id
    }

    pub fn add_pictures(&self, conn: &mut DBConn, picture_ids: Vec<u64>) -> Result<usize, ErrorResponder> {
        let values: Vec<_> = picture_ids
            .into_iter()
            .map(|pic_id| (groups_pictures::group_id.eq(self.id), groups_pictures::picture_id.eq(pic_id)))
            .collect();

        diesel::insert_into(groups_pictures::table)
            .values(&values)
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }

    pub fn remove_pictures(&self, conn: &mut DBConn, picture_ids: Vec<u64>) -> Result<usize, ErrorResponder> {
        diesel::delete(groups_pictures::table)
            .filter(groups_pictures::group_id.eq(self.id))
            .filter(groups_pictures::picture_id.eq_any(picture_ids))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
}
