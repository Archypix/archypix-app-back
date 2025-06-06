-- This file should undo anything in `up.sql`
DROP TABLE IF EXISTS "ratings";
DROP TABLE IF EXISTS "duplicates";
DROP TABLE IF EXISTS "duplicate_groups";
DROP TABLE IF EXISTS "hierarchies_arrangements";
DROP TABLE IF EXISTS "hierarchies";
DROP TABLE IF EXISTS "shared_groups";
DROP TABLE IF EXISTS "link_share_groups";
DROP TABLE IF EXISTS "groups_pictures";
DROP TABLE IF EXISTS "groups";
DROP TABLE IF EXISTS "arrangements";
DROP TABLE IF EXISTS "pictures_tags";
DROP TABLE IF EXISTS "pictures";
DROP TABLE IF EXISTS "tags";
DROP TABLE IF EXISTS "tag_groups";
DROP TABLE IF EXISTS "friends";
DROP TABLE IF EXISTS "totp_secrets";
DROP TABLE IF EXISTS "confirmations";
DROP TABLE IF EXISTS "auth_tokens";
DROP TABLE IF EXISTS "users";

DROP TYPE IF EXISTS user_status;
DROP TYPE IF EXISTS confirmation_action;
DROP TYPE IF EXISTS picture_orientation;
