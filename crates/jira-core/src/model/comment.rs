use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::rich_text::{mentioned_users, rich_text_to_text};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub issue_key: String,
    pub author: Option<String>,
    /// Cloud accountId; Data Center falls back to the login name, then the user key.
    pub author_account_id: Option<String>,
    /// API v2 wiki markup is preserved; API v3 ADF is converted to readable text.
    pub body: Option<String>,
    /// Mentioned users: ADF mention IDs on API v3, `[~...]` wiki link targets on API v2.
    pub mentions: Vec<String>,
    pub created: String,
    pub updated: String,
}

impl Comment {
    pub fn from_value(v: &Value, issue_key: &str) -> Option<Self> {
        let body_value = v.get("body");
        let author = v.get("author");
        Some(Comment {
            id: v.get("id")?.as_str()?.to_string(),
            issue_key: issue_key.to_string(),
            author: author
                .and_then(|a| a.get("displayName").or_else(|| a.get("emailAddress")))
                .and_then(|n| n.as_str())
                .map(|s| s.to_string()),
            author_account_id: author.and_then(|a| {
                ["accountId", "name", "key"]
                    .into_iter()
                    .find_map(|field| a.get(field).and_then(Value::as_str))
                    .map(str::to_owned)
            }),
            body: body_value.filter(|body| !body.is_null()).map(|body| {
                body.as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| rich_text_to_text(body))
            }),
            mentions: body_value.map(mentioned_users).unwrap_or_default(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn data_center_wiki_comment_keeps_body_and_reads_mentions_and_author() {
        let comment = Comment::from_value(
            &json!({
                "id": "10",
                "author": {"name": "jdoe", "key": "JIRAUSER10100", "displayName": "J Doe"},
                "body": "h3. Ping\n[~alice] please check",
                "created": "2026-11-01T10:00:00.000+0000"
            }),
            "PROJ-1",
        )
        .expect("comment");
        assert_eq!(
            comment.body.as_deref(),
            Some("h3. Ping\n[~alice] please check")
        );
        assert_eq!(comment.mentions, ["alice"]);
        assert_eq!(comment.author_account_id.as_deref(), Some("jdoe"));
    }

    #[test]
    fn cloud_adf_comment_prefers_account_id() {
        let comment = Comment::from_value(
            &json!({
                "id": "11",
                "author": {"accountId": "acc-1", "name": "legacy"},
                "body": {"type": "doc", "version": 1, "content": [{"type": "paragraph",
                    "content": [{"type": "mention", "attrs": {"id": "acc-2"}}]}]}
            }),
            "PROJ-1",
        )
        .expect("comment");
        assert_eq!(comment.author_account_id.as_deref(), Some("acc-1"));
        assert_eq!(comment.mentions, ["acc-2"]);
    }
}
