use crate::database::database::DBConn;
use crate::database::schema::*;
use crate::database::user::user::User;
use crate::database::utils::get_last_inserted_id;
use crate::grouping::arrangement_strategy::ArrangementStrategy;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use diesel::prelude::*;
use diesel::{Associations, Identifiable, Queryable, Selectable};
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq, Clone, JsonSchema, Serialize)]
#[diesel(primary_key(id))]
#[diesel(belongs_to(User))]
#[diesel(table_name = arrangements)]
pub struct Arrangement {
    pub id: u32,
    pub user_id: u32,
    pub name: String,
    pub strong_match_conversion: bool,
    pub strategy: Option<Vec<u8>>,
    pub groups_dependant: bool,
    pub tags_dependant: bool,
    pub exif_dependant: bool,
}

impl Arrangement {
    pub fn new(
        conn: &mut DBConn,
        user_id: u32,
        name: String,
        strong_match_conversion: bool,
        strategy: ArrangementStrategy,
    ) -> Result<Arrangement, ErrorResponder> {
        let strategy_bytes = serde_json::to_vec(&strategy).map_err(|e| ErrorType::InternalError(e.to_string()).res_no_rollback())?;

        let mut arrangement = Arrangement {
            id: 0,
            user_id,
            name,
            strong_match_conversion,
            strategy: Some(strategy_bytes),
            groups_dependant: strategy.is_groups_dependant(),
            tags_dependant: strategy.is_tags_dependant(),
            exif_dependant: strategy.is_exif_dependant(),
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
    /// Deserialize the strategy and return it
    pub fn get_strategy(&self) -> Result<Option<ArrangementStrategy>, ErrorResponder> {
        if let Some(strategy) = &self.strategy {
            return Ok(Some(
                serde_json::from_slice(strategy).map_err(|e| ErrorType::InternalError(e.to_string()).res())?,
            ));
        }
        Ok(None)
    }
    /// Updates the strategy of this arrangement
    pub fn set_strategy(&mut self, conn: &mut DBConn, strategy: ArrangementStrategy) -> Result<(), ErrorResponder> {
        self.strategy = Some(serde_json::to_vec(&strategy).map_err(|e| ErrorType::InternalError(e.to_string()).res())?);

        diesel::update(arrangements::table.filter(arrangements::id.eq(self.id)))
            .set(arrangements::strategy.eq(&self.strategy))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())?;
        Ok(())
    }

    /// List all user’s arrangements
    pub fn list_arrangements(conn: &mut DBConn, user_id: u32) -> Result<Vec<Arrangement>, ErrorResponder> {
        arrangements::table
            .filter(arrangements::user_id.eq(user_id))
            .load(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
    /// List all users’ non-manual arrangements, providing the deserialized strategy, the list of groups and the list of dependant arrangements
    pub fn list_arrangements_and_groups(conn: &mut DBConn, user_id: u32) -> Result<Vec<ArrangementDetails>, ErrorResponder> {
        let mut arrangements = Self::list_arrangements(conn, user_id)?
            .into_iter()
            .filter(|arrangement| arrangement.strategy.is_some())
            .map(|arrangement| {
                let strategy = arrangement.get_strategy()?.unwrap();
                let groups = strategy.groupings.get_groups();
                let dependant_groups = strategy.get_dependant_groups();
                Ok::<ArrangementDetails, ErrorResponder>(ArrangementDetails {
                    arrangement,
                    strategy,
                    dependant_groups,
                    dependant_arrangements: vec![],
                    groups,
                })
            })
            .collect::<Result<Vec<ArrangementDetails>, ErrorResponder>>()?;

        if arrangements.len() == 0 {
            return Ok(vec![]);
        }

        for i in 0..arrangements.len() - 1 {
            let cloned_arrangements = arrangements.clone();
            arrangements[i].set_dependant_arrangements_auto(&cloned_arrangements);
        }
        Ok(arrangements)
    }
    /// Get all arrangements containing at least one of the provided groups
    pub fn get_arrangements_from_groups_ids(conn: &mut DBConn, groups_ids: Vec<u32>) -> Result<Vec<Arrangement>, ErrorResponder> {
        Ok(arrangements::table
            .inner_join(groups::table.on(groups::arrangement_id.eq(arrangements::id)))
            .filter(groups::id.eq_any(groups_ids))
            .select(arrangements::all_columns)
            .load::<Arrangement>(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())?)
    }
}
#[derive(Clone)]
pub struct ArrangementDetails {
    pub arrangement: Arrangement,
    pub strategy: ArrangementStrategy,
    pub dependant_groups: Vec<u32>,
    pub dependant_arrangements: Vec<u32>,
    pub groups: Vec<u32>,
}
impl ArrangementDetails {
    pub fn set_dependant_arrangements_auto(&mut self, all_arrangements_details: &Vec<ArrangementDetails>) {
        self.dependant_arrangements = all_arrangements_details
            .iter()
            .filter(|arr| arr.groups.iter().any(|g| self.dependant_groups.contains(g)))
            .map(|arr| arr.arrangement.id)
            .clone()
            .collect();
    }
}
