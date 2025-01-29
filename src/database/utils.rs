use crate::database::database::DBConn;
use crate::database::schema::last_insert_id;
use crate::utils::errors_catcher::{ErrorResponder, ErrorType};
use diesel::dsl::select;
use diesel::RunQueryDsl;

pub fn is_error_duplicate_key(error: &diesel::result::Error, key: &str) -> bool {
    use diesel::result::DatabaseErrorKind;
    use diesel::result::Error;

    if let Error::DatabaseError(kind, info) = error {
        // println!("Error message: {}, error column: {:?}, error table: {:?}, constraint name: {:?}, kind: {:?}", info.message(), info.column_name(), info.table_name(), info.constraint_name(), kind);
        if let DatabaseErrorKind::UniqueViolation = kind {
            // Format examples:
            // Duplicate entry 'example@gmail.come' for key 'users.email'
            // Duplicate entry '3-signup-\x00' for key 'confirmations.PRIMARY'
            // Duplicate entry '3-signup-\x00-0' for key 'confirmations.UQ_confirmations'

            let error_parts = info.message().split('\'').collect::<Vec<&str>>();
            return error_parts.len() > 3 && error_parts[3] == key;
        }
    }
    false
}

pub fn get_last_inserted_id(conn: &mut DBConn) -> Result<u64, ErrorResponder> {
    select(last_insert_id())
        .get_result::<u64>(conn)
        .map_err(|e| ErrorType::DatabaseError("Failed to get last insert id".to_string(), e).res_rollback())
}
