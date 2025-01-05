#[macro_use]
extern crate rocket;
extern crate tera;

use crate::api::auth::confirm::{auth_confirm_code, auth_confirm_token, okapi_add_operation_for_auth_confirm_code_, okapi_add_operation_for_auth_confirm_token_};
use crate::api::auth::signin::{auth_signin, auth_signin_email, okapi_add_operation_for_auth_signin_, okapi_add_operation_for_auth_signin_email_};
use crate::api::auth::signup::{auth_signup, okapi_add_operation_for_auth_signup_};
use crate::api::auth::status::{auth_status, okapi_add_operation_for_auth_status_};
use crate::database::database::{get_connection, get_connection_pool};
use crate::utils::errors_catcher::{bad_request, internal_error, not_found, unauthorized, unprocessable_entity};
use crate::utils::utils::{get_backend_host, get_frontend_host};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use dotenvy::dotenv;
use rocket::http::Method;
use rocket_cors::{AllowedHeaders, AllowedOrigins, Cors, CorsOptions};
use rocket_okapi::openapi_get_routes;
use rocket_okapi::rapidoc::{make_rapidoc, GeneralConfig, HideShowConfig, RapiDocConfig};
use rocket_okapi::settings::UrlObject;
use rocket_okapi::swagger_ui::{make_swagger_ui, SwaggerUIConfig};
use user_agent_parser::UserAgentParser;

mod api {
    pub mod admin {
        pub mod admin;
    }

    pub mod auth {
        pub mod signup;
        pub mod signin;
        pub mod status;
        pub mod confirm;
    }
}
mod database {
    pub mod database;
    pub mod duplicates;
    pub mod schema;
    pub mod user;
    pub mod auth_token;
    pub mod tags;
    pub mod picture;
    pub mod group;
    pub mod hierarchy;
    pub mod utils;
}
mod ftp_server {
    pub mod ftp;
    pub mod ftp_auth;
    pub mod ftp_backend;
}
mod grouping {
    pub mod grouping_strategy;
}
mod utils {
    pub mod utils;
    pub mod errors_catcher;
    pub mod validation;
    pub mod auth;
}
mod mailing {
    pub mod mailer;
}

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

/// Entry point of Archypix app backend
#[launch]
#[tokio::main]
async fn rocket() -> _ {
    dotenv().ok();

    // migrate database
    let mut conn = get_connection();
    let res = conn.run_pending_migrations(MIGRATIONS).unwrap();
    println!("Migrations result: {:?}", res);

    rocket::build()
        .attach(cors_options())
        .manage(get_connection_pool())
        .manage(UserAgentParser::from_path("./static/user_agent_regexes.yaml").unwrap())
        .mount("/", openapi_get_routes![auth_signup, auth_signin, auth_signin_email, auth_status, auth_confirm_code, auth_confirm_token])
        .register("/", catchers![bad_request, unauthorized, not_found, unprocessable_entity, internal_error])
        .mount(
            "/swagger-ui/",
            make_swagger_ui(&SwaggerUIConfig {
                url: "../openapi.json".to_owned(),
                ..Default::default()
            }),
        )
        .mount(
            "/rapidoc/",
            make_rapidoc(&RapiDocConfig {
                general: GeneralConfig {
                    spec_urls: vec![UrlObject::new("General", "../openapi.json")],
                    ..Default::default()
                },
                hide_show: HideShowConfig {
                    allow_spec_url_load: false,
                    allow_spec_file_load: false,
                    ..Default::default()
                },
                ..Default::default()
            }),
        )
}

/// CORS configuration
fn cors_options() -> Cors {
    let origin = [get_frontend_host(), get_backend_host()];
    CorsOptions {
        allowed_origins: AllowedOrigins::some_exact(&origin),
        allowed_methods: vec![Method::Get, Method::Post, Method::Put, Method::Delete]
            .into_iter()
            .map(From::from)
            .collect(),
        allowed_headers: AllowedHeaders::all(),
        allow_credentials: true,
        ..Default::default()
    }
        .to_cors()
        .expect("Error while building CORS")
}




