-- Your SQL goes here

CREATE TYPE user_status AS ENUM ('unconfirmed', 'normal', 'banned', 'admin');
CREATE TABLE "users"
(
    "id"               SERIAL       NOT NULL PRIMARY KEY,
    "name"             VARCHAR(32)  NOT NULL,
    "email"            VARCHAR(320) NOT NULL,
    "password_hash"    CHAR(60)     NOT NULL,
    "creation_date"    TIMESTAMP    NOT NULL DEFAULT timezone('utc', now()),
    "status"           user_status  NOT NULL DEFAULT 'unconfirmed',
    "tfa_login"        BOOL         NOT NULL DEFAULT FALSE,
    "storage_count_ko" INT8         NOT NULL DEFAULT 0,
    "storage_limit_ko" INT8         NOT NULL DEFAULT 0
);

CREATE TABLE "auth_tokens"
(
    "user_id"       SERIAL    NOT NULL,
    "token"         BYTEA     NOT NULL CHECK (octet_length("token") = 32) NOT NULL,
    "creation_date" TIMESTAMP NOT NULL DEFAULT timezone('utc', now()),
    "last_use_date" TIMESTAMP NOT NULL DEFAULT timezone('utc', now()),
    "device_string" VARCHAR(128),
    "ip_address"    INET,
    PRIMARY KEY ("user_id", "token"),
    FOREIGN KEY ("user_id") REFERENCES "users" ("id")
);

CREATE TYPE confirmation_action AS ENUM ('signup', 'signin', 'delete_account');
CREATE TABLE "confirmations"
(
    "user_id"       SERIAL              NOT NULL,
    "action"        confirmation_action NOT NULL,
    "used"          BOOL                NOT NULL DEFAULT FALSE,
    "date"          TIMESTAMP           NOT NULL DEFAULT timezone('utc', now()),
    "token"         BYTEA               NOT NULL CHECK (octet_length("token") = 16) NOT NULL,
    "code_token"    BYTEA               NOT NULL CHECK (octet_length("code_token") = 16) NOT NULL,
    "code"          INT2                NOT NULL,
    "code_trials"   INT2                NOT NULL DEFAULT 0,
    "redirect_url"  VARCHAR(255),
    "device_string" VARCHAR(128),
    "ip_address"    INET,
    PRIMARY KEY ("user_id", "action", "token"),
    UNIQUE ("user_id", "action", "code_token"),
    FOREIGN KEY ("user_id") REFERENCES "users" ("id")
);

CREATE TABLE "totp_secrets"
(
    "user_id"       SERIAL    NOT NULL PRIMARY KEY,
    "creation_date" TIMESTAMP NOT NULL DEFAULT timezone('utc', now()),
    "secret"        BYTEA     NOT NULL CHECK (octet_length("secret") = 20) NOT NULL,
    FOREIGN KEY ("user_id") REFERENCES "users" ("id")
);

CREATE TABLE "friends"
(
    "user_id_1" INT4 NOT NULL,
    "user_id_2" INT4 NOT NULL,
    PRIMARY KEY ("user_id_1", "user_id_2"),
    FOREIGN KEY ("user_id_1") REFERENCES "users" ("id"),
    FOREIGN KEY ("user_id_2") REFERENCES "users" ("id")
);

CREATE TABLE "tag_groups"
(
    "id"       SERIAL      NOT NULL PRIMARY KEY,
    "user_id"  INT4        NOT NULL,
    "name"     VARCHAR(32) NOT NULL,
    "multiple" BOOL        NOT NULL,
    "required" BOOL        NOT NULL,
    FOREIGN KEY ("user_id") REFERENCES "users" ("id")
);

CREATE TABLE "tags"
(
    "id"           SERIAL      NOT NULL PRIMARY KEY,
    "tag_group_id" INT4        NOT NULL,
    "name"         VARCHAR(32) NOT NULL,
    "color"        BYTEA       NOT NULL CHECK (octet_length("color") = 3),
    "is_default"   BOOL        NOT NULL,
    FOREIGN KEY ("tag_group_id") REFERENCES "tag_groups" ("id")
);

CREATE TYPE picture_orientation AS ENUM ('Unspecified', 'Normal', 'HorizontalFlip', 'Rotate180', 'VerticalFlip', 'Rotate90HorizontalFlip', 'Rotate90', 'Rotate90VerticalFlip', 'Rotate270');
CREATE TABLE "pictures"
(
    "id"                BIGSERIAL           NOT NULL PRIMARY KEY,
    "name"              VARCHAR(64)         NOT NULL,
    "comment"           TEXT                NOT NULL,
    "owner_id"          INT4                NOT NULL,
    "author_id"         INT4                NOT NULL,
    "deleted_date"      TIMESTAMP,
    "copied"            BOOL                NOT NULL,
    "creation_date"     TIMESTAMP           NOT NULL,
    "edition_date"      TIMESTAMP           NOT NULL,
    "latitude"          DECIMAL,
    "longitude"         DECIMAL,
    "altitude"          INT2,
    "orientation"       picture_orientation NOT NULL DEFAULT 'Unspecified',
    "width"             INT2                NOT NULL,
    "height"            INT2                NOT NULL,
    "camera_brand"      VARCHAR(32),
    "camera_model"      VARCHAR(32),
    "focal_length"      DECIMAL,
    "exposure_time_num" INT4,
    "exposure_time_den" INT4,
    "iso_speed"         INT4,
    "f_number"          DECIMAL(4, 1),
    "size_ko"           INT4                NOT NULL,
    "blurhash" VARCHAR(28),
    FOREIGN KEY ("author_id") REFERENCES "users" ("id"),
    FOREIGN KEY ("owner_id") REFERENCES "users" ("id")
);

