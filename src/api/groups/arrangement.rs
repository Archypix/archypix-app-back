use crate::api::groups::manual_groups::CreateManualGroupRequest;
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
pub struct CreateArrangementRequest {
    strong_match_conversion: bool,
    name: String,
    strategy: ArrangementStrategy,
}
#[derive(Serialize, JsonSchema)]
pub struct CreateArrangementResponse {
    id: u32,
    groups: Vec<Group>,
}

/// Create a new arrangement
#[openapi(tag = "Arrangement")]
#[post("/arrangement", data = "<request>")]
pub async fn create_arrangement(
    db: &State<DBPool>,
    user: User,
    request: Json<CreateArrangementRequest>,
) -> Result<Json<CreateArrangementResponse>, ErrorResponder> {
    let mut conn = &mut db.get().unwrap();

    err_transaction(&mut conn, |conn| {
        // TODO: Add the arrangement to the database

        // TODO: Check all pictures against this new arrangement and create groups

        Ok(Json(CreateArrangementResponse { id: 0, groups: vec![] }))
    })
}
