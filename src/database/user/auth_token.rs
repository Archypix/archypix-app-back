use crate::database::database::DBConn;
use crate::database::schema::*;
use crate::database::utils::is_error_duplicate_key;
use crate::utils::auth::DeviceInfo;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use crate::utils::utils::random_token;
use chrono::{Local, NaiveDateTime, TimeDelta, Utc};
use diesel::delete;
use diesel::ExpressionMethods;
use diesel::{insert_into, update, Identifiable, Insertable, Queryable, RunQueryDsl, Selectable};
use ipnet::IpNet;
use rocket::Request;

#[derive(Queryable, Selectable, Identifiable, Insertable, Debug, PartialEq)]
#[diesel(primary_key(user_id, token))]
#[diesel(belongs_to(User))]
#[diesel(table_name = auth_tokens)]
pub struct AuthToken {
    pub user_id: i32,
    pub token: Vec<u8>,
    pub creation_date: NaiveDateTime,
    pub last_use_date: NaiveDateTime,
    pub device_string: Option<String>,
    pub ip_address: Option<IpNet>,
}

impl AuthToken {
    pub(crate) fn insert_token_for_user(
        conn: &mut DBConn,
        user_id: &i32,
        device_info: &DeviceInfo,
        try_count: u8,
    ) -> Result<Vec<u8>, ErrorResponder> {
        let auth_token = random_token(32);

        insert_into(auth_tokens::table)
            .values((
                auth_tokens::dsl::user_id.eq(user_id),
                auth_tokens::dsl::token.eq(&auth_token),
                auth_tokens::dsl::device_string.eq(&device_info.device_string),
                auth_tokens::dsl::ip_address.eq(&device_info.ip_address),
            ))
            .execute(conn)
            .map(|_| auth_token)
            .or_else(|e| {
                if is_error_duplicate_key(&e, "auth_tokens.PRIMARY") && try_count < 4 {
                    warn!("Auth token already exists, trying again.");
                    return AuthToken::insert_token_for_user(conn, user_id, device_info, try_count + 1);
                }
                ErrorType::DatabaseError("Failed to insert auth token".to_string(), e).res_err()
            })
    }
    pub fn update_last_use_date(&self, conn: &mut DBConn) -> Result<(), ErrorResponder> {
        // Working in UTC time.
        let current_naive = Utc::now().naive_utc();
        if current_naive - self.last_use_date > TimeDelta::try_minutes(10).unwrap() {
            info!("Updating last_use_date of auth_token for user {}", self.user_id);
            update(auth_tokens::table)
                .filter(auth_tokens::dsl::user_id.eq(self.user_id))
                .filter(auth_tokens::dsl::token.eq(self.token.clone()))
                .set((auth_tokens::dsl::last_use_date.eq(Local::now().naive_utc()),))
                .execute(conn)
                .map_err(|e| ErrorType::DatabaseError("Failed to update auth token use date".to_string(), e).res_no_rollback())?;
        }
        Ok(())
    }
    pub fn get_auth_token_from_headers(request: &Request<'_>) -> Option<Vec<u8>> {
        request.headers().get_one("X-Auth-Token").map(|s| hex::decode(s).ok()).flatten()
    }
    pub fn clear_auth_tokens(conn: &mut DBConn, user_id: &i32) -> Result<(), ErrorResponder> {
        delete(auth_tokens::table)
            .filter(auth_tokens::dsl::user_id.eq(user_id))
            .execute(conn)
            .map(|_| ())
            .map_err(|e| ErrorType::DatabaseError("Failed to delete existing auth tokens".to_string(), e).res())
    }
}
