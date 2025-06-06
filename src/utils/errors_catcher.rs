use crate::database::database::DBConn;
use diesel::result::Error;
use diesel::Connection;
use enum_kinds::EnumKind;
use rexiv2::Rexiv2Error;
use rocket::serde::json::Json;
use rocket::Request;
use rocket_okapi::gen::OpenApiGenerator;
use rocket_okapi::okapi::openapi3::Responses;
use rocket_okapi::response::OpenApiResponderInner;
use schemars::JsonSchema;
use serde::Serialize;
use strum_macros::{Display, EnumIter};

/// Rocket Responder for all errors
#[derive(Responder, Debug)]
pub enum ErrorResponder {
    #[response(status = 400, content_type = "json")]
    BadRequest(Json<ErrorResponse>),
    #[response(status = 401, content_type = "json")]
    Unauthorized(Json<ErrorResponse>),
    #[response(status = 404, content_type = "json")]
    NotFound(Json<ErrorResponse>),
    #[response(status = 422, content_type = "json")]
    UnprocessableEntity(Json<ErrorResponse>),
    #[response(status = 500, content_type = "json")]
    InternalError(Json<ErrorResponse>),
}
/// Convert Diesel [`Error`] to [`ErrorResponder`]
impl From<Error> for ErrorResponder {
    fn from(value: Error) -> Self {
        // Rollback all uncaught errors
        ErrorType::DatabaseError("Diesel error".to_string(), value).res()
    }
}
impl ErrorResponder {
    /// Extract the rollback boolean value from the inner [`ErrorResponse`] struct.
    pub fn do_rollback(&self) -> bool {
        match self {
            ErrorResponder::BadRequest(json) => json,
            ErrorResponder::Unauthorized(json) => json,
            ErrorResponder::NotFound(json) => json,
            ErrorResponder::UnprocessableEntity(json) => json,
            ErrorResponder::InternalError(json) => json,
        }
        .rollback
    }
    pub fn with_rollback(&self, rollback: bool) -> ErrorResponder {
        match self {
            ErrorResponder::BadRequest(json) => {
                let mut json = Json(json.0.clone());
                json.rollback = rollback;
                ErrorResponder::BadRequest(json)
            }
            ErrorResponder::Unauthorized(json) => {
                let mut json = Json(json.0.clone());
                json.rollback = rollback;
                ErrorResponder::Unauthorized(json)
            }
            ErrorResponder::NotFound(json) => {
                let mut json = Json(json.0.clone());
                json.rollback = rollback;
                ErrorResponder::NotFound(json)
            }
            ErrorResponder::UnprocessableEntity(json) => {
                let mut json = Json(json.0.clone());
                json.rollback = rollback;
                ErrorResponder::UnprocessableEntity(json)
            }
            ErrorResponder::InternalError(json) => {
                let mut json = Json(json.0.clone());
                json.rollback = rollback;
                ErrorResponder::InternalError(json)
            }
        }
    }
}
/// Dummy implementation for OpenApi
impl OpenApiResponderInner for ErrorResponder {
    fn responses(_: &mut OpenApiGenerator) -> rocket_okapi::Result<Responses> {
        Ok(Responses::default())
    }
}

/// Error response data struct
#[derive(JsonSchema, Serialize, Debug, Clone)]
pub struct ErrorResponse {
    pub error_type: ErrorTypeKind,
    pub message: String,
    // Rollback the diesel transaction if true
    pub rollback: bool,
}
impl From<ErrorResponder> for ErrorResponse {
    fn from(value: ErrorResponder) -> Self {
        match value {
            ErrorResponder::BadRequest(json) => json.into_inner(),
            ErrorResponder::Unauthorized(json) => json.into_inner(),
            ErrorResponder::NotFound(json) => json.into_inner(),
            ErrorResponder::UnprocessableEntity(json) => json.into_inner(),
            ErrorResponder::InternalError(json) => json.into_inner(),
        }
    }
}

