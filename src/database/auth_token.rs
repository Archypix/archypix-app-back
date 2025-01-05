use crate::database::database::DBConn;
use crate::database::schema::*;
use crate::database::utils::is_error_duplicate_key;
use crate::utils::auth::DeviceInfo;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use crate::utils::utils::{random_code, random_token};
use chrono::{Duration, NaiveDateTime, TimeDelta, Utc};
use diesel::{delete, QueryDsl, SelectableHelper};
use diesel::{insert_into, update, Identifiable, Insertable, Queryable, RunQueryDsl, Selectable};
use diesel::{ExpressionMethods, OptionalExtension};
use rocket::Request;
use totp_rs::{Rfc6238, TOTP};

#[derive(Queryable, Selectable, Identifiable, Insertable, Debug, PartialEq)]
#[diesel(primary_key(user_id, token))]
#[diesel(belongs_to(User))]
#[diesel(table_name = auth_tokens)]
pub struct AuthToken {
    pub user_id: u32,
    pub token: Vec<u8>,
    pub creation_date: NaiveDateTime,
    pub last_use_date: NaiveDateTime,
    pub device_string: Option<String>,
    pub ip_address: Option<Vec<u8>>,
}

impl AuthToken {
    pub(crate) fn insert_token_for_user(conn: &mut DBConn, user_id: &u32, device_info: &DeviceInfo, try_count: u8) -> Result<Vec<u8>, ErrorResponder> {
        let auth_token = random_token(32);

        insert_into(auth_tokens::table)
            .values((
                auth_tokens::dsl::user_id.eq(user_id),
                auth_tokens::dsl::token.eq(&auth_token),
                auth_tokens::dsl::device_string.eq(&device_info.device_string),
                auth_tokens::dsl::ip_address.eq(inet6_aton(&device_info.ip_address))
            ))
            .execute(conn)
            .map(|_| auth_token)
            .or_else(|e| {
                if is_error_duplicate_key(&e, "auth_tokens.PRIMARY") && try_count < 4 {
                    println!("Auth token already exists, trying again.");
                    return AuthToken::insert_token_for_user(conn, user_id, device_info, try_count + 1);
                }
                ErrorType::DatabaseError("Failed to insert auth token".to_string(), e).res_err_rollback()
            })
    }
    pub fn update_last_use_date(&self, conn: &mut DBConn) -> Result<(), ErrorResponder> {
        // Working in UTC time.
        let current_naive = Utc::now().naive_utc();
        if current_naive - self.last_use_date > TimeDelta::try_minutes(10).unwrap() {
            println!("Updating last_use_date");
            update(auth_tokens::table)
                .filter(auth_tokens::dsl::user_id.eq(self.user_id))
                .filter(auth_tokens::dsl::token.eq(self.token.clone()))
                .set((
                    auth_tokens::dsl::last_use_date.eq(utc_timestamp()),
                ))
                .execute(conn).map_err(|e| {
                ErrorType::DatabaseError("Failed to update auth token use date".to_string(), e).res()
            })?;
        }
        Ok(())
    }
    pub fn get_auth_token_from_headers(request: &Request<'_>) -> Option<Vec<u8>> {
        request.headers().get_one("X-Auth-Token").map(|s| hex::decode(s).ok()).flatten()
    }
    pub fn clear_auth_tokens(conn: &mut DBConn, user_id: &u32) -> Result<(), ErrorResponder> {
        delete(auth_tokens::table)
            .filter(auth_tokens::dsl::user_id.eq(user_id))
            .execute(conn)
            .map(|_| ())
            .map_err(|e| {
                ErrorType::DatabaseError("Failed to delete existing auth tokens".to_string(), e).res_rollback()
            })
    }
}


#[derive(Queryable, Selectable, Identifiable, Insertable, Debug, PartialEq)]
#[diesel(primary_key(user_id, token))]
#[diesel(belongs_to(User))]
#[diesel(table_name = confirmations)]
pub struct Confirmation {
    pub user_id: u32,
    pub action: ConfirmationAction,
    pub used: bool,
    pub date: NaiveDateTime,
    pub token: Vec<u8>,
    pub code_token: Vec<u8>,
    pub code: u16,
    pub code_trials: u8,
    pub redirect_url: Option<String>,
    pub device_string: Option<String>,
    pub ip_address: Option<Vec<u8>>,
}

