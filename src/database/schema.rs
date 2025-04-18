use diesel::sql_types::{Binary, Nullable, SqlType, VarChar};
use diesel::{allow_tables_to_appear_in_same_query, joinable, table};
use diesel_derives::define_sql_function;
use rocket_okapi::JsonSchema;
use serde::{Deserialize, Serialize};

define_sql_function! { fn last_insert_id() -> Unsigned<Bigint> }
define_sql_function! { fn inet6_ntoa(ip: Nullable<Binary>) -> Nullable<VarChar> }
define_sql_function! { fn inet6_aton(ip: Nullable<VarChar>) -> Nullable<Varbinary> }
define_sql_function! { fn utc_timestamp() -> Datetime }

#[derive(JsonSchema, Debug, PartialEq, Serialize, diesel_derive_enum::DbEnum)]
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
        id -> Unsigned<Integer>,
        name -> Varchar,
        email -> Varchar,
        // 60 character
        password_hash -> Char,
        creation_date -> Datetime,
        status -> UserStatusMapping,
        tfa_login -> Bool,
        storage_count_ko -> Unsigned<BigInt>,
        storage_limit_mo -> Unsigned<Integer>,
    }
}

table! {
    auth_tokens (user_id, token) {
        user_id -> Unsigned<Integer>,
        token -> Binary,
        creation_date -> Datetime,
        last_use_date -> Datetime,
        device_string -> Nullable<Varchar>,
        ip_address -> Nullable<Varbinary>,
    }
}
joinable!(auth_tokens -> users (user_id));
allow_tables_to_appear_in_same_query!(auth_tokens, users);

#[derive(JsonSchema, Debug, PartialEq, diesel_derive_enum::DbEnum, Deserialize, Serialize)]
pub enum ConfirmationAction {
    Signup,
    Signin,
    DeleteAccount,
}
table! {
    use diesel::sql_types::*;
    use super::ConfirmationActionMapping;
    confirmations (user_id, action, token) {
        user_id -> Unsigned<Integer>,
        // 16 byte
        action -> ConfirmationActionMapping,
        used -> Bool,
        date -> Datetime,
        token -> Binary,
        code_token -> Binary,
        code -> Unsigned<Smallint>,
        code_trials -> Unsigned<Tinyint>,
        redirect_url -> Nullable<Varchar>,
        device_string -> Nullable<Varchar>,
        ip_address -> Nullable<Varbinary>,
    }
}
joinable!(confirmations -> users (user_id));
allow_tables_to_appear_in_same_query!(confirmations, users);

table! {
    totp_secrets (user_id) {
        user_id -> Unsigned<Integer>,
        creation_date -> Datetime,
        // 20 byte
        secret -> Binary,
    }
}
joinable!(totp_secrets -> users (user_id));
allow_tables_to_appear_in_same_query!(totp_secrets, users);

table! {
    friends (user_id_1, user_id_2) {
        user_id_1 -> Unsigned<Integer>,
        user_id_2 -> Unsigned<Integer>,
    }
}
joinable!(friends -> users (user_id_1));
// joinable!(friends -> users (user_id_2));
allow_tables_to_appear_in_same_query!(friends, users);

table! {
    tag_groups (id) {
        id -> Unsigned<Integer>,
        user_id -> Unsigned<Integer>,
        name -> Varchar,
        multiple -> Bool,
        required -> Bool
    }
}
joinable!(tag_groups -> users (user_id));
allow_tables_to_appear_in_same_query!(tag_groups, users);

table! {
    tags (id) {
        id -> Unsigned<Integer>,
        tag_group_id -> Unsigned<Integer>,
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

#[derive(Debug, PartialEq, JsonSchema, diesel_derive_enum::DbEnum, Clone, Deserialize, Serialize)]
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
        id -> Unsigned<BigInt>,
        name -> Varchar,
        comment -> Text,
        owner_id -> Unsigned<Integer>,
        author_id -> Unsigned<Integer>,
        deleted_date -> Nullable<Datetime>,
        copied -> Bool,
        creation_date -> Datetime,
        edition_date -> Datetime,
        latitude -> Nullable<Decimal>,
        longitude -> Nullable<Decimal>,
        altitude -> Nullable<Unsigned<SmallInt>>,
        orientation -> PictureOrientationMapping,
        width -> Unsigned<SmallInt>,
        height -> Unsigned<SmallInt>,
        camera_brand -> Nullable<Varchar>,
        camera_model -> Nullable<Varchar>,
        focal_length -> Nullable<Decimal>,
        exposure_time_num -> Nullable<Unsigned<Integer>>,
        exposure_time_den -> Nullable<Unsigned<Integer>>,
        iso_speed -> Nullable<Unsigned<Integer>>,
        f_number -> Nullable<Decimal>,
    }
}
joinable!(pictures -> users (owner_id));
//joinable!(pictures -> users (author_id));
allow_tables_to_appear_in_same_query!(pictures, users);