/// All possible error types that can be converted to [`ErrorResponse`] and then [`ErrorResponder`]
#[derive(EnumKind, Debug, Display)]
#[enum_kind(ErrorTypeKind, derive(EnumIter, Display, JsonSchema, Serialize))]
pub enum ErrorType {
    BadRequest,
    Unauthorized,
    NotFound(String),
    UnprocessableEntity(String),
    InternalError(String),
    // Form validation (see UnprocessableEntity for type check related errors)
    InvalidInput(String),
    // User request guard
    UserNotFound,
    UserBanned,
    UserUnconfirmed,
    // Sign in types
    InvalidEmailOrPassword,
    TFARequiredOverEmail, // Only email confirm available
    TFARequired,          // TOTP or email confirm available
    InvalidTOTPCode,
    // Sign up types
    EmailAlreadyExists,
    // Confirm
    ConfirmationAlreadyUsed,
    ConfirmationExpired,
    ConfirmationTooManyAttempts,
    ConfirmationNotFound,
    // Admin
    UserNotAdmin,
    // Database error
    DatabaseError(String, Error),
    // Pictures and files
    UnableToLoadExifMetadata(Rexiv2Error),
    S3Error(String),
    UnableToCreateThumbnail(String),
    PictureNotFound,
    // Groups
    GroupIsNotManual,
    ArrangementNotFound,
    // Tags
    TagNotFound,
}

impl ErrorType {
    /// Convert to a result of [`ErrorResponder`] without Diesel transaction rollback
    pub fn res_err_no_rollback<T>(self) -> Result<T, ErrorResponder> {
        Err(self.to_responder(false))
    }
    /// Convert to a result of [`ErrorResponder`] with Diesel transaction rollback
    pub fn res_err<T>(self) -> Result<T, ErrorResponder> {
        Err(self.to_responder(true))
    }
    /// Convert to a [`ErrorResponder`] without Diesel transaction rollback
    pub fn res_no_rollback(self) -> ErrorResponder {
        self.to_responder(false)
    }
    /// Convert to a [`ErrorResponder`] with Diesel transaction rollback
    pub fn res(self) -> ErrorResponder {
        self.to_responder(true)
    }

