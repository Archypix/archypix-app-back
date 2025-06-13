use crate::database::database::DBPool;
use crate::database::group::arrangement::Arrangement;
use crate::database::group::group::Group;
use crate::database::group::link_share_group::LinkShareGroups;
use crate::database::group::shared_group::SharedGroup;
use crate::database::hierarchy::hierarchy_arrangement::HierarchyArrangements;
use crate::database::user::user::User;
use crate::grouping::arrangement_strategy::{ArrangementStrategy, ArrangementStrategyRequest};
use crate::grouping::grouping_process::{group_clear_pictures, group_pictures};
use crate::utils::errors_catcher::{err_transaction, ErrorResponder, ErrorType};
use diesel_derives::{Associations, Identifiable, Queryable, Selectable};
use itertools::Itertools;
use rocket::form::validate::Contains;
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::State;
use rocket_okapi::{openapi, JsonSchema};

#[derive(Deserialize, JsonSchema)]
pub struct ArrangementRequest {
    strong_match_conversion: bool,
    name: String,
    strategy: Option<ArrangementStrategyRequest>,
}
#[derive(Serialize, JsonSchema)]
pub struct ArrangementResponse {
    arrangement: ArrangementResponseArrangement,
    groups: Vec<Group>,
    to_be_deleted_groups: Vec<Group>,
}

#[derive(Debug, PartialEq, Clone, JsonSchema, Serialize)]
pub struct ArrangementResponseArrangement {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub strong_match_conversion: bool,
    pub strategy: Option<ArrangementStrategy>,
}
impl TryFrom<Arrangement> for ArrangementResponseArrangement {
    type Error = ErrorResponder;
    fn try_from(arrangement: Arrangement) -> Result<Self, Self::Error> {
        Ok(ArrangementResponseArrangement {
            id: arrangement.id,
            user_id: arrangement.user_id,
            strategy: arrangement.get_strategy()?,
            name: arrangement.name,
            strong_match_conversion: arrangement.strong_match_conversion,
        })
    }
}

/// List all user’s arrangements
#[openapi(tag = "Arrangement")]
#[get("/arrangement")]
pub async fn list_arrangements(db: &State<DBPool>, user: User) -> Result<Json<Vec<ArrangementResponse>>, ErrorResponder> {
    let conn = &mut db.get().unwrap();
    let arrangements_with_groups = Arrangement::from_user_id_with_groups(conn, user.id)?;

    let arrangements = arrangements_with_groups
        .into_iter()
        .map(|(arrangement, groups)| {
            Ok(ArrangementResponse {
                arrangement: ArrangementResponseArrangement::try_from(arrangement)?,
                groups: groups.iter().filter(|g| !g.to_be_deleted).cloned().collect_vec(),
                to_be_deleted_groups: groups.into_iter().filter(|g| g.to_be_deleted).collect_vec(),
            })
        })
        .collect::<Result<Vec<_>, ErrorResponder>>()?;

    Ok(Json(arrangements))
}

/// Create a new arrangement
#[openapi(tag = "Arrangement")]
#[post("/arrangement", data = "<data>")]
pub async fn create_arrangement(db: &State<DBPool>, user: User, data: Json<ArrangementRequest>) -> Result<Json<ArrangementResponse>, ErrorResponder> {
    let mut conn = &mut db.get().unwrap();

    err_transaction(&mut conn, |conn| {
        // Create the arrangement and persist it in the database
        let mut arrangement = Arrangement::new(conn, user.id, data.name.clone(), data.strong_match_conversion, None)?;

        // Create strategy (will eventually create groups)
        let strategy = match &data.strategy {
            Some(strategy_req) => Some(strategy_req.create(conn, arrangement.id)?),
            None => None,
        };

        if strategy.is_some() {
            // Save strategy in the arrangement
            arrangement.set_strategy(conn, strategy.clone())?;
            // Group all pictures according to the strategy
            group_pictures(conn, user.id, None, Some(arrangement.id), None, false)?;
        }

        Ok(Json(ArrangementResponse {
            groups: Group::from_arrangement(conn, arrangement.id, false)?,
            arrangement: ArrangementResponseArrangement {
                id: arrangement.id,
                user_id: arrangement.user_id,
                name: arrangement.name,
                strong_match_conversion: arrangement.strong_match_conversion,
                strategy,
            },
            to_be_deleted_groups: vec![],
        }))
    })
}

