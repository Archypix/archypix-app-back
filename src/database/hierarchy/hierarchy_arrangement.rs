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
    pub hierarchy_id: u32,
    pub arrangement_id: u32,
    pub parent_group_id: u32,
}

impl HierarchyArrangements {}
