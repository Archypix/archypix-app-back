use crate::database::auth_token::{AuthToken, Confirmation};
use crate::database::database::DBConn;
use crate::database::schema::*;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use chrono::NaiveDateTime;
use diesel::QueryDsl;
use diesel::{insert_into, select, update, Associations, Identifiable, Insertable, OptionalExtension, Queryable, RunQueryDsl, Selectable};
use diesel::{ExpressionMethods, SelectableHelper};
use pwhash::bcrypt;
use rocket::Request;

#[derive(Queryable, Selectable, Identifiable, Insertable, Debug, PartialEq)]
#[diesel(primary_key(id))]
#[diesel(table_name = users)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: String,
    pub password_hash: String,
    pub creation_date: NaiveDateTime,
    pub status: UserStatus,
    pub tfa_login: bool,
    pub storage_count_ko: u64,
    pub storage_limit_mo: u32,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, PartialEq)]
#[diesel(primary_key(user_id_acceptor, user_id_sharer))]
#[diesel(belongs_to(User, foreign_key = user_id_acceptor, foreign_key = user_id_sharer))]
#[diesel(table_name = shares_auto_accept)]
pub struct ShareAutoAccept {
    pub user_id_acceptor: u32,
    pub user_id_sharer: u32,
}

impl User {
    pub fn from_id(conn: &mut DBConn, id: &u32) -> Result<User, ErrorResponder> {
        User::from_id_opt(conn, id).and_then(|user_opt| {
            user_opt.ok_or_else(|| ErrorType::UserNotFound.res())
        })
    }
    pub fn from_id_opt(conn: &mut DBConn, id: &u32) -> Result<Option<User>, ErrorResponder> {
        users::table
            .filter(users::dsl::id.eq(id))
            .select(User::as_select())
            .first::<User>(conn)
            .optional()
            .map_err(|e| {
                ErrorType::DatabaseError("Failed to get user from id".to_string(), e).res_rollback()
            })
    }
    pub fn find_logged_in(conn: &mut DBConn, user_id: u32, auth_token: Vec<u8>) -> Result<(User, AuthToken), ErrorResponder> {
        User::find_logged_in_opt(conn, user_id, auth_token)
            .and_then(|data| {
                data.ok_or_else(|| ErrorType::UserNotFound.res())
            })
    }
    pub fn find_logged_in_opt(conn: &mut DBConn, user_id: u32, auth_token: Vec<u8>) -> Result<Option<(User, AuthToken)>, ErrorResponder> {
        users::table.left_join(auth_tokens::table)
            .filter(users::dsl::id.eq(user_id))
            .filter(auth_tokens::dsl::token.eq(auth_token))
            .select((User::as_select(), Option::<AuthToken>::as_select()))
            .first::<(User, Option<AuthToken>)>(conn)
            .optional()
            .map_err(|e| {
                ErrorType::DatabaseError("Failed to get user and auth token".to_string(), e).res_rollback()
            })
            .map(|data| {
                data.and_then(|(user, auth)| {
                    auth.map(|auth| (user, auth))
                })
            })
    }

    pub fn find_by_email_opt(conn: &mut DBConn, email: &str) -> Result<Option<User>, ErrorResponder> {
        users::table
            .filter(users::dsl::email.eq(email))
            .select(User::as_select())
            .first::<User>(conn)
            .optional()
            .map_err(|e| {
                ErrorType::DatabaseError("Failed to get user from email".to_string(), e).res_rollback()
            })
    }

    pub(crate) fn create_user(conn: &mut DBConn, name: &str, email: &str, password: &str) -> Result<u32, ErrorResponder> {
        // Check if the user exists and update only if status is unconfirmed
        let existing_user = User::find_by_email_opt(conn, email)?;

        if let Some(user) = existing_user {
            if user.status != UserStatus::Unconfirmed {
                return Err(ErrorType::EmailAlreadyExists.res());
            }
            update(users::table)
                .filter(users::dsl::id.eq(user.id))
                .set((
                    users::dsl::name.eq::<String>(name.to_string()),
                    users::dsl::password_hash.eq(bcrypt::hash(password).unwrap()),
                    users::dsl::creation_date.eq(chrono::Utc::now().naive_utc()),
                ))
                .execute(conn)
                .map_err(|e| {
                    ErrorType::DatabaseError("Failed to update user name and password.".to_string(), e).res_rollback()
                })?;

            // Only the latest singup confirmation is valid
            Confirmation::mark_all_as_used(conn, &user.id, ConfirmationAction::Signup)?;

            return Ok(user.id);
        }

        insert_into(users::table)
            .values((
                users::dsl::name.eq::<String>(name.to_string()),
                users::dsl::email.eq(email.to_string()),
                users::dsl::password_hash.eq(bcrypt::hash(password).unwrap()),
            ))
            .execute(conn)
            .map_err(|e| {
                ErrorType::DatabaseError("Failed to insert user".to_string(), e).res_rollback()
            })
            .and_then(|_| {
                select(last_insert_id()).get_result::<u64>(conn)
                    .map(|id| id as u32)
                    .map_err(|e| {
                        ErrorType::DatabaseError("Failed to get last insert id".to_string(), e).res_rollback()
                    })
            })
    }

    pub fn switch_status(&self, conn: &mut DBConn, status: &UserStatus) -> Result<(), ErrorResponder> {
        Self::switch_status_from_id(conn, &self.id, status)
    }
    pub fn switch_status_from_id(conn: &mut DBConn, user_id: &u32, status: &UserStatus) -> Result<(), ErrorResponder> {
        update(users::table)
            .filter(users::dsl::id.eq(user_id))
            .set(users::dsl::status.eq(status))
            .execute(conn)
            .map_err(|e| {
                ErrorType::DatabaseError("Failed to update user status".to_string(), e).res_rollback()
            })?;
        Ok(())
    }

    pub fn get_id_from_headers(request: &Request<'_>) -> Option<u32> {
        request.headers().get_one("X-User-Id").map(|s| s.parse::<u32>().ok()).flatten()
    }
}

impl ShareAutoAccept {}