table! {
    pictures_tags (picture_id, tag_id) {
        picture_id -> Unsigned<BigInt>,
        tag_id -> Unsigned<Integer>,
    }
}
joinable!(pictures_tags -> pictures (picture_id));
joinable!(pictures_tags -> tags (tag_id));
allow_tables_to_appear_in_same_query!(pictures_tags, pictures);
allow_tables_to_appear_in_same_query!(pictures_tags, tags);
allow_tables_to_appear_in_same_query!(pictures_tags, groups_pictures);
allow_tables_to_appear_in_same_query!(pictures_tags, shared_groups);
allow_tables_to_appear_in_same_query!(pictures_tags, groups);

table! {
    arrangements (id) {
        id -> Unsigned<Integer>,
        user_id -> Unsigned<Integer>,
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
        id -> Unsigned<Integer>,
        arrangement_id -> Unsigned<Integer>,
        share_match_conversion -> Bool,
        name -> Varchar,
    }
}
joinable!(groups -> arrangements (arrangement_id));
allow_tables_to_appear_in_same_query!(groups, arrangements);
allow_tables_to_appear_in_same_query!(groups, pictures);

table! {
    groups_pictures (group_id, picture_id) {
        group_id -> Unsigned<Integer>,
        picture_id -> Unsigned<BigInt>,
    }
}
joinable!(groups_pictures -> groups (group_id));
joinable!(groups_pictures -> pictures (picture_id));
allow_tables_to_appear_in_same_query!(groups_pictures, groups);
allow_tables_to_appear_in_same_query!(groups_pictures, pictures);

table! {
    link_share_groups (token) {
        token -> Binary,
        group_id -> Unsigned<Integer>,
        permissions -> Unsigned<TinyInt>,
    }
}
joinable!(link_share_groups -> groups (group_id));
allow_tables_to_appear_in_same_query!(link_share_groups, groups);
allow_tables_to_appear_in_same_query!(link_share_groups, groups_pictures);

table! {
    use diesel::sql_types::*;
    shared_groups (user_id, group_id) {
        user_id -> Unsigned<Integer>,
        group_id -> Unsigned<Integer>,
        permissions -> Unsigned<TinyInt>,
        match_conversion_group_id -> Nullable<Unsigned<Integer>>,
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
        id -> Unsigned<Integer>,
        user_id -> Unsigned<Integer>,
        name -> Varchar,
    }
}
joinable!(hierarchies -> users (user_id));
allow_tables_to_appear_in_same_query!(hierarchies, users);

table! {
    hierarchies_arrangements(hierarchy_id, arrangement_id) {
        hierarchy_id -> Unsigned<Integer>,
        arrangement_id -> Unsigned<Integer>,
        parent_group_id -> Unsigned<Integer>,
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
        id -> Unsigned<Integer>,
        user_id -> Unsigned<Integer>,
    }
}
joinable!(duplicate_groups -> users (user_id));
allow_tables_to_appear_in_same_query!(duplicate_groups, users);

table! {
    duplicates (group_id, picture_id) {
        group_id -> Unsigned<Integer>,
        picture_id -> Unsigned<BigInt>,
    }
}
joinable!(duplicates -> duplicate_groups (group_id));
joinable!(duplicates -> pictures (picture_id));
allow_tables_to_appear_in_same_query!(duplicates, duplicate_groups);
allow_tables_to_appear_in_same_query!(duplicates, pictures);

table! {
    ratings (user_id, picture_id) {
        user_id -> Unsigned<Integer>,
        picture_id -> Unsigned<BigInt>,
        rating -> Unsigned<TinyInt>,
    }
}
joinable!(ratings -> users (user_id));
joinable!(ratings -> pictures (picture_id));
allow_tables_to_appear_in_same_query!(ratings, users);
allow_tables_to_appear_in_same_query!(ratings, pictures);
