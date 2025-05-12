use crate::database::database::DBPool;
use crate::database::group::arrangement::Arrangement;
use crate::database::group::group::Group;
use crate::database::user::user::User;
use crate::grouping::arrangement_strategy::ArrangementStrategy;
use crate::utils::errors_catcher::{err_transaction, ErrorResponder, ErrorType};
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::State;
use rocket_okapi::{openapi, JsonSchema};

#[derive(Deserialize, JsonSchema)]
pub struct ArrangementRequest {
    strong_match_conversion: bool,
    name: String,
    strategy: ArrangementStrategy,
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
    arrangement_id: u32,
    request: Json<ArrangementRequest>,
) -> Result<Json<ArrangementResponse>, ErrorResponder> {
    let mut conn = &mut db.get().unwrap();
    let arrangement = Arrangement::from_id_and_user_id(conn, arrangement_id, user.id)?;

    err_transaction(&mut conn, |conn| {
        // TODO: Uddate the arrangement in the database

        // TODO: Check all pictures against this edited arrangement and update groups

        // TODO: Update arrangements that depends on this one

        Ok(Json(ArrangementResponse { arrangement, groups: vec![] }))
    })
}

/// Delete an arrangement
#[openapi(tag = "Arrangement")]
#[delete("/arrangement/<arrangement_id>")]
pub async fn delete_arrangement(db: &State<DBPool>, user: User, arrangement_id: u32) -> Result<(), ErrorResponder> {
    let mut conn = &mut db.get().unwrap();
    let arrangement = Arrangement::from_id_and_user_id(conn, arrangement_id, user.id)?;

    err_transaction(&mut conn, |conn| {
        // TODO: Delete the arrangement in the database

        // TODO: Delete all groups and pictures associated with this arrangement

        // TODO: Edit strategies of the arrangements that depend on this one
        Ok(())
    })
}
