use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::adf::adf_to_text;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub issue_key: String,
    pub author: Option<String>,
    pub body: Option<String>,
    pub created: String,
    pub updated: String,
}

impl Comment {
    pub fn from_value(v: &Value, issue_key: &str) -> Option<Self> {
        Some(Comment {
            id: v.get("id")?.as_str()?.to_string(),
            issue_key: issue_key.to_string(),
            author: v
                .get("author")
                .and_then(|a| a.get("displayName").or_else(|| a.get("emailAddress")))
                .and_then(|n| n.as_str())
                .map(|s| s.to_string()),
            body: v.get("body").and_then(|body| {
                if body.is_object() {
                    Some(adf_to_text(body))
                } else {
                    body.as_str().map(|s| s.to_string())
                }
            }),
            created: v
                .get("created")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string(),
            updated: v
                .get("updated")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string(),
        })
    }
}
