use crate::database::database::DBConn;
use crate::database::schema::totp_secrets;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use chrono::NaiveDateTime;
use diesel::ExpressionMethods;
use diesel::{insert_into, OptionalExtension, QueryDsl, RunQueryDsl, SelectableHelper};
use diesel_derives::{Identifiable, Insertable, Queryable, Selectable};
use totp_rs::{Rfc6238, TOTP};

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
            .values((totp_secrets::dsl::user_id.eq(user_id), totp_secrets::dsl::secret.eq(secret)))
            .execute(conn)
            .map(|_| ())
            .map_err(|e| ErrorType::DatabaseError("Failed to insert TOTP secret".to_string(), e).res_rollback())
    }
    pub fn has_user_totp(conn: &mut DBConn, user_id: &u32) -> Result<bool, ErrorResponder> {
        totp_secrets::table
            .filter(totp_secrets::dsl::user_id.eq(user_id))
            .select(totp_secrets::dsl::user_id)
            .first::<u32>(conn)
            .optional()
            .map(|opt| opt.is_some())
            .map_err(|e| ErrorType::DatabaseError("Failed to check if user has TOTP".to_string(), e).res_rollback())
    }
    pub fn get_user_totp_secrets(conn: &mut DBConn, user_id: &u32) -> Result<Vec<TOTPSecret>, ErrorResponder> {
        totp_secrets::table
            .filter(totp_secrets::dsl::user_id.eq(user_id))
            .select(TOTPSecret::as_select())
            .load::<TOTPSecret>(conn)
            .map_err(|e| ErrorType::DatabaseError("Failed to get user TOTP secrets".to_string(), e).res_rollback())
    }
    pub fn check_user_totp(conn: &mut DBConn, user_id: &u32, code: &str) -> Result<bool, ErrorResponder> {
        let secrets = TOTPSecret::get_user_totp_secrets(conn, user_id)?;
        for secret in secrets {
            if secret
                .to_totp()?
                .check_current(code)
                .map_err(|_| ErrorType::InternalError("SystemTimeError occurred when checking TOTP.".to_string()).res())?
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn to_totp(&self) -> Result<TOTP, ErrorResponder> {
        let rf6238 = Rfc6238::new(
            6,
            self.secret.clone(),
            Some("Archypix".to_string()),
            "clementgre@archypix.com".to_string(),
        )
        .map_err(|_| ErrorType::InternalError("Unable to create Rfc6238 (for TOTP)".to_string()).res())?;
        TOTP::from_rfc6238(rf6238).map_err(|_| ErrorType::InternalError("Unable to create TOTP".to_string()).res())
    }
}
