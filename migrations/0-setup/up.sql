CREATE TABLE users
(
    CONSTRAINT PK_users PRIMARY KEY (id),
    id               INT UNSIGNED AUTO_INCREMENT,
    name             VARCHAR(32)                                       NOT NULL,
    email     VARCHAR(320) NOT NULL UNIQUE,
    password_hash    CHAR(60)                                          NOT NULL,
    creation_date    DATETIME                                          NOT NULL DEFAULT (UTC_TIMESTAMP()),
    status           ENUM ('unconfirmed', 'normal', 'banned', 'admin') NOT NULL DEFAULT 'unconfirmed',
    tfa_login BOOLEAN      NOT NULL DEFAULT FALSE,
    storage_count_ko BIGINT UNSIGNED                                   NOT NULL DEFAULT 0,
    storage_limit_mo INT UNSIGNED                                      NOT NULL DEFAULT 0
);

CREATE TABLE auth_tokens
(
    CONSTRAINT PK_auth_tokens PRIMARY KEY (user_id, token),
    user_id       INT UNSIGNED NOT NULL,
    token         BINARY(32)   NOT NULL,
    creation_date DATETIME     NOT NULL DEFAULT (UTC_TIMESTAMP()),
    last_use_date DATETIME     NOT NULL DEFAULT (UTC_TIMESTAMP()),
    device_string VARCHAR(128),
    ip_address    VARBINARY(16),
    FOREIGN KEY (user_id) REFERENCES users (id)
);

CREATE TABLE confirmations
(
    CONSTRAINT PK_confirmations PRIMARY KEY (user_id, action, token),
    CONSTRAINT UQ_confirmations UNIQUE (user_id, action, code_token),
    user_id       INT UNSIGNED                                NOT NULL,
    action        ENUM ('signup', 'signin', 'delete_account') NOT NULL,
    used         BOOLEAN    NOT NULL DEFAULT FALSE,
    date          DATETIME                                    NOT NULL DEFAULT (UTC_TIMESTAMP()),
    token         BINARY(16)                                  NOT NULL,
    code_token   BINARY(16) NOT NULL,
    code          SMALLINT UNSIGNED                           NOT NULL,
    code_trials   TINYINT UNSIGNED                            NOT NULL DEFAULT 0,
    redirect_url VARCHAR(255),
    device_string VARCHAR(128),
    ip_address    VARBINARY(16),
    FOREIGN KEY (user_id) REFERENCES users (id)
);

CREATE TABLE totp_secrets
(
    CONSTRAINT PK_totp_secrets PRIMARY KEY (user_id),
    user_id       INT UNSIGNED NOT NULL,
    creation_date DATETIME     NOT NULL DEFAULT (UTC_TIMESTAMP()),
    secret        BINARY(20)   NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users (id)
);

CREATE TABLE friends
(
    CONSTRAINT PK_friends PRIMARY KEY (user_id_1, user_id_2),
    user_id_1 INT UNSIGNED,
    user_id_2 INT UNSIGNED,
    FOREIGN KEY (user_id_1) REFERENCES users (id),
    FOREIGN KEY (user_id_2) REFERENCES users (id)
);

CREATE TABLE tag_groups
(
    CONSTRAINT PK_tag_groups PRIMARY KEY (id),
    id             INT UNSIGNED AUTO_INCREMENT,
    user_id        INT UNSIGNED NOT NULL,
    name           VARCHAR(32)  NOT NULL,
    multiple       BOOLEAN      NOT NULL DEFAULT FALSE,
    required       BOOLEAN      NOT NULL DEFAULT FALSE,
    FOREIGN KEY (user_id) REFERENCES users (id)
);

CREATE TABLE tags
(
    CONSTRAINT PK_tags PRIMARY KEY (id),
    id           INT UNSIGNED AUTO_INCREMENT,
    tag_group_id INT UNSIGNED NOT NULL,
    name         VARCHAR(32)  NOT NULL,
    color        BINARY(3)    NOT NULL DEFAULT 0x000000,
    is_default   BOOLEAN      NOT NULL DEFAULT FALSE,
    FOREIGN KEY (tag_group_id) REFERENCES tag_groups (id)
);

CREATE TABLE pictures
(
    CONSTRAINT PK_photos PRIMARY KEY (id),
    id                BIGINT UNSIGNED AUTO_INCREMENT,
    name    VARCHAR(64) NOT NULL,
    comment TEXT,
    owner_id          INT UNSIGNED                                                                                                                                             NOT NULL,
    author_id         INT UNSIGNED                                                                                                                                             NOT NULL,
    deleted_date      DATETIME                                                                                                                                                          DEFAULT NULL,
    copied            BOOLEAN                                                                                                                                                  NOT NULL,
    creation_date     DATETIME                                                                                                                                                 NOT NULL,
    edition_date      DATETIME                                                                                                                                                 NOT NULL,
    latitude          DECIMAL(8, 6),
    longitude         DECIMAL(9, 6),
    altitude          SMALLINT,
    orientation       ENUM ('Unspecified', 'Normal', 'HorizontalFlip', 'Rotate180', 'VerticalFlip', 'Rotate90HorizontalFlip', 'Rotate90', 'Rotate90VerticalFlip', 'Rotate270') NOT NULL DEFAULT 'Unspecified',
    width             SMALLINT UNSIGNED                                                                                                                                        NOT NULL,
    height            SMALLINT UNSIGNED                                                                                                                                        NOT NULL,
    camera_brand      VARCHAR(32),
    camera_model      VARCHAR(32),
    focal_length      DECIMAL(6, 2),
    exposure_time_num INT UNSIGNED,
    exposure_time_den INT UNSIGNED,
    iso_speed         INT UNSIGNED,
    f_number          DECIMAL(4, 1),
    FOREIGN KEY (owner_id) REFERENCES users (id),
    FOREIGN KEY (author_id) REFERENCES users (id)
);

