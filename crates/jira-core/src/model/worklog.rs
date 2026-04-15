use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worklog {
    pub id: String,
    pub issue_key: String,
    pub author: Option<String>,
    pub time_spent: String,
    pub time_spent_seconds: u64,
    pub started: String,
    pub comment: Option<String>,
    pub created: String,
    pub updated: String,
}

impl Worklog {
    pub fn from_value(v: &Value, issue_key: &str) -> Option<Self> {
        Some(Worklog {
            id: v.get("id")?.as_str()?.to_string(),
            issue_key: issue_key.to_string(),
            author: v
                .get("author")
                .and_then(|a| a.get("displayName").or_else(|| a.get("emailAddress")))
                .and_then(|n| n.as_str())
                .map(|s| s.to_string()),
            time_spent: v
                .get("timeSpent")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string(),
            time_spent_seconds: v
                .get("timeSpentSeconds")
                .and_then(|t| t.as_u64())
                .unwrap_or(0),
            started: v
                .get("started")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string(),
            comment: v
                .get("comment")
                .and_then(|c| {
                    // ADF comment — extract plain text
                    c.get("content")
                        .and_then(|arr| arr.as_array())
                        .and_then(|nodes| nodes.first())
                        .and_then(|node| node.get("content"))
                        .and_then(|arr| arr.as_array())
                        .and_then(|nodes| nodes.first())
                        .and_then(|node| node.get("text"))
                        .and_then(|t| t.as_str())
                        .map(|s| s.to_string())
                })
                .or_else(|| {
                    // Fallback: plain string comment
                    v.get("comment")
                        .and_then(|c| c.as_str())
                        .map(|s| s.to_string())
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
