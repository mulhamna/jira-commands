use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::attachment::Attachment;
use super::field::FieldValue;

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
    /// Attachments on this issue
    pub attachments: Vec<Attachment>,
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

/// Extended create request with dynamic custom fields.
#[derive(Debug, Clone, Default)]
pub struct CreateIssueRequestV2 {
    pub project_key: String,
    pub summary: String,
    /// Markdown description — converted to ADF automatically. Ignored if `description_adf` is set.
    pub description: Option<String>,
    /// Pre-built ADF description — takes priority over `description`.
    pub description_adf: Option<Value>,
    pub issue_type: String,
    pub assignee: Option<String>,
    pub priority: Option<String>,
    /// Labels (plain string list, e.g. ["bug", "backend"])
    pub labels: Vec<String>,
    /// Component names (e.g. ["auth", "api"])
    pub components: Vec<String>,
    /// Parent issue key for sub-tasks (e.g. "PROJ-100")
    pub parent: Option<String>,
    /// Fix version names (e.g. ["v1.0", "v1.1"])
    pub fix_versions: Vec<String>,
    /// Custom field ID → typed value
    pub custom_fields: HashMap<String, FieldValue>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateIssueRequest {
    pub summary: Option<String>,
    /// Markdown description — converted to ADF. Ignored if `description_adf` is set.
    pub description: Option<String>,
    /// Pre-built ADF description — takes priority over `description`.
    pub description_adf: Option<Value>,
    pub assignee: Option<String>,
    pub priority: Option<String>,
    pub status: Option<String>,
    /// Labels (replaces existing labels)
    pub labels: Option<Vec<String>>,
    /// Component names (replaces existing components)
    pub components: Option<Vec<String>>,
    /// Fix version names (replaces existing fix versions)
    pub fix_versions: Option<Vec<String>>,
    /// Parent issue key
    pub parent: Option<String>,
    /// Arbitrary custom field values (field_id → value)
    pub custom_fields: HashMap<String, FieldValue>,
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

        let attachments = fields
            .get("attachment")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(Attachment::from_value).collect())
            .unwrap_or_default();

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
            attachments,
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