CREATE TABLE pictures_tags
(
    CONSTRAINT PK_pictures_tags PRIMARY KEY (picture_id, tag_id),
    picture_id BIGINT UNSIGNED,
    tag_id     INT UNSIGNED,
    FOREIGN KEY (picture_id) REFERENCES pictures (id),
    FOREIGN KEY (tag_id) REFERENCES tags (id)
);

CREATE TABLE arrangements
(
    CONSTRAINT PK_arrangements PRIMARY KEY (id),
    id                      INT UNSIGNED AUTO_INCREMENT NOT NULL,
    user_id                 INT UNSIGNED                NOT NULL,
    name                    VARCHAR(32)                 NOT NULL,
    strong_match_conversion BOOLEAN                     NOT NULL DEFAULT FALSE,
    strategy         BLOB COMMENT 'Null if manual grouping',
    groups_dependant BOOLEAN NOT NULL DEFAULT FALSE COMMENT 'True if the strategy filters or groups in function of the pictures other’s groups presence',
    tags_dependant   BOOLEAN NOT NULL DEFAULT FALSE COMMENT 'True if the strategy filters or groups in function of the pictures tags',
    exif_dependant   BOOLEAN NOT NULL DEFAULT FALSE COMMENT 'True if the strategy filters or groups in function of the pictures exif',
    FOREIGN KEY (user_id) REFERENCES users (id)
);

CREATE TABLE `groups`
(
    CONSTRAINT PK_groups PRIMARY KEY (id),
    id                     INT UNSIGNED AUTO_INCREMENT NOT NULL,
    arrangement_id         INT UNSIGNED                NOT NULL,
    share_match_conversion BOOLEAN                     NOT NULL DEFAULT FALSE,
    name                   VARCHAR(32)                 NOT NULL,
    FOREIGN KEY (arrangement_id) REFERENCES arrangements (id)
);

CREATE TABLE groups_pictures
(
    CONSTRAINT PK_groups_pictures PRIMARY KEY (group_id, picture_id),
    group_id   INT UNSIGNED    NOT NULL,
    picture_id BIGINT UNSIGNED NOT NULL,
    FOREIGN KEY (group_id) REFERENCES `groups` (id),
    FOREIGN KEY (picture_id) REFERENCES pictures (id)
);

CREATE TABLE link_share_groups
(
    CONSTRAINT PK_link_share_groups PRIMARY KEY (token),
    token       BINARY(16)   NOT NULL,
    group_id    INT UNSIGNED NOT NULL,
    permissions TINYINT      NOT NULL DEFAULT 0, -- Bits : Add pictures / Share back / Edit exif / Edit picture / Delete
    FOREIGN KEY (group_id) REFERENCES `groups` (id)
);

CREATE TABLE shared_groups
(
    CONSTRAINT PK_shared_groups PRIMARY KEY (user_id, group_id),
    user_id                   INT UNSIGNED NOT NULL,
    group_id                  INT UNSIGNED NOT NULL,
    permissions               TINYINT      NOT NULL DEFAULT 0, -- Bits : Add pictures / Share back / Edit exif / Edit picture / Delete
    match_conversion_group_id INT UNSIGNED          DEFAULT NULL,
    copied                    BOOLEAN      NOT NULL DEFAULT FALSE,
    confirmed                 BOOLEAN      NOT NULL DEFAULT FALSE,
    FOREIGN KEY (user_id) REFERENCES users (id),
    FOREIGN KEY (group_id) REFERENCES `groups` (id),
    FOREIGN KEY (match_conversion_group_id) REFERENCES `groups` (id)
);

CREATE TABLE hierarchies
(
    CONSTRAINT PK_hierarchy PRIMARY KEY (id),
    id      INT UNSIGNED AUTO_INCREMENT NOT NULL,
    user_id INT UNSIGNED                NOT NULL,
    name    VARCHAR(32)                 NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users (id)
);

CREATE TABLE hierarchies_arrangements
(
    CONSTRAINT PK_hierarchy_groups PRIMARY KEY (hierarchy_id, arrangements_id),
    hierarchy_id    INT UNSIGNED NOT NULL,
    arrangements_id INT UNSIGNED NOT NULL,
    parent_group_id INT UNSIGNED,
    FOREIGN KEY (hierarchy_id) REFERENCES hierarchies (id),
    FOREIGN KEY (arrangements_id) REFERENCES arrangements (id),
    FOREIGN KEY (parent_group_id) REFERENCES `groups` (id)
);

CREATE TABLE duplicate_groups
(
    CONSTRAINT PK_duplicate_groups PRIMARY KEY (id),
    id      INT UNSIGNED AUTO_INCREMENT NOT NULL,
    user_id INT UNSIGNED                NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users (id)
);

CREATE TABLE duplicates
(
    CONSTRAINT PK_duplicates PRIMARY KEY (group_id, picture_id),
    group_id   INT UNSIGNED    NOT NULL,
    picture_id BIGINT UNSIGNED NOT NULL,
    FOREIGN KEY (group_id) REFERENCES duplicate_groups (id),
    FOREIGN KEY (picture_id) REFERENCES pictures (id)
);

CREATE TABLE ratings
(
    CONSTRAINT PK_ratings PRIMARY KEY (user_id, picture_id),
    user_id    INT UNSIGNED    NOT NULL,
    picture_id BIGINT UNSIGNED NOT NULL,
    rating     TINYINT         NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users (id),
    FOREIGN KEY (picture_id) REFERENCES pictures (id)
);
