use crate::database::database::DBConn;
use crate::database::schema::*;
use crate::database::utils::is_error_duplicate_key;
use crate::utils::auth::DeviceInfo;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use crate::utils::utils::{random_code, random_token};
use chrono::{Duration, NaiveDateTime, Utc};
use diesel::QueryDsl;
use diesel::{insert_into, update, Identifiable, Insertable, Queryable, RunQueryDsl, Selectable};
use diesel::{ExpressionMethods, OptionalExtension};
use ipnet::IpNet;

#[derive(Queryable, Selectable, Identifiable, Insertable, Debug, PartialEq)]
#[diesel(primary_key(user_id, token))]
#[diesel(belongs_to(User))]
#[diesel(table_name = confirmations)]
pub struct Confirmation {
    pub user_id: i32,
    pub action: ConfirmationAction,
    pub used: bool,
    pub date: NaiveDateTime,
    pub token: Vec<u8>,
    pub code_token: Vec<u8>,
    pub code: i16,
    pub code_trials: i16,
    pub redirect_url: Option<String>,
    pub device_string: Option<String>,
    pub ip_address: Option<IpNet>,
}

impl Confirmation {
    pub(crate) fn insert_confirmation(
        conn: &mut DBConn,
        user_id: i32,
        action: ConfirmationAction,
        device_info: &DeviceInfo,
        redirect_url: &Option<String>,
        try_count: u8,
    ) -> Result<(Vec<u8>, Vec<u8>, i16), ErrorResponder> {
        let token = random_token(16);
        let code_token = random_token(16);
        let code = random_code(4) as i16;

        insert_into(confirmations::table)
            .values((
                confirmations::dsl::user_id.eq::<i32>(user_id),
                confirmations::dsl::action.eq(&action),
                confirmations::dsl::token.eq(&token),
                confirmations::dsl::code_token.eq(&code_token),
                confirmations::dsl::code.eq(&code),
                confirmations::dsl::redirect_url.eq(redirect_url),
                confirmations::dsl::device_string.eq(&device_info.device_string),
                confirmations::dsl::ip_address.eq(&device_info.ip_address),
            ))
            .execute(conn)
            .map(|_| (token, code_token, code))
            .or_else(|e| {
                if (is_error_duplicate_key(&e, "confirmations.PRIMARY") || is_error_duplicate_key(&e, "confirmations.UQ_confirmations"))
                    && try_count < 3
                {
                    warn!("Confirmation token already exists, trying again.");
                    return Confirmation::insert_confirmation(conn, user_id, action, device_info, redirect_url, try_count + 1);
                }
                ErrorType::DatabaseError("Failed to insert confirmation".to_string(), e).res_err()
            })
    }
    pub fn check_code_and_mark_as_used(
        conn: &mut DBConn,
        user_id: &i32,
        action: &ConfirmationAction,
        code_token: &Vec<u8>,
        code: &i16,
        max_minutes: i64,
    ) -> Result<Option<String>, ErrorResponder> {
        let confirmation = confirmations::table
            .filter(confirmations::dsl::user_id.eq(user_id))
            .filter(confirmations::dsl::action.eq(action))
            .filter(confirmations::dsl::code_token.eq(code_token))
            .first::<Confirmation>(conn)
            .optional()
            .map_err(|e| ErrorType::DatabaseError("Failed to get confirmation".to_string(), e).res())?;
        if let Some(mut confirmation) = confirmation {
            if confirmation.used {
                return ErrorType::ConfirmationAlreadyUsed.res_err_no_rollback();
            }
            if confirmation.date < Utc::now().naive_utc() - Duration::minutes(max_minutes) {
                return ErrorType::ConfirmationExpired.res_err_no_rollback();
            }
            if confirmation.code_trials >= 3 {
                return ErrorType::ConfirmationTooManyAttempts.res_err_no_rollback();
            }
            if confirmation.code != *code {
                confirmation.code_trials += 1;
                update(confirmations::table)
                    .filter(confirmations::dsl::user_id.eq(user_id))
                    .filter(confirmations::dsl::action.eq(action))
                    .filter(confirmations::dsl::code_token.eq(code_token))
                    .set((confirmations::dsl::code_trials.eq(confirmation.code_trials),))
                    .execute(conn)
                    .map_err(|e| ErrorType::DatabaseError("Failed to update confirmation code trials".to_string(), e).res())?;
                return ErrorType::ConfirmationNotFound.res_err_no_rollback();
            }

            confirmation.mark_as_used(conn)?;
            return Ok(confirmation.redirect_url);
        }
        ErrorType::ConfirmationNotFound.res_err_no_rollback()
    }
    pub fn check_token_and_mark_as_used(
        conn: &mut DBConn,
        user_id: &i32,
        action: &ConfirmationAction,
        token: &Vec<u8>,
        max_minutes: i64,
    ) -> Result<Option<String>, ErrorResponder> {
        let confirmation = confirmations::table
            .filter(confirmations::dsl::user_id.eq(user_id))
            .filter(confirmations::dsl::action.eq(action))
            .filter(confirmations::dsl::token.eq(token))
            .first::<Confirmation>(conn)
            .optional()
            .map_err(|e| ErrorType::DatabaseError("Failed to get confirmation".to_string(), e).res())?;
        if let Some(confirmation) = confirmation {
            if confirmation.used {
                return ErrorType::ConfirmationAlreadyUsed.res_err_no_rollback();
            }
            if confirmation.date < Utc::now().naive_utc() - Duration::minutes(max_minutes) {
                return ErrorType::ConfirmationExpired.res_err_no_rollback();
            }
            confirmation.mark_as_used(conn)?;
            return Ok(confirmation.redirect_url);
        }
        ErrorType::ConfirmationNotFound.res_err_no_rollback()
    }
    pub fn mark_as_used(&self, conn: &mut DBConn) -> Result<(), ErrorResponder> {
        update(confirmations::table)
            .filter(confirmations::dsl::user_id.eq(&self.user_id))
            .filter(confirmations::dsl::action.eq(&self.action))
            .filter(confirmations::dsl::token.eq(&self.token))
            .set((confirmations::dsl::used.eq(true),))
            .execute(conn)
            .map(|_| ())
            .map_err(|e| ErrorType::DatabaseError("Failed to mark confirmation as used".to_string(), e).res())
    }
    pub fn mark_all_as_used(conn: &mut DBConn, user_id: &i32, action: ConfirmationAction) -> Result<(), ErrorResponder> {
        update(confirmations::table)
            .filter(confirmations::dsl::user_id.eq(user_id))
            .filter(confirmations::dsl::action.eq(action))
            .set((confirmations::dsl::used.eq(true),))
            .execute(conn)
            .map(|_| ())
            .map_err(|e| ErrorType::DatabaseError("Failed to mark all confirmations as used".to_string(), e).res())
    }
}
