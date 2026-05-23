use std::collections::BTreeMap;

use rmcp::schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

fn any_json_object(_: &mut SchemaGenerator) -> Schema {
    serde_json::from_value(json!({
        "type": "object",
        "additionalProperties": {}
    }))
    .expect("static schema literal must deserialize")
}

fn any_json_object_map(_: &mut SchemaGenerator) -> Schema {
    serde_json::from_value(json!({
        "type": "object",
        "additionalProperties": {}
    }))
    .expect("static schema literal must deserialize")
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolResponse {
    #[schemars(schema_with = "any_json_object")]
    pub result: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AuthSetCredentialsArgs {
    pub profile: Option<String>,
    pub url: Option<String>,
    pub email: Option<String>,
    pub token: Option<String>,
    pub project: Option<String>,
    pub timeout_secs: Option<u64>,
    pub deployment: Option<String>,
    pub auth_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IssueListArgs {
    pub project_key: Option<String>,
    pub jql: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IssueKeyArgs {
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IssueTypesListArgs {
    pub project_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IssueFieldsArgs {
    pub project_key: String,
    pub issue_type_id: Option<String>,
    pub required_only: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IssueCreateArgs {
    pub project_key: String,
    pub summary: String,
    pub issue_type: String,
    pub description: Option<String>,
    #[serde(default)]
    #[schemars(schema_with = "any_json_object")]
    pub description_adf: Option<Value>,
    pub assignee: Option<String>,
    pub priority: Option<String>,
    pub labels: Option<Vec<String>>,
    pub components: Option<Vec<String>>,
    pub parent: Option<String>,
    pub fix_versions: Option<Vec<String>>,
    #[serde(default)]
    #[schemars(schema_with = "any_json_object_map")]
    pub custom_fields: Option<BTreeMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BulkCreateEntry {
    pub project_key: String,
    pub summary: String,
    pub issue_type: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    #[schemars(schema_with = "any_json_object")]
    pub description_adf: Option<Value>,
    pub assignee: Option<String>,
    pub priority: Option<String>,
    pub labels: Option<Vec<String>>,
    pub components: Option<Vec<String>>,
    pub parent: Option<String>,
    pub fix_versions: Option<Vec<String>>,
    #[serde(default)]
    #[schemars(schema_with = "any_json_object_map")]
    pub custom_fields: Option<BTreeMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BulkCreateArgs {
    pub issues: Vec<BulkCreateEntry>,
    pub confirm: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IssueUpdateArgs {
    pub key: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    #[schemars(schema_with = "any_json_object")]
    pub description_adf: Option<Value>,
    pub assignee: Option<String>,
    pub priority: Option<String>,
    pub labels: Option<Vec<String>>,
    pub components: Option<Vec<String>>,
    pub parent: Option<String>,
    pub fix_versions: Option<Vec<String>>,
    #[serde(default)]
    #[schemars(schema_with = "any_json_object_map")]
    pub custom_fields: Option<BTreeMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IssueDeleteArgs {
    pub key: String,
    pub confirm: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IssueCloneArgs {
    pub key: String,
    pub project_key: Option<String>,
    pub summary: Option<String>,
    pub assignee: Option<String>,
    pub move_issue: Option<bool>,
    pub confirm: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IssueTransitionArgs {
    pub key: String,
    pub transition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IssueLinkAddArgs {
    pub outward_key: String,
    pub inward_key: String,
    pub link_type: String,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IssueLinkDeleteArgs {
    pub id: String,
    pub confirm: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum AttachmentInput {
    Path {
        path: String,
    },
    Inline {
        filename: String,
        media_type: Option<String>,
        base64: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IssueAttachArgs {
    pub key: String,
    pub attachments: Vec<AttachmentInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CommentAddArgs {
    pub key: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorklogAddArgs {
    pub key: String,
    pub time_spent: String,
    pub comment: Option<String>,
    pub started: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorklogDeleteArgs {
    pub key: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BulkTransitionArgs {
    pub jql: String,
    pub to: String,
    pub confirm: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BulkUpdateArgs {
    pub jql: String,
    pub assignee: Option<String>,
    pub priority: Option<String>,
    pub confirm: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ArchiveArgs {
    pub jql: String,
    pub confirm: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BatchCreateOp {
    pub project_key: String,
    pub summary: String,
    pub issue_type: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    #[schemars(schema_with = "any_json_object")]
    pub description_adf: Option<Value>,
    pub assignee: Option<String>,
    pub priority: Option<String>,
    pub labels: Option<Vec<String>>,
    pub components: Option<Vec<String>>,
    pub parent: Option<String>,
    pub fix_versions: Option<Vec<String>>,
    #[serde(default)]
    #[schemars(schema_with = "any_json_object_map")]
    pub custom_fields: Option<BTreeMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BatchUpdateOp {
    pub key: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    #[schemars(schema_with = "any_json_object")]
    pub description_adf: Option<Value>,
    pub assignee: Option<String>,
    pub priority: Option<String>,
    pub labels: Option<Vec<String>>,
    pub components: Option<Vec<String>>,
    pub parent: Option<String>,
    pub fix_versions: Option<Vec<String>>,
    #[serde(default)]
    #[schemars(schema_with = "any_json_object_map")]
    pub custom_fields: Option<BTreeMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BatchTransitionOp {
    pub key: String,
    pub transition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BatchArchiveOp {
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum BatchOperation {
    Create(BatchCreateOp),
    Update(BatchUpdateOp),
    Transition(BatchTransitionOp),
    Archive(BatchArchiveOp),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BatchArgs {
    pub operations: Vec<BatchOperation>,
    pub confirm: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApiRequestArgs {
    pub method: String,
    pub path: String,
    #[serde(default)]
    #[schemars(schema_with = "any_json_object_map")]
    pub query: Option<BTreeMap<String, Value>>,
    #[serde(default)]
    #[schemars(schema_with = "any_json_object")]
    pub body: Option<Value>,
}
