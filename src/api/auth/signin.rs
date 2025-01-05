use crate::database::auth_token::{AuthToken, Confirmation, TOTPSecret};
use crate::database::database::{DBConn, DBPool};
use crate::database::schema::{ConfirmationAction, UserStatus};
use crate::database::user::User;
use crate::mailing::mailer::send_rendered_email;
use crate::utils::auth::DeviceInfo;
use crate::utils::errors_catcher::{err_transaction, ErrorResponder, ErrorType};
use crate::utils::utils::{get_frontend_host, left_pad};
use diesel::Connection;
use pwhash::bcrypt;
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket_okapi::{openapi, JsonSchema};
use std::env;

#[derive(JsonSchema, Deserialize, Debug)]
pub struct SigninData {
    email: String,
    password: String,
    totp_code: Option<String>,
    /// Optional redirect URL for the TFA confirmation (email confirmation)
    redirect_url: Option<String>
}

#[derive(JsonSchema, Serialize, Debug)]
pub struct SigninResponse {
    pub status: UserStatus,
    pub user_id: u32,
    pub name: String,
    pub email: String,
    pub auth_token: String,
}

#[derive(JsonSchema, Serialize, Debug)]
pub struct SigninEmailResponse {
    pub user_id: u32,
    pub code_token: String
}

/// Endpoint to sign in a user.
/// If the user requires 2FA, it will either throw `TFARequired`, `TFARequiredOverEmail` or `InvalidTOTPCode`.
#[openapi(tag = "Authentication")]
#[post("/auth/signin", data = "<data>")]
pub fn auth_signin(data: Json<SigninData>, db: &rocket::State<DBPool>, device_info: DeviceInfo) -> Result<Json<SigninResponse>, ErrorResponder> {
    let conn: &mut DBConn = &mut db.get().unwrap();

    err_transaction(conn, |conn| {
        let user = check_user_password_and_status(conn, &data.email, &data.password)?;

        if user.tfa_login {
            if let Some(totp_code) = &data.totp_code {
                if !TOTPSecret::check_user_totp(conn, &user.id, totp_code)? {
                    return ErrorType::InvalidTOTPCode.res_err();
                }
            } else {
                // 2FA Required, checking if TOTP is available
                if TOTPSecret::has_user_totp(conn, &user.id)? {
                    return ErrorType::TFARequired.res_err();
                }
                return ErrorType::TFARequiredOverEmail.res_err();
            }
        }

        let auth_token = AuthToken::insert_token_for_user(conn, &user.id, &device_info, 0)?;

        Ok(Json(SigninResponse {
            status: user.status,
            user_id: user.id,
            name: user.name,
            email: user.email,
            auth_token: hex::encode(auth_token),
        }))
    })
}


/// Login endpoint for users that require 2FA; sends a confirmation email.
#[openapi(tag = "Authentication")]
#[post("/auth/signin/email", data = "<data>")]
pub fn auth_signin_email(data: Json<SigninData>, db: &rocket::State<DBPool>, device_info: DeviceInfo) -> Result<Json<SigninEmailResponse>, ErrorResponder> {
    let conn: &mut DBConn = &mut db.get().unwrap();
    err_transaction(conn, |conn| {
        let user = check_user_password_and_status(conn, &data.email, &data.password)?;

        let (token, code_token, code) = Confirmation::insert_confirmation(conn, user.id, ConfirmationAction::Signin, &device_info, &data.redirect_url, 0)?;
        let code_str = left_pad(&code.to_string(), '0', 4);

        // Sending email
        let signin_url = format!("{}/signin?id={}&token={}", get_frontend_host(), user.id, hex::encode(&token));
        let subject = "Confirm your email address".to_string();
        let mut context = tera::Context::new();
        context.insert("name", &user.name);
        context.insert("url", &signin_url);
        context.insert("code", &code_str);
        context.insert("ip", &device_info.ip_address.unwrap_or("Unknown".to_string()));
        context.insert("agent", &device_info.device_string);
        send_rendered_email((user.name.clone(), data.email.clone()), subject, "confirm_signin".to_string(), context);

        Ok(Json(SigninEmailResponse {
            user_id: user.id,
            code_token: hex::encode(code_token),
        }))
    })
}

/// Checks the user's email and password, returning the user if the credentials are correct.
/// - Throw `InvalidEmailOrPassword` if the email or password is incorrect.
/// - Throw `UserBanned` if the user is banned.
/// - Throw `UserUnconfirmed` if the user is unconfirmed (account not email verified).
fn check_user_password_and_status(conn: &mut DBConn, email: &str, password: &str) -> Result<User, ErrorResponder> {
    let user = User::find_by_email_opt(conn, email)
        .and_then(|user| {
            if let Some(user) = user {
                if bcrypt::verify(password, &*user.password_hash) {
                    return Ok(user);
                }
            }
            ErrorType::InvalidEmailOrPassword.res_err()
        })?;

    match user.status {
        UserStatus::Banned => {
            ErrorType::UserBanned.res_err()
        }
        UserStatus::Unconfirmed => {
            ErrorType::UserUnconfirmed.res_err()
        }
        _ => {
            Ok(user)
        }
    }
}
