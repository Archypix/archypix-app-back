use crate::grouping::arrangement_strategy::ExifDataTypeValue;
use rocket::serde::{Deserialize, Serialize};
use rocket_okapi::JsonSchema;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ExifValuesGrouping {
    pub data_type: ExifDataTypeValue, // data vec contains the values for each group
    pub values_to_group_id: Vec<i32>, // The value at index i is the id of the group for the value at index i in the data vec
    pub group_names_format: String,
    pub other_group_id: Option<i32>,
}
