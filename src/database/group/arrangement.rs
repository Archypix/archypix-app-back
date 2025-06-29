use crate::database::database::DBConn;
use crate::database::group::group::Group;
use crate::database::schema::*;
use crate::database::user::user::User;
use crate::grouping::arrangement_strategy::ArrangementStrategy;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use diesel::prelude::*;
use diesel::r2d2::PooledConnection;
use diesel::{Associations, Identifiable, Queryable, Selectable};
use itertools::Itertools;
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq, Clone, JsonSchema, Serialize)]
#[diesel(primary_key(id))]
#[diesel(belongs_to(User))]
#[diesel(table_name = arrangements)]
pub struct Arrangement {
    pub id: i32,
    pub user_id: i32,
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
        user_id: i32,
        name: String,
        strong_match_conversion: bool,
        strategy: Option<ArrangementStrategy>,
    ) -> Result<Arrangement, ErrorResponder> {
        let strategy_bytes = serde_json::to_vec(&strategy).map_err(|e| ErrorType::InternalError(e.to_string()).res_no_rollback())?;
        let dependency_type = ArrangementDependencyType::from(&strategy);

        diesel::insert_into(arrangements::table)
            .values((
                arrangements::user_id.eq(user_id),
                arrangements::name.eq(&name),
                arrangements::strategy.eq(Some(strategy_bytes)),
                arrangements::strong_match_conversion.eq(strong_match_conversion),
                arrangements::groups_dependant.eq(dependency_type.groups_dependant),
                arrangements::tags_dependant.eq(dependency_type.tags_dependant),
                arrangements::exif_dependant.eq(dependency_type.exif_dependant),
            ))
            .get_result(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }

    pub fn update(
        conn: &mut DBConn,
        id: i32,
        name: &String,
        strong_match_conversion: bool,
        strategy: &Option<ArrangementStrategy>,
    ) -> Result<Arrangement, ErrorResponder> {
        let dependency_type = ArrangementDependencyType::from(strategy);

        diesel::update(arrangements::table.filter(arrangements::id.eq(id)))
            .set((
                arrangements::name.eq(name),
                arrangements::strategy.eq(Self::strategy_to_binary(strategy)?),
                arrangements::strong_match_conversion.eq(&strong_match_conversion),
                arrangements::groups_dependant.eq(dependency_type.groups_dependant),
                arrangements::tags_dependant.eq(dependency_type.tags_dependant),
                arrangements::exif_dependant.eq(dependency_type.exif_dependant),
            ))
            .returning(Arrangement::as_returning())
            .get_result(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }

    pub fn from_user_id(conn: &mut DBConn, user_id: i32) -> Result<Vec<Arrangement>, ErrorResponder> {
        arrangements::table
            .filter(arrangements::user_id.eq(user_id))
            .load(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
    pub fn from_user_id_with_groups(conn: &mut DBConn, user_id: i32) -> Result<Vec<(Arrangement, Vec<Group>)>, ErrorResponder> {
        let arrangements = Self::from_user_id(conn, user_id)?;
        let groups = Group::from_user_id_all(conn, user_id)?;

        Ok(arrangements
            .into_iter()
            .map(|arrangement| {
                let arrangement_groups = groups.iter().filter(|group| group.arrangement_id == arrangement.id).cloned().collect();
                (arrangement, arrangement_groups)
            })
            .collect_vec())
    }
    pub fn from_id_and_user_id(conn: &mut DBConn, arrangement_id: i32, user_id: i32) -> Result<Arrangement, ErrorResponder> {
        Self::from_id_and_user_id_opt(conn, arrangement_id, user_id)?.ok_or_else(|| ErrorType::ArrangementNotFound.res())
    }
    pub fn from_id_and_user_id_opt(conn: &mut DBConn, arrangement_id: i32, user_id: i32) -> Result<Option<Arrangement>, ErrorResponder> {
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
    pub fn set_strategy(&mut self, conn: &mut DBConn, strategy: Option<ArrangementStrategy>) -> Result<(), ErrorResponder> {
        self.strategy = Self::strategy_to_binary(&strategy)?;

        diesel::update(arrangements::table.filter(arrangements::id.eq(self.id)))
            .set(arrangements::strategy.eq(&self.strategy))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())?;
        Ok(())
    }
    pub fn strategy_to_binary(strategy: &Option<ArrangementStrategy>) -> Result<Option<Vec<u8>>, ErrorResponder> {
        if let Some(strategy) = strategy {
            return Ok(Some(
                serde_json::to_vec(strategy).map_err(|e| ErrorType::InternalError(e.to_string()).res())?,
            ));
        }
        Ok(None)
    }

    /// List all user’s arrangements
    pub fn list_arrangements(conn: &mut DBConn, user_id: i32) -> Result<Vec<Arrangement>, ErrorResponder> {
        arrangements::table
            .filter(arrangements::user_id.eq(user_id))
            .load(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())
    }
    /// List all users’ non-manual arrangements, providing the deserialized strategy, the list of groups and the list of dependant arrangements
    pub fn list_arrangements_and_groups(conn: &mut DBConn, user_id: i32) -> Result<Vec<ArrangementDetails>, ErrorResponder> {
        let mut arrangements = Self::list_arrangements(conn, user_id)?
            .into_iter()
            .filter(|arrangement| arrangement.strategy.is_some())
            .map(|arrangement| {
                let strategy = arrangement.get_strategy()?.unwrap();
                let dependant_groups = strategy.get_dependant_groups();
                Ok::<ArrangementDetails, ErrorResponder>(ArrangementDetails {
                    arrangement,
                    strategy,
                    dependant_groups,
                    dependant_arrangements: vec![],
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
    pub fn get_arrangements_from_groups_ids(conn: &mut DBConn, groups_ids: Vec<i32>) -> Result<Vec<Arrangement>, ErrorResponder> {
        Ok(arrangements::table
            .inner_join(groups::table.on(groups::arrangement_id.eq(arrangements::id)))
            .filter(groups::id.eq_any(groups_ids))
            .select(arrangements::all_columns)
            .load::<Arrangement>(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())?)
    }

    /// Delete the arrangement with the given id, without taking care of the dependencies (hierarchies, shared groups, strategies...)
    pub fn delete(conn: &mut DBConn, arrangement_id: i32) -> Result<(), ErrorResponder> {
        diesel::delete(arrangements::table.filter(arrangements::id.eq(arrangement_id)))
            .execute(conn)
            .map_err(|e| ErrorType::DatabaseError(e.to_string(), e).res())?;
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct ArrangementDetails {
    pub arrangement: Arrangement,
    pub strategy: ArrangementStrategy,
    pub dependant_groups: Vec<i32>, // Ids of the groups on which this arrangement’s strategy depends (directly determinateed from the arrangement strategy)
    pub dependant_arrangements: Vec<i32>, // Ids of the arrangements on which this arrangement depends (got with set_dependant_arrangements_auto fetching the groups’s arrangements)
}
impl ArrangementDetails {
    pub fn set_dependant_arrangements_auto(&mut self, all_arrangements_details: &Vec<ArrangementDetails>) {
        self.dependant_arrangements = all_arrangements_details
            .iter()
            .filter(|arr| arr.strategy.groupings.get_groups().iter().any(|g| self.dependant_groups.contains(g)))
            .map(|arr| arr.arrangement.id)
            .clone()
            .collect();
    }
}

impl PartialEq for ArrangementDetails {
    fn eq(&self, other: &Self) -> bool {
        self.arrangement.id == other.arrangement.id
    }
    fn ne(&self, other: &Self) -> bool {
        self.arrangement.id != other.arrangement.id
    }
}

#[derive(Clone, Debug)]
pub struct ArrangementDependencyType {
    pub groups_dependant: bool,
    pub tags_dependant: bool,
    pub exif_dependant: bool,
}

impl ArrangementDependencyType {
    pub fn new_groups_dependant() -> Self {
        Self {
            groups_dependant: true,
            tags_dependant: false,
            exif_dependant: false,
        }
    }
    pub fn new_tags_dependant() -> Self {
        Self {
            groups_dependant: false,
            tags_dependant: true,
            exif_dependant: false,
        }
    }
    pub fn new_exif_dependant() -> Self {
        Self {
            groups_dependant: false,
            tags_dependant: false,
            exif_dependant: true,
        }
    }
    pub fn new_none() -> Self {
        Self {
            groups_dependant: false,
            tags_dependant: false,
            exif_dependant: false,
        }
    }
    /// Returns true if at least one of the dependencies of this type matches one of the provided.
    pub fn match_any(&self, other: &Self) -> bool {
        (self.groups_dependant && other.groups_dependant)
            || (self.tags_dependant && other.tags_dependant)
            || (self.exif_dependant && other.exif_dependant)
    }
}

impl From<&Option<ArrangementStrategy>> for ArrangementDependencyType {
    fn from(strategy: &Option<ArrangementStrategy>) -> Self {
        if let Some(strategy) = strategy {
            ArrangementDependencyType {
                groups_dependant: strategy.is_groups_dependant(),
                tags_dependant: strategy.is_tags_dependant(),
                exif_dependant: strategy.is_exif_dependant(),
            }
        } else {
            Self::new_none()
        }
    }
}
impl From<&Arrangement> for ArrangementDependencyType {
    fn from(a: &Arrangement) -> Self {
        ArrangementDependencyType {
            groups_dependant: a.groups_dependant,
            tags_dependant: a.tags_dependant,
            exif_dependant: a.exif_dependant,
        }
    }
}

impl From<&ArrangementDetails> for ArrangementDependencyType {
    fn from(ad: &ArrangementDetails) -> Self {
        ArrangementDependencyType {
            groups_dependant: ad.arrangement.groups_dependant,
            tags_dependant: ad.arrangement.tags_dependant,
            exif_dependant: ad.arrangement.exif_dependant,
        }
    }
}
