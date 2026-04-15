use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: String,
    pub key: String,
    pub summary: String,
    /// ADF (Atlassian Document Format) JSON
    pub description: Option<Value>,
    pub status: String,
    pub assignee: Option<String>,
    pub reporter: Option<String>,
    pub priority: Option<String>,
    pub issue_type: String,
    pub project_key: String,
    pub created: String,
    pub updated: String,
    /// Raw fields map for custom fields
    pub fields: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateIssueRequest {
    pub project_key: String,
    pub summary: String,
    pub description: Option<String>,
    pub issue_type: String,
    pub assignee: Option<String>,
    pub priority: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateIssueRequest {
    pub summary: Option<String>,
    pub description: Option<String>,
    pub assignee: Option<String>,
    pub priority: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub issues: Vec<Issue>,
    pub next_page_token: Option<String>,
    pub total: Option<u64>,
}

/// Raw Jira API issue response — used internally for deserialization
#[derive(Debug, Deserialize)]
pub(crate) struct RawIssue {
    pub id: String,
    pub key: String,
    pub fields: Value,
}

impl RawIssue {
    pub fn into_issue(self) -> Issue {
        let fields = &self.fields;

        let summary = fields
            .get("summary")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let description = fields.get("description").cloned();

        let status = fields
            .get("status")
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let assignee = fields
            .get("assignee")
            .and_then(|v| v.get("emailAddress").or_else(|| v.get("displayName")))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let reporter = fields
            .get("reporter")
            .and_then(|v| v.get("emailAddress").or_else(|| v.get("displayName")))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let priority = fields
            .get("priority")
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let issue_type = fields
            .get("issuetype")
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let project_key = fields
            .get("project")
            .and_then(|v| v.get("key"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let created = fields
            .get("created")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let updated = fields
            .get("updated")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Issue {
            id: self.id,
            key: self.key,
            summary,
            description,
            status,
            assignee,
            reporter,
            priority,
            issue_type,
            project_key,
            created,
            updated,
            fields: self.fields,
        }
    }
}

/// Raw search response from Jira API
#[derive(Debug, Deserialize)]
pub(crate) struct RawSearchResponse {
    pub issues: Vec<RawIssue>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
    pub total: Option<u64>,
}
