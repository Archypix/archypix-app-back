use crate::api::auth::signin::SigninResponse;
use crate::database::auth_token::{AuthToken, Confirmation};
use crate::database::database::{DBConn, DBPool};
use crate::database::schema::ConfirmationAction;
use crate::database::schema::UserStatus;
use crate::database::user::User;
use crate::utils::auth::{DeviceInfo, UserAuthInfo};
use crate::utils::errors_catcher::{err_transaction, ErrorResponder, ErrorType};
use crate::utils::utils::get_frontend_host;
use crate::utils::validation::validate_input;
use diesel::Connection;
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket_okapi::{openapi, JsonSchema};
use std::env;
use validator::Validate;

#[derive(JsonSchema, Deserialize, Debug, Validate)]
pub struct ConfirmCodeData {
    action: ConfirmationAction,
    /// token sent to the browser when the action was initiated
    code_token: String,
    /// 4-digit code emailed to the user
    #[validate(range(min = 0, max = 9999, message = "Code must be a 4 digit number"))]
    code: u16,
}

#[derive(JsonSchema, Deserialize, Debug, Validate)]
pub struct ConfirmTokenData {
    action: ConfirmationAction,
    /// Emailed token
    token: String,
}

#[derive(JsonSchema, Serialize, Debug)]
pub struct ConfirmSignInUpResponse {
    pub status: UserStatus,
    pub user_id: u32,
    pub auth_token: String,
    pub name: String,
    pub email: String,
    pub redirect_url: String,
}

#[derive(JsonSchema, Serialize, Debug)]
#[serde(untagged)]
pub enum ConfirmResponse {
    SignInUp(ConfirmSignInUpResponse),
}

/// Confirm any 2FA request with a code_token and a code (from email code).
#[openapi(tag = "Authentication")]
#[post("/auth/confirm/code", data = "<data>")]
pub fn auth_confirm_code(data: Json<ConfirmCodeData>, db: &rocket::State<DBPool>, user_auth_info: UserAuthInfo, device_info: DeviceInfo) -> Result<Json<ConfirmResponse>, ErrorResponder> {
    validate_input(&data)?;
    let conn: &mut DBConn = &mut db.get().unwrap();
    let user_id = user_auth_info.user_id.ok_or(ErrorType::UserNotFound.res())?;
    let user = User::from_id(conn, &user_id)?;

    let code_token = hex::decode(&data.code_token).map_err(|_| ErrorType::UnprocessableEntity.res())?;

    err_transaction(conn, |conn| {
        let redirect_url = Confirmation::check_code_and_mark_as_used(conn, &user_id, &data.action, &code_token, &data.code, 15)?
            .unwrap_or(get_frontend_host());
        confirm_execute(conn, &data.action, user, redirect_url, &device_info)
    })
}

/// Confirm any 2FA request with a token (from email link).
#[openapi(tag = "Authentication")]
#[post("/auth/confirm/token", data = "<data>")]
pub fn auth_confirm_token(data: Json<ConfirmTokenData>, db: &rocket::State<DBPool>, user_auth_info: UserAuthInfo, device_info: DeviceInfo) -> Result<Json<ConfirmResponse>, ErrorResponder> {
    validate_input(&data)?;
    let conn: &mut DBConn = &mut db.get().unwrap();
    let user_id = user_auth_info.user_id.ok_or(ErrorType::UserNotFound.res())?;
    let user = User::from_id(conn, &user_id)?;

    let token = hex::decode(&data.token).map_err(|_| ErrorType::UnprocessableEntity.res())?;

    err_transaction(conn, |conn| {
        let redirect_url = Confirmation::check_token_and_mark_as_used(conn, &user_id, &data.action, &token, 15)?
            .unwrap_or(get_frontend_host());
        confirm_execute(conn, &data.action, user, redirect_url, &device_info)
    })
}

/// Execute the confirmation action and return the response.
/// This function is called after the confirmation code or token is validated.
fn confirm_execute(conn: &mut DBConn, action: &ConfirmationAction, user: User, redirect_url: String, device_info: &DeviceInfo) -> Result<Json<ConfirmResponse>, ErrorResponder> {
    match action {
        ConfirmationAction::Signup => {
            user.switch_status(conn, &UserStatus::Normal)?;
            let auth_token = AuthToken::insert_token_for_user(conn, &user.id, device_info, 0)?;
            Ok(Json(ConfirmResponse::SignInUp(ConfirmSignInUpResponse {
                status: user.status,
                name: user.name,
                email: user.email,
                user_id: user.id,
                auth_token: hex::encode(auth_token),
                redirect_url,
            })))
        }
        ConfirmationAction::Signin => {
            let auth_token = AuthToken::insert_token_for_user(conn, &user.id, &device_info, 0)?;

            Ok(Json(ConfirmResponse::SignInUp(ConfirmSignInUpResponse {
                status: user.status,
                name: user.name,
                email: user.email,
                user_id: user.id,
                auth_token: hex::encode(auth_token),
                redirect_url,
            })))
        }
        _ => {
            ErrorType::BadRequest.res_err()
        }
    }
}
