use diesel::{Associations, Identifiable, Queryable, Selectable};

use crate::database::group::{arrangement::Arrangement, group::Group};
use crate::database::hierarchy::hierarchy::Hierarchy;
use crate::database::schema::*;

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

impl HierarchyArrangements {}