    /// Converts to a [`ErrorResponder`]
    fn to_responder(self, rollback: bool) -> ErrorResponder {
        let kind = ErrorTypeKind::from(&self);
        match self {
            // Default HTTP types
            ErrorType::BadRequest => ErrorResponder::BadRequest(Self::create_response("Bad request".to_string(), kind, rollback)),
            ErrorType::Unauthorized => ErrorResponder::Unauthorized(Self::create_response("Unauthorized".to_string(), kind, rollback)),
            ErrorType::NotFound(path) => ErrorResponder::NotFound(Self::create_response(format!("Not found: {}", path), kind, rollback)),
            ErrorType::UnprocessableEntity(msg) => ErrorResponder::UnprocessableEntity(Self::create_response(msg, kind, rollback)),
            ErrorType::InternalError(msg) => {
                ErrorResponder::InternalError(Self::create_response(format!("Internal error: {}", msg).to_string(), kind, rollback))
            }
            // Form validation (see UnprocessableEntity for type check related errors)
            ErrorType::InvalidInput(msg) => ErrorResponder::UnprocessableEntity(Self::create_response(msg, kind, rollback)),
            // Sign in / status types
            ErrorType::UserNotFound => ErrorResponder::Unauthorized(Self::create_response("User not found".to_string(), kind, rollback)),
            ErrorType::UserBanned => ErrorResponder::Unauthorized(Self::create_response("User is banned".to_string(), kind, rollback)),
            ErrorType::UserUnconfirmed => ErrorResponder::Unauthorized(Self::create_response("User is not confirmed".to_string(), kind, rollback)),
            // Sign in types
            ErrorType::InvalidEmailOrPassword => {
                ErrorResponder::Unauthorized(Self::create_response("Invalid email or password".to_string(), kind, rollback))
            }
            ErrorType::TFARequiredOverEmail => {
                ErrorResponder::Unauthorized(Self::create_response("2FA required over email".to_string(), kind, rollback))
            }
            ErrorType::TFARequired => ErrorResponder::Unauthorized(Self::create_response("2FA required".to_string(), kind, rollback)),
            ErrorType::InvalidTOTPCode => ErrorResponder::Unauthorized(Self::create_response("Invalid TOTP code".to_string(), kind, rollback)),
            // Sign up types
            ErrorType::EmailAlreadyExists => ErrorResponder::Unauthorized(Self::create_response("Email already exists".to_string(), kind, rollback)),
            // Confirm
            ErrorType::ConfirmationAlreadyUsed => {
                ErrorResponder::Unauthorized(Self::create_response("Confirmation code/token already used".to_string(), kind, rollback))
            }
            ErrorType::ConfirmationExpired => {
                ErrorResponder::Unauthorized(Self::create_response("Confirmation code/token expired".to_string(), kind, rollback))
            }
            ErrorType::ConfirmationTooManyAttempts => {
                ErrorResponder::Unauthorized(Self::create_response("Too many attempts".to_string(), kind, rollback))
            }
            ErrorType::ConfirmationNotFound => ErrorResponder::Unauthorized(Self::create_response("Invalid code/token".to_string(), kind, rollback)),
            // Admin
            ErrorType::UserNotAdmin => ErrorResponder::Unauthorized(Self::create_response("User is not an admin".to_string(), kind, rollback)),
            // Database error
            ErrorType::DatabaseError(msg, err) => {
                ErrorResponder::InternalError(Self::create_response(format!("Database error: {} - {}", msg, err), kind, rollback))
            }
            // Pictures and files
            ErrorType::UnableToLoadExifMetadata(err) => ErrorResponder::InternalError(Self::create_response(
                format!("Unable to load Exif metadata: {}", err.to_string()),
                kind,
                rollback,
            )),
            ErrorType::S3Error(msg) => ErrorResponder::InternalError(Self::create_response(format!("S3 error: {}", msg), kind, rollback)),
            ErrorType::UnableToCreateThumbnail(msg) => {
                ErrorResponder::InternalError(Self::create_response(format!("Unable to create thumbnail: {}", msg), kind, rollback))
            }
            ErrorType::PictureNotFound => ErrorResponder::NotFound(Self::create_response("Picture not found".to_string(), kind, rollback)),
            // Groups
            ErrorType::GroupIsNotManual => ErrorResponder::BadRequest(Self::create_response(
                "You can’t manage pictures of a non-manual group.".to_string(),
                kind,
                rollback,
            )),
            ErrorType::ArrangementNotFound => ErrorResponder::NotFound(Self::create_response("Arrangement not found".to_string(), kind, rollback)),
            ErrorType::TagNotFound => ErrorResponder::NotFound(Self::create_response("Tag not found".to_string(), kind, rollback)),
        }
    }
    /// Converts to an [`ErrorResponse`] struct
    fn create_response(message: String, error_type: ErrorTypeKind, rollback: bool) -> Json<ErrorResponse> {
        Json(ErrorResponse {
            message,
            error_type,
            rollback,
        })
    }
}

#[catch(400)]
pub fn bad_request() -> ErrorResponder {
    ErrorType::BadRequest.res_no_rollback()
}
#[catch(401)]
pub fn unauthorized() -> ErrorResponder {
    ErrorType::Unauthorized.res_no_rollback()
}
#[catch(404)]
pub fn not_found(req: &Request) -> ErrorResponder {
    ErrorType::NotFound(req.uri().to_string()).res_no_rollback()
}
/// When a JSON value type is incorrect
#[catch(422)]
pub fn unprocessable_entity() -> ErrorResponder {
    ErrorType::UnprocessableEntity("Invalid JSON structure".to_string()).res_no_rollback()
}
#[catch(500)]
pub fn internal_error() -> ErrorResponder {
    ErrorType::InternalError(String::from("Internal Error")).res_no_rollback()
}

/// Diesel transaction encapsulation to handle rollback
/// depending on the rollback boolean value contained in the returned Err(ErrorResponder) struct.
pub fn err_transaction<T, F>(conn: &mut DBConn, f: F) -> Result<T, ErrorResponder>
where
    F: FnOnce(&mut DBConn) -> Result<T, ErrorResponder>,
{
    let result = conn.transaction::<Result<T, ErrorResponder>, ErrorResponder, _>(|conn| {
        let res = f(conn);
        if let Err(err) = res {
            if err.do_rollback() {
                Err(err)
            } else {
                // Returns Ok(Err(ErrorResponder)) to avoid rollback
                Ok(Err(err))
            }
        } else {
            Ok(res)
        }
    });
    match result {
        Ok(Ok(res)) => Ok(res),
        Ok(Err(err)) => Err(err),
        Err(err) => Err(err),
    }
}
