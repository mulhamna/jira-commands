use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueLinkType {
    pub id: String,
    pub name: String,
    pub inward: String,
    pub outward: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedIssue {
    pub id: String,
    pub key: String,
    pub summary: String,
    pub status: String,
    pub priority: Option<String>,
    pub issue_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueLink {
    pub id: String,
    #[serde(rename = "type")]
    pub link_type: IssueLinkType,
    #[serde(rename = "inwardIssue")]
    pub inward_issue: Option<LinkedIssue>,
    #[serde(rename = "outwardIssue")]
    pub outward_issue: Option<LinkedIssue>,
}

impl LinkedIssue {
    pub fn from_value(value: &Value) -> Option<Self> {
        let id = value.get("id")?.as_str()?.to_string();
        let key = value.get("key")?.as_str()?.to_string();
        let fields = value.get("fields")?;

        let summary = fields.get("summary")?.as_str()?.to_string();
        let status = fields
            .get("status")
            .and_then(|s| s.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();
        let priority = fields
            .get("priority")
            .and_then(|p| p.get("name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let issue_type = fields
            .get("issuetype")
            .and_then(|i| i.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        Some(Self {
            id,
            key,
            summary,
            status,
            priority,
            issue_type,
        })
    }
}

impl IssueLink {
    pub fn from_value(value: &Value) -> Option<Self> {
        let id = value.get("id")?.as_str()?.to_string();
        let type_val = value.get("type")?;
        let link_type = IssueLinkType {
            id: type_val.get("id")?.as_str()?.to_string(),
            name: type_val.get("name")?.as_str()?.to_string(),
            inward: type_val.get("inward")?.as_str()?.to_string(),
            outward: type_val.get("outward")?.as_str()?.to_string(),
        };

        let inward_issue = value.get("inwardIssue").and_then(LinkedIssue::from_value);
        let outward_issue = value.get("outwardIssue").and_then(LinkedIssue::from_value);

        Some(Self {
            id,
            link_type,
            inward_issue,
            outward_issue,
        })
    }
}