impl Confirmation {
    pub(crate) fn insert_confirmation(conn: &mut DBConn, user_id: u32, action: ConfirmationAction, device_info: &DeviceInfo, redirect_url: &Option<String>, try_count: u8) -> Result<(Vec<u8>, Vec<u8>, u16), ErrorResponder> {
        let token = random_token(16);
        let code_token = random_token(16);
        let code = random_code(4) as u16;

        insert_into(confirmations::table)
            .values((
                confirmations::dsl::user_id.eq::<u32>(user_id),
                confirmations::dsl::action.eq(&action),
                confirmations::dsl::token.eq(&token),
                confirmations::dsl::code_token.eq(&code_token),
                confirmations::dsl::code.eq(&code),
                confirmations::dsl::redirect_url.eq(redirect_url),
                confirmations::dsl::device_string.eq(&device_info.device_string),
                confirmations::dsl::ip_address.eq(inet6_aton(&device_info.ip_address))
            ))
            .execute(conn)
            .map(|_| (token, code_token, code))
            .or_else(|e| {
                if (is_error_duplicate_key(&e, "confirmations.PRIMARY") || is_error_duplicate_key(&e, "confirmations.UQ_confirmations")) && try_count < 3 {
                    println!("Confirmation token already exists, trying again.");
                    return Confirmation::insert_confirmation(conn, user_id, action, device_info, redirect_url, try_count + 1);
                }
                ErrorType::DatabaseError("Failed to insert confirmation".to_string(), e).res_err_rollback()
            })
    }
    pub fn check_code_and_mark_as_used(conn: &mut DBConn, user_id: &u32, action: &ConfirmationAction, code_token: &Vec<u8>, code: &u16, max_minutes: i64) -> Result<Option<String>, ErrorResponder> {
        let mut confirmation = confirmations::table
            .filter(confirmations::dsl::user_id.eq(user_id))
            .filter(confirmations::dsl::action.eq(action))
            .filter(confirmations::dsl::code_token.eq(code_token))
            .first::<Confirmation>(conn)
            .optional()
            .map_err(|e| {
                ErrorType::DatabaseError("Failed to get confirmation".to_string(), e).res_rollback()
            })?;
        if let Some(mut confirmation) = confirmation {
            if confirmation.used {
                return ErrorType::ConfirmationAlreadyUsed.res_err();
            }
            if confirmation.date < Utc::now().naive_utc() - Duration::minutes(max_minutes) {
                return ErrorType::ConfirmationExpired.res_err();
            }
            if confirmation.code_trials >= 3 {
                return ErrorType::ConfirmationTooManyAttempts.res_err();
            }
            if confirmation.code != *code {
                confirmation.code_trials += 1;
                update(confirmations::table)
                    .filter(confirmations::dsl::user_id.eq(user_id))
                    .filter(confirmations::dsl::action.eq(action))
                    .filter(confirmations::dsl::code_token.eq(code_token))
                    .set((
                        confirmations::dsl::code_trials.eq(confirmation.code_trials),
                    ))
                    .execute(conn)
                    .map_err(|e| {
                        ErrorType::DatabaseError("Failed to update confirmation code trials".to_string(), e).res_rollback()
                    })?;
                return ErrorType::ConfirmationNotFound.res_err();
            }

            confirmation.mark_as_used(conn)?;
            return Ok(confirmation.redirect_url);
        }
        ErrorType::ConfirmationNotFound.res_err()
    }
    pub fn check_token_and_mark_as_used(conn: &mut DBConn, user_id: &u32, action: &ConfirmationAction, token: &Vec<u8>, max_minutes: i64) -> Result<Option<String>, ErrorResponder> {
        let mut confirmation = confirmations::table
            .filter(confirmations::dsl::user_id.eq(user_id))
            .filter(confirmations::dsl::action.eq(action))
            .filter(confirmations::dsl::token.eq(token))
            .first::<Confirmation>(conn)
            .optional()
            .map_err(|e| {
                ErrorType::DatabaseError("Failed to get confirmation".to_string(), e).res_rollback()
            })?;
        if let Some(mut confirmation) = confirmation {
            if confirmation.used {
                return ErrorType::ConfirmationAlreadyUsed.res_err();
            }
            if confirmation.date < Utc::now().naive_utc() - Duration::minutes(max_minutes) {
                return ErrorType::ConfirmationExpired.res_err();
            }
            confirmation.mark_as_used(conn)?;
            return Ok(confirmation.redirect_url);
        }
        ErrorType::ConfirmationNotFound.res_err()
    }
    pub fn mark_as_used(&self, conn: &mut DBConn) -> Result<(), ErrorResponder> {
        update(confirmations::table)
            .filter(confirmations::dsl::user_id.eq(&self.user_id))
            .filter(confirmations::dsl::action.eq(&self.action))
            .filter(confirmations::dsl::token.eq(&self.token))
            .set((
                confirmations::dsl::used.eq(true),
            ))
            .execute(conn)
            .map(|_| ())
            .map_err(|e| {
                ErrorType::DatabaseError("Failed to mark confirmation as used".to_string(), e).res_rollback()
            })
    }
    pub fn mark_all_as_used(conn: &mut DBConn, user_id: &u32, action: ConfirmationAction) -> Result<(), ErrorResponder> {
        update(confirmations::table)
            .filter(confirmations::dsl::user_id.eq(user_id))
            .filter(confirmations::dsl::action.eq(action))
            .set((
                confirmations::dsl::used.eq(true),
            ))
            .execute(conn)
            .map(|_| ())
            .map_err(|e| {
                ErrorType::DatabaseError("Failed to mark all confirmations as used".to_string(), e).res_rollback()
            })
    }
}

