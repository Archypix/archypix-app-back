use crate::database::database::DBConn;
use crate::database::schema::*;
use crate::database::user::user::User;
use crate::database::utils::get_last_inserted_id;
use crate::grouping::grouping_strategy::GroupingStrategy;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use diesel::prelude::*;
use diesel::{Associations, Identifiable, Queryable, Selectable};

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq, Clone)]
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
        strategy: GroupingStrategy,
    ) -> Result<Arrangement, ErrorResponder> {
        let strategy_bytes = serde_json::to_vec(&strategy).map_err(|e| ErrorType::InternalError(e.to_string()).res())?;

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
    pub fn get_strategy(&self) -> Result<Option<GroupingStrategy>, ErrorResponder> {
        if let Some(strategy) = &self.strategy {
            return Ok(Some(
                serde_json::from_slice(strategy).map_err(|e| ErrorType::InternalError(e.to_string()).res())?,
            ));
        }
        Ok(None)
    }
    /// Updates the strategy of this arrangement
    pub fn set_strategy(&mut self, conn: &mut DBConn, strategy: GroupingStrategy) -> Result<(), ErrorResponder> {
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
    /// List all users’ non-manual arrangements, providing the deserialized strategy and the list of groups
    pub fn list_arrangements_and_groups(conn: &mut DBConn, user_id: u32) -> Result<Vec<ArrangementDetails>, ErrorResponder> {
        Self::list_arrangements(conn, user_id)?
            .into_iter()
            .filter(|arrangement| arrangement.strategy.is_some())
            .map(|arrangement| {
                let strategy = arrangement.get_strategy()?.unwrap();
                let groups = strategy.groupings.get_groups();
                let dependant_arrangements = strategy.get_dependant_arrangements();
                Ok(ArrangementDetails {
                    arrangement,
                    strategy,
                    dependant_arrangements,
                    groups,
                })
            })
            .collect()
    }
}
#[derive(Clone)]
pub struct ArrangementDetails {
    pub arrangement: Arrangement,
    pub strategy: GroupingStrategy,
    pub dependant_arrangements: Vec<u32>,
    pub groups: Vec<u32>,
}