/// Edit an arrangement
#[openapi(tag = "Arrangement")]
#[patch("/arrangement/<arrangement_id>", data = "<request>")]
pub async fn edit_arrangement(
    db: &State<DBPool>,
    user: User,
    arrangement_id: i32,
    request: Json<ArrangementRequest>,
) -> Result<Json<ArrangementResponse>, ErrorResponder> {
    let mut conn = &mut db.get().unwrap();
    let arrangement = Arrangement::from_id_and_user_id(conn, arrangement_id, user.id)?;

    err_transaction(&mut conn, |conn| {
        // 1. Update the groups of the arrangement due to the strategy change (marks old groups as "to be deleted", and create the required new ones).
        let new_strategy = match (&arrangement.get_strategy()?, &request.strategy) {
            (Some(old_strategy), Some(new_strategy_req)) => Some(new_strategy_req.edit(conn, arrangement.id, old_strategy)?),
            (None, Some(new_strategy)) => {
                Group::mark_all_as_to_be_deleted(conn, arrangement.id)?;
                Some(new_strategy.create(conn, arrangement.id)?)
            }
            // When switching to manual arrangement. No need to mark old groups as "to be deleted", they will stay as the new manual groups.
            (Some(_), None) | (None, None) => None,
        };

        // 2. Update the arrangement in the database
        let arrangement = Arrangement::update(conn, arrangement.id, &request.name, request.strong_match_conversion, &new_strategy)?;

        // 4. Check all pictures against this edited arrangement
        if new_strategy.is_some() {
            // Arrangement is not manual -> act like if the arrangement was just created
            group_pictures(conn, user.id, None, Some(arrangement.id), None, false)?;
        }

        let groups = Group::from_arrangement_all(conn, arrangement.id)?;
        let not_to_be_deleted_groups = groups.iter().filter(|g| !g.to_be_deleted).cloned().collect_vec();
        let to_be_deleted_groups = groups.iter().filter(|g| g.to_be_deleted).cloned().collect_vec();

        Ok(Json(ArrangementResponse {
            arrangement: ArrangementResponseArrangement {
                id: arrangement.id,
                user_id: arrangement.user_id,
                name: arrangement.name,
                strong_match_conversion: arrangement.strong_match_conversion,
                strategy: new_strategy,
            },
            groups: not_to_be_deleted_groups,
            to_be_deleted_groups,
        }))
    })
}

/// Delete an arrangement
/// The arrangement must not appear in any hierarchy, and no arrangement can depend on it.
#[openapi(tag = "Arrangement")]
#[delete("/arrangement/<arrangement_id>")]
pub async fn delete_arrangement(db: &State<DBPool>, user: User, arrangement_id: i32) -> Result<(), ErrorResponder> {
    let mut conn = &mut db.get().unwrap();
    let arrangement = Arrangement::from_id_and_user_id(conn, arrangement_id, user.id)?;

    // 1. Check that no hierarchy depends on this arrangement
    let nb_hierarchy_arrangement = HierarchyArrangements::from_arrangement_id(conn, arrangement.id)?.len();
    if nb_hierarchy_arrangement > 0 {
        return Err(ErrorType::UnprocessableEntity("Can’t delete this arrangement because it is used in a hierarchy".to_string()).res());
    }

    // 2. Check that no arrangement depends on this one
    Arrangement::list_arrangements_and_groups(conn, user.id)?
        .into_iter()
        .filter(|a| a.arrangement.id != arrangement.id)
        .try_for_each(|a| {
            if a.dependant_arrangements.contains(&arrangement.id) {
                Err(ErrorType::UnprocessableEntity("Can’t delete this arrangement because another arrangement depends on it".to_string()).res())
            } else {
                Ok(())
            }
        })?;

    // 3. Remove pictures from groups of the arrangement (should be done carefully to remove the pictures from other users if needed)
    let group_ids = Group::from_arrangement_all(conn, arrangement.id)?.into_iter().map(|g| g.id).collect_vec();
    group_ids.iter().try_for_each(|group_id| group_clear_pictures(conn, *group_id))?;

    err_transaction(&mut conn, |conn| {
        // 4. Delete the shared groups, link share groups, groups, and the arrangement itself
        SharedGroup::delete_by_group_ids(conn, &group_ids)?;
        LinkShareGroups::delete_by_group_ids(conn, &group_ids)?;
        Group::delete_by_arrangement_id(conn, arrangement.id)?;
        Arrangement::delete(conn, arrangement.id)?;
        Ok(())
    })
}
