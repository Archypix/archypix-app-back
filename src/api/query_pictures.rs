use crate::database::schema::*;
use crate::rocket::futures::StreamExt;
use diesel::dsl::{exists, not, Filter};
use diesel::query_dsl::methods;
use diesel::ExpressionMethods;
use diesel::QueryDsl;
use rocket::serde::{Deserialize, Serialize};
use rocket_okapi::JsonSchema;
use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PicturesQuery {
    pub filters: Vec<PictureFilter>, // Applies an AND between filters
    pub sorts: Vec<PictureSort>,
    pub page: u32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum PictureFilter {
    Arrangement { invert: bool, ids: Vec<u32> }, // user must be the owner
    Group { invert: bool, ids: Vec<u32> },       // can be a shared group
    Deleted { invert: bool },
    Owned { invert: bool },                   // Only pictures owned by the user
    TagGroup { invert: bool, ids: Vec<u32> }, // user must be the owner
    Tag { invert: bool, ids: Vec<u32> },      // user must be the owner
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum PictureSort {
    CreationDate { ascend: bool },
    EditionDate { ascend: bool },
}
