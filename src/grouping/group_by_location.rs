use rocket::serde::{Deserialize, Serialize};
use rocket_okapi::JsonSchema;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LocationGrouping {
    pub clusters_ids: Vec<i32>, // Ids of the groups for each cluster
    pub is_date_ordered: bool,
    pub sharpness: u32,
}
