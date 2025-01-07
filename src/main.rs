#[macro_use]
extern crate rocket;
extern crate tera;

use crate::api::auth::confirm::{
    auth_confirm_code, auth_confirm_token, okapi_add_operation_for_auth_confirm_code_, okapi_add_operation_for_auth_confirm_token_,
};
use crate::api::auth::signin::{auth_signin, auth_signin_email, okapi_add_operation_for_auth_signin_, okapi_add_operation_for_auth_signin_email_};
use crate::api::auth::signup::{auth_signup, okapi_add_operation_for_auth_signup_};
use crate::api::auth::status::{auth_status, okapi_add_operation_for_auth_status_};
use crate::api::picture::{add_picture, get_picture, okapi_add_operation_for_add_picture_, okapi_add_operation_for_get_picture_};
use crate::database::database::{get_connection, get_connection_pool};
use crate::picture_storer::picture_file_storer::PictureFileStorer;
use crate::picture_storer::picture_storer::PictureStorer;
use crate::utils::errors_catcher::{bad_request, internal_error, not_found, unauthorized, unprocessable_entity};
use crate::utils::utils::{get_backend_host, get_frontend_host};
use aws_config::BehaviorVersion;
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
mod ftp_server {
    pub mod ftp;
    pub mod ftp_auth;
    pub mod ftp_backend;
}
mod grouping {
    pub mod grouping_strategy;
}
mod mailing {
    pub mod mailer;
}
mod picture_storer {
    pub mod picture_file_storer;
    pub mod picture_s3_storer;
    pub mod picture_storer;
}
mod utils {
    pub mod auth;
    pub mod errors_catcher;
    pub mod exif;
    pub mod utils;
    pub mod validation;
}

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

/// Entry point of Archypix app backend
#[launch]
#[tokio::main]
async fn rocket() -> _ {
    dotenv().ok();

    // Migrate SQL database
    let mut conn = get_connection();
    let res = conn.run_pending_migrations(MIGRATIONS).unwrap();
    println!("Migrations result: {:?}", res);

    // Load S3 Client
    let mut config_builder = aws_sdk_s3::Config::builder()
        .behavior_version(BehaviorVersion::latest())
        .force_path_style(true)
        .region(aws_sdk_s3::config::Region::new(
            env::var("AWS_REGION").expect("Missing AWS_REGION environment variable"),
        ))
        .credentials_provider(aws_sdk_s3::config::Credentials::new(
            env::var("AWS_ACCESS_KEY_ID").unwrap(),
            env::var("AWS_SECRET_ACCESS_KEY").unwrap(),
            None,
            None,
            "Static",
        ));
    if let Some(endpoint) = env::var("AWS_ENDPOINT").ok() {
        config_builder = config_builder
            .endpoint_url(endpoint)
    }
    let config = config_builder.build();
    let client = aws_sdk_s3::Client::from_conf(config);

    // Create S3 buckets if they don't exist

    let list_buckets_output = client.list_buckets().send().await.unwrap();
    let existing_bucket_names: Vec<String> = list_buckets_output
        .buckets()
        .iter()
        .map(|bucket| bucket.name().unwrap_or_default().to_string())
        .collect();

    let bucket_name = String::from("archypix-picturessss");

    if!existing_bucket_names.contains(&bucket_name) {
        // Create the bucket if it doesn't exist
        let create_bucket_output = client.create_bucket().bucket(&bucket_name).send().await.unwrap();
        println!("Created bucket: {:?}", create_bucket_output);
    } else {
        println!("Bucket '{}' already exists.", &bucket_name);
    }

    let picture_storer = PictureStorer::from(PictureFileStorer::new("./pictures/".to_string()));

    rocket::build()
        .attach(cors_options())
        .manage(picture_storer)
        .manage(client)
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
                get_picture
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
