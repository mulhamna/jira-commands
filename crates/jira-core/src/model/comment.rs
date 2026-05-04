use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::adf::{adf_to_text, mentioned_account_ids};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub issue_key: String,
    pub author: Option<String>,
    pub author_account_id: Option<String>,
    pub body: Option<String>,
    pub mentions: Vec<String>,
    pub created: String,
    pub updated: String,
}

impl Comment {
    pub fn from_value(v: &Value, issue_key: &str) -> Option<Self> {
        let body_value = v.get("body");
        Some(Comment {
            id: v.get("id")?.as_str()?.to_string(),
            issue_key: issue_key.to_string(),
            author: v
                .get("author")
                .and_then(|a| a.get("displayName").or_else(|| a.get("emailAddress")))
                .and_then(|n| n.as_str())
                .map(|s| s.to_string()),
            author_account_id: v
                .get("author")
                .and_then(|a| a.get("accountId"))
                .and_then(|n| n.as_str())
                .map(|s| s.to_string()),
            body: body_value.and_then(|body| {
                if body.is_object() {
                    Some(adf_to_text(body))
                } else {
                    body.as_str().map(|s| s.to_string())
                }
            }),
            mentions: body_value
                .filter(|body| body.is_object())
                .map(mentioned_account_ids)
                .unwrap_or_default(),
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
