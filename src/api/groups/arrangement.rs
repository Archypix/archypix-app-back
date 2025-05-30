use crate::database::database::DBPool;
use crate::database::group::arrangement::Arrangement;
use crate::database::group::group::Group;
use crate::database::user::user::User;
use crate::grouping::arrangement_strategy::ArrangementStrategy;
use crate::grouping::grouping_process::{group_clear_pictures, group_pictures};
use crate::utils::errors_catcher::{err_transaction, ErrorResponder, ErrorType};
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::State;
use rocket_okapi::{openapi, JsonSchema};

#[derive(Deserialize, JsonSchema)]
pub struct ArrangementRequest {
    strong_match_conversion: bool,
    name: String,
    strategy: Option<ArrangementStrategy>,
}
#[derive(Serialize, JsonSchema)]
pub struct ArrangementResponse {
    arrangement: Arrangement,
    groups: Vec<Group>,
}

/// List all user’s arrangements
#[openapi(tag = "Arrangement")]
#[get("/arrangement")]
pub async fn list_arrangements(db: &State<DBPool>, user: User) -> Result<Json<Vec<Arrangement>>, ErrorResponder> {
    let conn = &mut db.get().unwrap();
    let arrangements = Arrangement::from_user_id(conn, user.id)?;
    Ok(Json(arrangements))
}

/// Create a new arrangement
#[openapi(tag = "Arrangement")]
#[post("/arrangement", data = "<data>")]
pub async fn create_arrangement(db: &State<DBPool>, user: User, data: Json<ArrangementRequest>) -> Result<Json<ArrangementResponse>, ErrorResponder> {
    let mut conn = &mut db.get().unwrap();

    err_transaction(&mut conn, |conn| {
        let arrangement = Arrangement::new(conn, user.id, data.name.clone(), data.strong_match_conversion, data.strategy.clone())?;

        // TODO: Check all pictures against this new arrangement and create groups

        Ok(Json(ArrangementResponse { arrangement, groups: vec![] }))
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
        // 1. Update the arrangement in the database
        Arrangement::update(conn, arrangement.id, &request.name, request.strong_match_conversion, &request.strategy)?;

        // 2. TODO: If strategy grouping has changed, we need to edit:
        //  - groups of the arrangement
        //  - hierarchies
        //  - arrangements depending on this one
        //  - shared group instances and link shared group instances

        // 3. Check all pictures against this edited arrangement
        if arrangement.strategy.is_some() {
            // Arrangement is/was not manual
            // For now we treat that like if pictures has all changed with full dependency type, but only for this arrangement.
            group_pictures(conn, user.id, None, Some(arrangement.id), None, true)?;
        }

        Ok(Json(ArrangementResponse { arrangement, groups: vec![] }))
    })
}

/// Delete an arrangement
#[openapi(tag = "Arrangement")]
#[delete("/arrangement/<arrangement_id>")]
pub async fn delete_arrangement(db: &State<DBPool>, user: User, arrangement_id: i32) -> Result<(), ErrorResponder> {
    let mut conn = &mut db.get().unwrap();
    let arrangement = Arrangement::from_id_and_user_id(conn, arrangement_id, user.id)?;

    err_transaction(&mut conn, |conn| {
        // TODO: Delete the arrangement in the database

        // 1. Remove pictures from all groups of this arrangement (should be done carefully to remove the pictures from other users if needed)
        let group_ids = if let Some(strategy) = arrangement.get_strategy()? {
            strategy.groupings.get_groups()
        } else {
            Group::from_arrangement(conn, arrangement.id)?.into_iter().map(|g| g.id).collect()
        };
        group_ids.iter().try_for_each(|group_id| group_clear_pictures(conn, *group_id))?;

        // 2. It is now safe to delete shared groups instances

        // 3. Delete link shared groups

        // 4. Edit hierarchies that depend on this arrangement

        // 5. Edit arrangements that depend on this arrangement

        // 6. Delete the arrangement itself
        Arrangement::delete(conn, arrangement.id)?;
        Ok(())
    })
}
