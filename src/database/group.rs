use crate::database::database::DBConn;
use crate::database::schema::*;
use crate::database::utils::get_last_inserted_id;
use crate::database::{picture::Picture, user::User};
use crate::grouping::grouping_strategy::GroupingStrategy;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use diesel::prelude::*;
use diesel::{select, Associations, Identifiable, QueryResult, Queryable, Selectable};
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq)]
#[diesel(primary_key(id))]
#[diesel(belongs_to(User))]
#[diesel(table_name = arrangements)]
pub struct Arrangement {
    pub id: u32,
    pub user_id: u32,
    pub name: String,
    pub strong_match_conversion: bool,
    pub strategy: Vec<u8>,
}

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

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq)]
#[diesel(primary_key(group_id, picture_id))]
#[diesel(belongs_to(Group))]
#[diesel(belongs_to(Picture))]
#[diesel(table_name = groups_pictures)]
pub struct GroupPicture {
    pub group_id: u32,
    pub picture_id: u64,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq)]
#[diesel(primary_key(token))]
#[diesel(belongs_to(Group))]
#[diesel(table_name = link_share_groups)]
pub struct LinkShareGroups {
    pub token: Vec<u8>,
    pub group_id: u32,
    pub permissions: u8,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq)]
#[diesel(primary_key(user_id, group_id))]
#[diesel(belongs_to(User))]
#[diesel(belongs_to(Group))]
#[diesel(table_name = shared_groups)]
pub struct SharedGroup {
    pub user_id: u32,
    pub group_id: u32,
    pub permissions: Vec<u8>,
    pub match_conversion_group_id: Option<u32>,
    pub copied: bool,
    pub confirmed: bool,
}

impl Arrangement {
    pub fn new(
        conn: &mut DBConn,
        user_id: u32,
        name: String,
        strong_match_conversion: bool,
        strategy: GroupingStrategy,
    ) -> Result<Arrangement, ErrorResponder> {
        let strategy_bytes = serde_json::to_vec(&strategy).map_err(|e| ErrorType::InternalError(e.to_string()).res())?;

        let mut arrangement = Arrangement {
            id: 0,
            user_id,
            name,
            strong_match_conversion,
            strategy: strategy_bytes,
        };

        let _ = diesel::insert_into(arrangements::table)
            .values((
                arrangements::name.eq(&arrangement.name),
                arrangements::strategy.eq(&arrangement.strategy),
                arrangements::strong_match_conversion.eq(&arrangement.strong_match_conversion),
            ))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())?;

        arrangement.id = get_last_inserted_id(conn)? as u32;
        Ok(arrangement)
    }

    pub fn from_id_and_user_id(conn: &mut DBConn, arrangement_id: u32, user_id: u32) -> Result<Arrangement, ErrorResponder> {
        Self::from_id_and_user_id_opt(conn, arrangement_id, user_id)?.ok_or_else(|| ErrorType::ArrangementNotFound.res())
    }
    pub fn from_id_and_user_id_opt(conn: &mut DBConn, arrangement_id: u32, user_id: u32) -> Result<Option<Arrangement>, ErrorResponder> {
        arrangements::table
            .filter(arrangements::id.eq(arrangement_id))
            .filter(arrangements::user_id.eq(user_id))
            .first(conn)
            .optional()
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }

    pub fn get_strategy(&self) -> Result<GroupingStrategy, ErrorResponder> {
        serde_json::from_slice(&self.strategy).map_err(|e| ErrorType::InternalError(e.to_string()).res())
    }
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

impl GroupPicture {}

impl SharedGroup {}