#[derive(Queryable, Selectable, Identifiable, Insertable, Debug, PartialEq)]
#[diesel(primary_key(user_id))]
#[diesel(belongs_to(User))]
#[diesel(table_name = totp_secrets)]
pub struct TOTPSecret {
    pub user_id: u32,
    pub creation_date: NaiveDateTime,
    pub secret: Vec<u8>,
}

impl TOTPSecret {
    pub fn insert_secret_for_user(conn: &mut DBConn, user_id: &u32, secret: &Vec<u8>) -> Result<(), ErrorResponder> {
        insert_into(totp_secrets::table)
            .values((
                totp_secrets::dsl::user_id.eq(user_id),
                totp_secrets::dsl::secret.eq(secret),
            ))
            .execute(conn)
            .map(|_| ())
            .map_err(|e| {
                ErrorType::DatabaseError("Failed to insert TOTP secret".to_string(), e).res_rollback()
            })
    }
    pub fn has_user_totp(conn: &mut DBConn, user_id: &u32) -> Result<bool, ErrorResponder> {
        totp_secrets::table
            .filter(totp_secrets::dsl::user_id.eq(user_id))
            .select(totp_secrets::dsl::user_id)
            .first::<u32>(conn)
            .optional()
            .map(|opt| opt.is_some())
            .map_err(|e| {
                ErrorType::DatabaseError("Failed to check if user has TOTP".to_string(), e).res_rollback()
            })
    }
    pub fn get_user_totp_secrets(conn: &mut DBConn, user_id: &u32) -> Result<Vec<TOTPSecret>, ErrorResponder> {
        totp_secrets::table
            .filter(totp_secrets::dsl::user_id.eq(user_id))
            .select(TOTPSecret::as_select())
            .load::<TOTPSecret>(conn)
            .map_err(|e| {
                ErrorType::DatabaseError("Failed to get user TOTP secrets".to_string(), e).res_rollback()
            })
    }
    pub fn check_user_totp(conn: &mut DBConn, user_id: &u32, code: &str) -> Result<bool, ErrorResponder> {
        let secrets = TOTPSecret::get_user_totp_secrets(conn, user_id)?;
        for secret in secrets {
            if secret.to_totp()?.check_current(code).map_err(|_| {
                ErrorType::InternalError("SystemTimeError occurred when checking TOTP.".to_string()).res()
            })? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn to_totp(&self) -> Result<TOTP, ErrorResponder> {
        let rf6238 = Rfc6238::new(6, self.secret.clone(), Some("Archypix".to_string()), "clementgre@archypix.com".to_string())
            .map_err(|_| ErrorType::InternalError("Unable to create Rfc6238 (for TOTP)".to_string()).res())?;
        TOTP::from_rfc6238(rf6238).map_err(|_| ErrorType::InternalError("Unable to create TOTP".to_string()).res())
    }
}
