#[macro_use]
extern crate rocket;
extern crate tera;

use crate::api::auth::confirm::{
    auth_confirm_code, auth_confirm_token, okapi_add_operation_for_auth_confirm_code_, okapi_add_operation_for_auth_confirm_token_,
};
use crate::api::auth::signin::{auth_signin, auth_signin_email, okapi_add_operation_for_auth_signin_, okapi_add_operation_for_auth_signin_email_};
use crate::api::auth::signup::{auth_signup, okapi_add_operation_for_auth_signup_};
use crate::api::auth::status::{auth_status, okapi_add_operation_for_auth_status_};
use crate::api::groups::manual_groups::{
    add_pictures_to_group, create_manual_group, okapi_add_operation_for_add_pictures_to_group_, okapi_add_operation_for_create_manual_group_,
    okapi_add_operation_for_remove_pictures_from_group_, remove_pictures_from_group,
};
use crate::api::picture::{add_picture, get_picture, okapi_add_operation_for_add_picture_, okapi_add_operation_for_get_picture_};
use crate::database::database::{get_connection, get_connection_pool};
use crate::utils::errors_catcher::{bad_request, internal_error, not_found, unauthorized, unprocessable_entity};
use crate::utils::s3::PictureStorer;
use crate::utils::thumbnail::create_temp_directories;
use crate::utils::utils::{get_backend_host, get_frontend_host};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use dotenvy::dotenv;
use rocket::http::Method;
use rocket_cors::{AllowedHeaders, AllowedOrigins, Cors, CorsOptions};
use rocket_okapi::openapi_get_routes;
use rocket_okapi::rapidoc::{make_rapidoc, GeneralConfig, HideShowConfig, RapiDocConfig};
use rocket_okapi::settings::UrlObject;
use rocket_okapi::swagger_ui::{make_swagger_ui, SwaggerUIConfig};
use std::env;
use user_agent_parser::UserAgentParser;

mod api {
    pub mod picture;

    pub mod admin {
        pub mod admin;
    }

    pub mod groups {
        pub mod manual_groups;
    }

    pub mod auth {
        pub mod confirm;
        pub mod signin;
        pub mod signup;
        pub mod status;
    }
}
mod database {
    pub mod auth_token;
    pub mod database;
    pub mod duplicates;
    pub mod group;
    pub mod hierarchy;
    pub mod picture;
    pub mod schema;
    pub mod tags;
    pub mod user;
    pub mod utils;
}
/*mod ftp_server {
    pub mod ftp;
    pub mod ftp_auth;
    pub mod ftp_backend;
}*/
mod grouping {
    pub mod grouping_strategy;
}
mod mailing {
    pub mod mailer;
}
mod utils {
    pub mod auth;
    pub mod errors_catcher;
    pub mod exif;
    pub mod s3;
    pub mod thumbnail;
    pub mod utils;
    pub mod validation;
}

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

/// Entry point of Archypix app backend
#[launch]
#[tokio::main]
async fn rocket() -> _ {
    println!("Starting Archypix app backend...");
    dotenv().ok();

    // Migrate SQL database
    let mut conn = get_connection();
    let res = conn.run_pending_migrations(MIGRATIONS).unwrap();
    println!("Migrations result: {:?}", res);

    // Load S3 Client
    let picture_storer = PictureStorer::new().await;

    // Create pictures temp directories
    create_temp_directories();

    rocket::build()
        .attach(cors_options())
        .manage(picture_storer)
        .manage(get_connection_pool())
        .manage(UserAgentParser::from_path("./static/user_agent_regexes.yaml").unwrap())
        .mount(
            "/",
            openapi_get_routes![
                auth_signup,
                auth_signin,
                auth_signin_email,
                auth_status,
                auth_confirm_code,
                auth_confirm_token,
                add_picture,
                get_picture,
                create_manual_group,
                add_pictures_to_group,
                remove_pictures_from_group
            ],
        )
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
