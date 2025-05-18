use crate::grouping::arrangement_strategy::ExifDataTypeValue;
use rocket::serde::{Deserialize, Serialize};
use rocket_okapi::JsonSchema;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExifIntervalGrouping {
    /* ... | interval -2 | interval -1 |origin| interval 1 | interval 2 | ...
     * ... | decreasing  | decreasing  |origin| increasing | increasing | ...
     * ... | index 1     | index 0     |origin| index 0    | index 1    | ...
     */
    pub interval: ExifDataTypeValue,    // First value is origin, second is interval
    pub group_ids_increasing: Vec<i32>, // ids of groups for intervals after the origin
    pub group_ids_decreasing: Vec<i32>, // ids of groups for intervals before the origin (in reverse order)
    pub group_names_format: String,     // Datetime format or number format.
}
