use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sprint {
    pub id: u64,
    pub name: String,
    pub state: String,
    pub board_id: Option<u64>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}
