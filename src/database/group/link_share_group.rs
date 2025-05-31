use crate::database::database::DBConn;
use crate::database::group::group::Group;
use crate::database::schema::*;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use diesel::prelude::*;
use diesel::{Associations, ExpressionMethods, Identifiable, Queryable, RunQueryDsl, Selectable};

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq)]
#[diesel(primary_key(token))]
#[diesel(belongs_to(Group))]
#[diesel(table_name = link_share_groups)]
pub struct LinkShareGroups {
    pub token: Vec<u8>,
    pub group_id: i32,
    pub permissions: i16,
}

impl LinkShareGroups {
    pub fn delete_by_group_ids(conn: &mut DBConn, group_ids: &Vec<i32>) -> Result<(), ErrorResponder> {
        diesel::delete(link_share_groups::table.filter(link_share_groups::group_id.eq_any(group_ids)))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())?;
        Ok(())
    }
}
