use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub id: String,
    pub name: String,
    pub field_type: String,
    pub required: bool,
    pub schema: Option<Value>,
}
