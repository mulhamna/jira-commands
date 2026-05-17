use std::collections::BTreeMap;

use serde_json::Value;

/// An issue workflow transition, returned by `GET /rest/api/3/issue/{key}/transitions`.
///
/// Unknown fields from the Jira response are captured in `extra` so that
/// callers re-serializing the transition (e.g. the MCP server) preserve
/// the full payload shape.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Transition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub to: Option<TransitionStatus>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TransitionStatus {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
