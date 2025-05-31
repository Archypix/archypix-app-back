use diesel::sql_types::{Binary, Inet, Nullable, SqlType, VarChar};
use diesel::{allow_tables_to_appear_in_same_query, joinable, table};
use diesel_derives::define_sql_function;
use rocket_okapi::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Debug, PartialEq, Serialize, diesel_derive_enum::DbEnum)]
#[DbValueStyle = "snake_case"]
pub enum UserStatus {
    Unconfirmed,
    Normal,
    Banned,
    Admin,
}
table! {
    use diesel::sql_types::*;
    use super::UserStatusMapping;
    users (id) {
        id -> Serial,
        name -> Varchar,
        email -> Varchar,
        // 60 character
        password_hash -> Char,
        creation_date -> Timestamp,
        status -> UserStatusMapping,
        tfa_login -> Bool,
        storage_count_ko -> Int8,
        storage_limit_ko -> Int8,
    }
}

table! {
    auth_tokens (user_id, token) {
        user_id -> Serial,
        token -> Binary,
        creation_date -> Timestamp,
        last_use_date -> Timestamp,
        device_string -> Nullable<Varchar>,
        ip_address -> Nullable<Inet>,
    }
}
joinable!(auth_tokens -> users (user_id));
allow_tables_to_appear_in_same_query!(auth_tokens, users);

#[derive(JsonSchema, Debug, PartialEq, Deserialize, Serialize, diesel_derive_enum::DbEnum)]
#[DbValueStyle = "snake_case"]
pub enum ConfirmationAction {
    Signup,
    Signin,
    DeleteAccount,
}
table! {
    use diesel::sql_types::*;
    use super::ConfirmationActionMapping;
    confirmations (user_id, action, token) {
        user_id -> Serial,
        // 16 byte
        action -> ConfirmationActionMapping,
        used -> Bool,
        date -> Timestamp,
        token -> Binary,
        code_token -> Binary,
        code -> Int2,
        code_trials -> Int2,
        redirect_url -> Nullable<Varchar>,
        device_string -> Nullable<Varchar>,
        ip_address -> Nullable<Inet>,
    }
}
joinable!(confirmations -> users (user_id));
allow_tables_to_appear_in_same_query!(confirmations, users);

table! {
    totp_secrets (user_id) {
        user_id -> Serial,
        creation_date -> Timestamp,
        // 20 byte
        secret -> Binary,
    }
}
joinable!(totp_secrets -> users (user_id));
allow_tables_to_appear_in_same_query!(totp_secrets, users);

table! {
    friends (user_id_1, user_id_2) {
        user_id_1 -> Int4,
        user_id_2 -> Int4,
    }
}
joinable!(friends -> users (user_id_1));
// joinable!(friends -> users (user_id_2));
allow_tables_to_appear_in_same_query!(friends, users);

table! {
    tag_groups (id) {
        id -> Serial,
        user_id -> Int4,
        name -> Varchar,
        multiple -> Bool,
        required -> Bool
    }
}
joinable!(tag_groups -> users (user_id));
allow_tables_to_appear_in_same_query!(tag_groups, users);

table! {
    tags (id) {
        id -> Serial,
        tag_group_id -> Int4,
        name -> Varchar,
        color -> Binary,
        is_default -> Bool,
    }
}
joinable!(tags -> tag_groups (tag_group_id));
allow_tables_to_appear_in_same_query!(tags, tag_groups);
allow_tables_to_appear_in_same_query!(tags, pictures);
allow_tables_to_appear_in_same_query!(tags, groups);
allow_tables_to_appear_in_same_query!(tags, groups_pictures);
allow_tables_to_appear_in_same_query!(tags, shared_groups);

#[derive(Debug, PartialEq, JsonSchema, Clone, Deserialize, Serialize, diesel_derive_enum::DbEnum)]
#[DbValueStyle = "PascalCase"]
pub enum PictureOrientation {
    Unspecified,
    Normal,
    HorizontalFlip,
    Rotate180,
    VerticalFlip,
    Rotate90HorizontalFlip,
    Rotate90,
    Rotate90VerticalFlip,
    Rotate270,
}

table! {
    use diesel::sql_types::*;
    use super::PictureOrientationMapping;
    pictures (id) {
        id -> BigSerial,
        name -> Varchar,
        comment -> Text,
        owner_id -> Int4,
        author_id -> Int4,
        deleted_date -> Nullable<Timestamp>,
        copied -> Bool,
        creation_date -> Timestamp,
        edition_date -> Timestamp,
        latitude -> Nullable<Decimal>,
        longitude -> Nullable<Decimal>,
        altitude -> Nullable<Int2>,
        orientation -> PictureOrientationMapping,
        width -> Int2,
        height -> Int2,
        camera_brand -> Nullable<Varchar>,
        camera_model -> Nullable<Varchar>,
        focal_length -> Nullable<Decimal>,
        exposure_time_num -> Nullable<Int4>,
        exposure_time_den -> Nullable<Int4>,
        iso_speed -> Nullable<Int4>,
        f_number -> Nullable<Decimal>,
        size_ko -> Int4,
    }
}
joinable!(pictures -> users (owner_id));
//joinable!(pictures -> users (author_id));
allow_tables_to_appear_in_same_query!(pictures, users);

