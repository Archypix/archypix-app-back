use diesel::{Associations, Identifiable, Queryable, RunQueryDsl, Selectable};

use crate::database::database::DBConn;
use crate::database::group::{arrangement::Arrangement, group::Group};
use crate::database::hierarchy::hierarchy::Hierarchy;
use crate::database::schema::*;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use diesel::prelude::*;

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq)]
#[diesel(primary_key(hierarchy_id, arrangement_id))]
#[diesel(belongs_to(Hierarchy))]
#[diesel(belongs_to(Arrangement))]
#[diesel(belongs_to(Group, foreign_key = parent_group_id))]
#[diesel(table_name = hierarchies_arrangements)]
pub struct HierarchyArrangements {
    pub hierarchy_id: i32,
    pub arrangement_id: i32,
    pub parent_group_id: Option<i32>,
}

impl HierarchyArrangements {
    pub fn from_arrangement_id(conn: &mut DBConn, arrangement_id: i32) -> Result<Vec<HierarchyArrangements>, ErrorResponder> {
        hierarchies_arrangements::table
            .filter(hierarchies_arrangements::arrangement_id.eq(arrangement_id))
            .load(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
}