CREATE TABLE "pictures_tags"
(
    "picture_id" INT8 NOT NULL,
    "tag_id"     INT4 NOT NULL,
    PRIMARY KEY ("picture_id", "tag_id"),
    FOREIGN KEY ("picture_id") REFERENCES "pictures" ("id"),
    FOREIGN KEY ("tag_id") REFERENCES "tags" ("id")
);

CREATE TABLE "arrangements"
(
    "id"                      SERIAL      NOT NULL PRIMARY KEY,
    "user_id"                 INT4        NOT NULL,
    "name"                    VARCHAR(32) NOT NULL,
    "strong_match_conversion" BOOL        NOT NULL,
    "strategy"                BYTEA,
    "groups_dependant"        BOOL        NOT NULL,
    "tags_dependant"          BOOL        NOT NULL,
    "exif_dependant"          BOOL        NOT NULL,
    FOREIGN KEY ("user_id") REFERENCES "users" ("id")
);

CREATE TABLE "groups"
(
    "id"                     SERIAL      NOT NULL PRIMARY KEY,
    "arrangement_id"         INT4        NOT NULL,
    "share_match_conversion" BOOL        NOT NULL,
    "name"                   VARCHAR(32) NOT NULL,
    "to_be_deleted" BOOL NOT NULL DEFAULT FALSE,
    FOREIGN KEY ("arrangement_id") REFERENCES "arrangements" ("id")
);

CREATE TABLE "groups_pictures"
(
    "group_id"   INT4 NOT NULL,
    "picture_id" INT8 NOT NULL,
    PRIMARY KEY ("group_id", "picture_id"),
    FOREIGN KEY ("group_id") REFERENCES "groups" ("id"),
    FOREIGN KEY ("picture_id") REFERENCES "pictures" ("id")
);

CREATE TABLE "link_share_groups"
(
    "token"       BYTEA CHECK (octet_length("token") = 16) NOT NULL PRIMARY KEY,
    "group_id"    INT4                                     NOT NULL,
    "permissions" INT2                                     NOT NULL,
    FOREIGN KEY ("group_id") REFERENCES "groups" ("id")
);

CREATE TABLE "shared_groups"
(
    "user_id"                   INT4 NOT NULL,
    "group_id"                  INT4 NOT NULL,
    "permissions"               INT2 NOT NULL, -- Bits : Add pictures / Share back / Edit exif / Edit picture / Delete
    "match_conversion_group_id" INT4          DEFAULT NULL,
    "copied"                    BOOL NOT NULL DEFAULT FALSE,
    "confirmed"                 BOOL NOT NULL DEFAULT FALSE,
    PRIMARY KEY ("user_id", "group_id"),
    FOREIGN KEY ("user_id") REFERENCES "users" ("id"),
    FOREIGN KEY ("group_id") REFERENCES "groups" ("id"),
    FOREIGN KEY ("match_conversion_group_id") REFERENCES "groups" ("id")
);

CREATE TABLE "hierarchies"
(
    "id"      SERIAL      NOT NULL PRIMARY KEY,
    "user_id" INT4        NOT NULL,
    "name"    VARCHAR(32) NOT NULL,
    FOREIGN KEY ("user_id") REFERENCES "users" ("id")
);

CREATE TABLE "hierarchies_arrangements"
(
    "hierarchy_id"    INT4 NOT NULL,
    "arrangement_id"  INT4 NOT NULL,
    "parent_group_id" INT4,
    PRIMARY KEY ("hierarchy_id", "arrangement_id"),
    FOREIGN KEY ("hierarchy_id") REFERENCES "hierarchies" ("id"),
    FOREIGN KEY ("arrangement_id") REFERENCES "arrangements" ("id"),
    FOREIGN KEY ("parent_group_id") REFERENCES "groups" ("id")
);

CREATE TABLE "duplicate_groups"
(
    "id"      SERIAL NOT NULL PRIMARY KEY,
    "user_id" INT4   NOT NULL,
    FOREIGN KEY ("user_id") REFERENCES "users" ("id")
);

CREATE TABLE "duplicates"
(
    "group_id"   INT4 NOT NULL,
    "picture_id" INT8 NOT NULL,
    PRIMARY KEY ("group_id", "picture_id"),
    FOREIGN KEY ("group_id") REFERENCES "duplicate_groups" ("id"),
    FOREIGN KEY ("picture_id") REFERENCES "pictures" ("id")
);

CREATE TABLE "ratings"
(
    "user_id"    INT4 NOT NULL,
    "picture_id" INT8 NOT NULL,
    "rating"     INT2 NOT NULL,
    PRIMARY KEY ("user_id", "picture_id"),
    FOREIGN KEY ("user_id") REFERENCES "users" ("id"),
    FOREIGN KEY ("picture_id") REFERENCES "pictures" ("id")
);
