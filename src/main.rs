#[macro_use]
extern crate rocket;
extern crate tera;

use crate::api::auth::confirm::{
    auth_confirm_code, auth_confirm_token, okapi_add_operation_for_auth_confirm_code_, okapi_add_operation_for_auth_confirm_token_,
};
use crate::api::auth::signin::{auth_signin, auth_signin_email, okapi_add_operation_for_auth_signin_, okapi_add_operation_for_auth_signin_email_};
use crate::api::auth::signup::{auth_signup, okapi_add_operation_for_auth_signup_};
use crate::api::auth::status::{auth_status, okapi_add_operation_for_auth_status_};
use crate::api::groups::arrangement::{
    create_arrangement, delete_arrangement, edit_arrangement, list_arrangements, okapi_add_operation_for_create_arrangement_,
    okapi_add_operation_for_delete_arrangement_, okapi_add_operation_for_edit_arrangement_, okapi_add_operation_for_list_arrangements_,
};
use crate::api::groups::manual_groups::{
    add_pictures_to_group, create_manual_group, okapi_add_operation_for_add_pictures_to_group_, okapi_add_operation_for_create_manual_group_,
    okapi_add_operation_for_remove_pictures_from_group_, remove_pictures_from_group,
};
use crate::api::picture::{
    add_picture, get_picture, get_picture_details, get_pictures_details, okapi_add_operation_for_add_picture_, okapi_add_operation_for_get_picture_,
    okapi_add_operation_for_get_picture_details_, okapi_add_operation_for_get_pictures_details_,
};
use crate::api::query_pictures::{okapi_add_operation_for_query_pictures_, query_pictures};
use crate::api::tags::{
    create_tag_group, delete_tag_group, edit_picture_tags, list_tags, okapi_add_operation_for_create_tag_group_,
    okapi_add_operation_for_delete_tag_group_, okapi_add_operation_for_edit_picture_tags_, okapi_add_operation_for_list_tags_,
    okapi_add_operation_for_patch_tag_group_, patch_tag_group,
};
use crate::database::database::{get_connection, get_connection_pool};
use crate::database::picture::picture::Picture;
use crate::utils::errors_catcher::{bad_request, internal_error, not_found, unauthorized, unprocessable_entity};
use crate::utils::s3::PictureStorer;
use crate::utils::thumbnail::create_temp_directories;
use crate::utils::utils::{get_backend_host, get_frontend_host};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use dotenvy::dotenv;
use rocket::http::Method;
use rocket::log::private::LevelFilter;
use rocket_cors::{AllowedHeaders, AllowedOrigins, Cors, CorsOptions};
use rocket_okapi::openapi_get_routes;
use rocket_okapi::rapidoc::{make_rapidoc, GeneralConfig, HideShowConfig, RapiDocConfig};
use rocket_okapi::settings::UrlObject;
use rocket_okapi::swagger_ui::{make_swagger_ui, SwaggerUIConfig};
use std::env;
use user_agent_parser::UserAgentParser;

pub mod api {
    automod::dir!(pub "src/api/");
    pub mod admin {
        automod::dir!(pub "src/api/admin");
    }
    pub mod auth {
        automod::dir!(pub "src/api/auth");
    }
    pub mod groups {
        automod::dir!(pub "src/api/groups");
    }
}
pub mod database {
    automod::dir!(pub "src/database");
    pub mod group {
        automod::dir!(pub "src/database/group");
    }
    pub mod hierarchy {
        automod::dir!(pub "src/database/hierarchy");
    }
    pub mod picture {
        automod::dir!(pub "src/database/picture");
    }
    pub mod tag {
        automod::dir!(pub "src/database/tag");
    }
    pub mod user {
        automod::dir!(pub "src/database/user");
    }
}
pub mod grouping {
    //automod::dir!(pub "src/grouping");
    pub mod arrangement_strategy;
    pub mod group_by_exif_interval;
    pub mod group_by_exif_value;
    pub mod group_by_filter;
    pub mod group_by_location;
    pub mod group_by_tag;
    pub mod grouping_process;
    pub mod strategy_filtering;
    pub mod strategy_grouping;
    pub mod topological_sorts;
    pub mod tests {
        #[cfg(test)]
        pub mod arrangement_sort_algorithms;
    }
}
pub mod mailing {
    automod::dir!(pub "src/mailing");
}
pub mod utils {
    automod::dir!(pub "src/utils");
}

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

/// Entry point of Archypix app backend
#[launch]
#[tokio::main]
async fn rocket() -> _ {
    env_logger::Builder::new()
        .filter(None, LevelFilter::Info)
        .filter_module("rocket_cors", LevelFilter::Warn)
        .filter_module("archypix_app_back", LevelFilter::Trace)
        .init();

    info!("Starting Archypix app backend...");
    trace!("Backend version: {}", env!("CARGO_PKG_VERSION"));
    dotenv().ok();

    // Migrate SQL database
    let mut conn = get_connection();
    let res = conn.run_pending_migrations(MIGRATIONS).unwrap();
    info!("Migrations result: {:?}", res);

    // Load S3 Client
    let picture_storer = PictureStorer::new().await;

    // Create pictures temp directories
    create_temp_directories();

    let cors = cors_options();
    rocket::build()
        .manage(picture_storer)
        .manage(get_connection_pool())
        .manage(UserAgentParser::from_path("./static/user_agent_regexes.yaml").unwrap())
        .mount(
            "/",
            openapi_get_routes![
                // Auth
                auth_signup,
                auth_signin,
                auth_signin_email,
                auth_status,
                auth_confirm_code,
                auth_confirm_token,
                // Picture
                add_picture,
                get_picture,
                query_pictures,
                get_pictures_details,
                get_picture_details,
                // Tags
                list_tags,
                create_tag_group,
                patch_tag_group,
                delete_tag_group,
                edit_picture_tags,
                // Arrangements
                list_arrangements,
                create_arrangement,
                edit_arrangement,
                delete_arrangement,
                // Groups
                create_manual_group,
                add_pictures_to_group,
                remove_pictures_from_group
            ],
        )
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
        .mount("/", rocket_cors::catch_all_options_routes())
        .attach(cors.clone())
        .manage(cors)
        .register("/", catchers![bad_request, unauthorized, not_found, unprocessable_entity, internal_error])
}

/// CORS configuration
fn cors_options() -> Cors {
    let origin = [get_frontend_host(), get_backend_host()];
    CorsOptions {
        allowed_origins: AllowedOrigins::some_exact(&origin),
        allowed_methods: vec![Method::Get, Method::Post, Method::Put, Method::Patch, Method::Delete]
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
