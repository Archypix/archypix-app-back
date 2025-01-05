use crate::database::schema::UserStatus;
use crate::database::user::User;
use crate::utils::errors_catcher::ErrorResponder;
use rocket::serde::json::Json;
use rocket::serde::Serialize;
use rocket_okapi::{openapi, JsonSchema};

#[derive(JsonSchema, Serialize, Debug)]
pub struct StatusResponse {
    pub(crate) name: String,
    pub(crate) email: String,
    pub(crate) status: UserStatus,
}

/// Get the account information of the authenticated user.
/// If the credentials are invalid or match an unconfirmed or banned user, it returns an error from
/// the User Request Guard.
#[openapi(tag = "Authentication")]
#[get("/auth/status")]
pub fn auth_status(user: User) -> Result<Json<StatusResponse>, ErrorResponder> {
    Ok(Json(StatusResponse {
        name: user.name,
        email: user.email,
        status: user.status,
    }))
}
