use crate::database::database::DBPool;
use crate::database::group::arrangement::Arrangement;
use crate::database::group::group::Group;
use crate::database::user::user::User;
use crate::utils::errors_catcher::{err_transaction, ErrorResponder, ErrorType};
use rocket::serde::{json::Json, Deserialize};
use rocket::State;
use rocket_okapi::{openapi, JsonSchema};

#[derive(Deserialize, JsonSchema)]
pub struct CreateManualGroupRequest {
    arrangement_id: u32,
    name: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct ModifyGroupPicturesRequest {
    group_id: u32,
    arrangement_id: u32,
    picture_ids: Vec<u64>,
}

/// Create a new manual group
#[openapi(tag = "Groups")]
#[post("/group/manual", data = "<request>")]
pub async fn create_manual_group(db: &State<DBPool>, user: User, request: Json<CreateManualGroupRequest>) -> Result<Json<Group>, ErrorResponder> {
    let mut conn = &mut db.get().unwrap();

    err_transaction(&mut conn, |conn| {
        // Verify the arrangement is manual and owned by the user
        let arrangement = Arrangement::from_id_and_user_id(conn, request.arrangement_id, user.id)?;
        if arrangement.strategy.is_some() {
            return Err(ErrorType::GroupIsNotManual.res_no_rollback());
        }

        let group = Group::insert(conn, request.arrangement_id, request.name.clone(), false)?;
        Ok(Json(group))
    })
}

/// Add pictures to a manual group
#[openapi(tag = "Groups")]
#[post("/group/manual/pictures", data = "<request>")]
pub async fn add_pictures_to_group(db: &State<DBPool>, user: User, request: Json<ModifyGroupPicturesRequest>) -> Result<(), ErrorResponder> {
    let mut conn = &mut db.get().unwrap();

    err_transaction(&mut conn, |conn| {
        // Verify the arrangement is manual and owned by the user
        let arrangement = Arrangement::from_id_and_user_id(conn, request.arrangement_id, user.id)?;
        if arrangement.strategy.is_some() {
            return Err(ErrorType::GroupIsNotManual.res_no_rollback());
        }
        // Get the group and verify it belongs to the arrangement
        Group::add_pictures(conn, request.group_id, &request.picture_ids)?;
        // TODO: Update the pictures on the accounts to which this group is shared.
        Ok(())
    })
}

/// Remove pictures from a manual group
#[openapi(tag = "Groups")]
#[delete("/group/manual/pictures", data = "<request>")]
pub async fn remove_pictures_from_group(db: &State<DBPool>, user: User, request: Json<ModifyGroupPicturesRequest>) -> Result<(), ErrorResponder> {
    let mut conn = &mut db.get().unwrap();

    err_transaction(&mut conn, |conn| {
        // Verify the arrangement is manual and owned by the user
        let arrangement = Arrangement::from_id_and_user_id(conn, request.arrangement_id, user.id)?;
        if arrangement.strategy.is_some() {
            return Err(ErrorType::GroupIsNotManual.res_no_rollback());
        }
        // Get the group and verify it belongs to the arrangement
        let group = Group::from_id_and_arrangement(conn, request.group_id, request.arrangement_id)?;
        group.remove_pictures(conn, &request.picture_ids)?;
        // TODO: Update the pictures on the accounts to which this group is shared.
        Ok(())
    })
}