table! {
    pictures_tags (picture_id, tag_id) {
        picture_id -> Int8,
        tag_id -> Int4,
    }
}
joinable!(pictures_tags -> pictures (picture_id));
joinable!(pictures_tags -> tags (tag_id));
allow_tables_to_appear_in_same_query!(pictures_tags, pictures);
allow_tables_to_appear_in_same_query!(pictures_tags, tags);
allow_tables_to_appear_in_same_query!(pictures_tags, tag_groups);
allow_tables_to_appear_in_same_query!(pictures_tags, groups_pictures);
allow_tables_to_appear_in_same_query!(pictures_tags, shared_groups);
allow_tables_to_appear_in_same_query!(pictures_tags, groups);

table! {
    arrangements (id) {
        id -> Serial,
        user_id -> Int4,
        name -> Varchar,
        strong_match_conversion -> Bool,
        strategy -> Nullable<Blob>,
        groups_dependant -> Bool,
        tags_dependant -> Bool,
        exif_dependant -> Bool,
    }
}
joinable!(arrangements -> users (user_id));
allow_tables_to_appear_in_same_query!(arrangements, users);

table! {
    groups (id) {
        id -> Serial,
        arrangement_id -> Int4,
        share_match_conversion -> Bool,
        name -> Varchar,
        to_be_deleted -> Bool,
    }
}
joinable!(groups -> arrangements (arrangement_id));
allow_tables_to_appear_in_same_query!(groups, arrangements);
allow_tables_to_appear_in_same_query!(groups, pictures);

table! {
    groups_pictures (group_id, picture_id) {
        group_id -> Int4,
        picture_id -> Int8,
    }
}
joinable!(groups_pictures -> groups (group_id));
joinable!(groups_pictures -> pictures (picture_id));
allow_tables_to_appear_in_same_query!(groups_pictures, groups);
allow_tables_to_appear_in_same_query!(groups_pictures, pictures);

table! {
    link_share_groups (token) {
        token -> Binary,
        group_id -> Int4,
        permissions -> Int2,
    }
}
joinable!(link_share_groups -> groups (group_id));
allow_tables_to_appear_in_same_query!(link_share_groups, groups);
allow_tables_to_appear_in_same_query!(link_share_groups, groups_pictures);

table! {
    use diesel::sql_types::*;
    shared_groups (user_id, group_id) {
        user_id -> Int4,
        group_id -> Int4,
        permissions -> Int2,
        match_conversion_group_id -> Nullable<Int4>,
        copied -> Bool,
        confirmed -> Bool,
    }
}
joinable!(shared_groups -> groups (group_id));
joinable!(shared_groups -> users (user_id));
//joinable!(shared_groups -> groups (match_conversion_group_id));
allow_tables_to_appear_in_same_query!(shared_groups, groups);
allow_tables_to_appear_in_same_query!(shared_groups, groups_pictures);
allow_tables_to_appear_in_same_query!(shared_groups, pictures);
allow_tables_to_appear_in_same_query!(shared_groups, users);

table! {
    hierarchies (id) {
        id -> Serial,
        user_id -> Int4,
        name -> Varchar,
    }
}
joinable!(hierarchies -> users (user_id));
allow_tables_to_appear_in_same_query!(hierarchies, users);

table! {
    hierarchies_arrangements(hierarchy_id, arrangement_id) {
        hierarchy_id -> Int4,
        arrangement_id -> Int4,
        parent_group_id -> Nullable<Int4>,
    }
}
joinable!(hierarchies_arrangements -> hierarchies (hierarchy_id));
joinable!(hierarchies_arrangements -> arrangements (arrangement_id));
joinable!(hierarchies_arrangements -> groups (parent_group_id));
allow_tables_to_appear_in_same_query!(hierarchies_arrangements, hierarchies);
allow_tables_to_appear_in_same_query!(hierarchies_arrangements, arrangements);
allow_tables_to_appear_in_same_query!(hierarchies_arrangements, groups);

table! {
    duplicate_groups (id) {
        id -> Serial,
        user_id -> Int4,
    }
}
joinable!(duplicate_groups -> users (user_id));
allow_tables_to_appear_in_same_query!(duplicate_groups, users);

table! {
    duplicates (group_id, picture_id) {
        group_id -> Int4,
        picture_id -> Int8,
    }
}
joinable!(duplicates -> duplicate_groups (group_id));
joinable!(duplicates -> pictures (picture_id));
allow_tables_to_appear_in_same_query!(duplicates, duplicate_groups);
allow_tables_to_appear_in_same_query!(duplicates, pictures);

table! {
    ratings (user_id, picture_id) {
        user_id -> Int4,
        picture_id -> Int8,
        rating -> Int2,
    }
}
joinable!(ratings -> users (user_id));
joinable!(ratings -> pictures (picture_id));
allow_tables_to_appear_in_same_query!(ratings, users);
allow_tables_to_appear_in_same_query!(ratings, pictures);
allow_tables_to_appear_in_same_query!(ratings, friends);
