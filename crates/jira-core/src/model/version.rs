use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectVersion {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub released: bool,
    #[serde(default)]
    pub release_date: Option<String>,
    #[serde(default)]
    pub start_date: Option<String>,
}
