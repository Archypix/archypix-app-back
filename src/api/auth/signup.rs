use diesel::Connection;
use rocket::serde::{json::Json, Deserialize};
use rocket_okapi::{openapi, JsonSchema};
use serde::Serialize;
use std::env;
use validator::Validate;

use crate::database::auth_token::Confirmation;
use crate::database::database::DBPool;
use crate::database::schema::ConfirmationAction;
use crate::database::user::User;
use crate::mailing::mailer::send_rendered_email;
use crate::utils::auth::DeviceInfo;
use crate::utils::errors_catcher::{err_transaction, ErrorResponder, ErrorType};
use crate::utils::utils::{get_frontend_host, left_pad};
use crate::utils::validation::validate_input;
use crate::utils::validation::validate_password;
use crate::utils::validation::validate_user_name;

#[derive(JsonSchema, Deserialize, Debug, Validate)]
pub struct SignupData {
    #[validate(custom(function = validate_user_name))]
    name: String,
    #[validate(email(code = "email_invalid", message = "Invalid email"))]
    email: String,
    #[validate(custom(function = validate_password))]
    password: String,
    /// Optional redirect URL for the email confirmation
    redirect_url: Option<String>,
}

#[derive(JsonSchema, Serialize, Debug)]
pub struct SignupResponse {
    pub(crate) user_id: u32,
    pub(crate) code_token: String,
}

/// Endpoint to register a new user account.
/// A confirmation entry will be added to the database, and an email will be sent to the user.
#[openapi(tag = "Authentication")]
#[post("/auth/signup", data = "<data>")]
pub fn auth_signup(data: Json<SignupData>, db: &rocket::State<DBPool>, device_info: DeviceInfo) -> Result<Json<SignupResponse>, ErrorResponder> {
    validate_input(&data)?;
    let conn = &mut db.get().unwrap();

    err_transaction(conn, |conn| {
        // Inserting user
        let uid = User::create_user(conn, &data.name, &data.email, &data.password)?;

        // Inserting confirmation
        let (confirm_token, confirm_code_token, confirm_code) = Confirmation::insert_confirmation(conn, uid, ConfirmationAction::Signup, &device_info, &data.redirect_url, 0)?;
        let confirm_code_str = left_pad(&confirm_code.to_string(), '0', 4);

        // Sending email
        let signup_url = format!("{}/signup?id={}&token={}", get_frontend_host(), uid, hex::encode(&confirm_token));
        let subject = "Confirm your email address".to_string();
        let mut context = tera::Context::new();
        context.insert("name", &data.name);
        context.insert("url", &signup_url);
        context.insert("code", &confirm_code_str);
        context.insert("ip", &device_info.ip_address.unwrap_or("Unknown".to_string()));
        context.insert("agent", &device_info.device_string);
        send_rendered_email((data.name.clone(), data.email.clone()), subject, "confirm_signup".to_string(), context);

        Ok(Json(SignupResponse {
            user_id: uid,
            code_token: hex::encode(confirm_code_token),
        }))
    })
}
