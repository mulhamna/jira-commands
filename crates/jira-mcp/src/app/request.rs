use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde_json::Value;

use jira_core::model::{field::FieldValue, CreateIssueRequestV2, Issue, UpdateIssueRequest};

use crate::{
    error::{AppError, AppResult},
    models::{BatchCreateOp, BatchUpdateOp, IssueCreateArgs, IssueUpdateArgs},
};

/// Map custom field inputs to FieldValue for serialization.
///
/// Wraps each value as `FieldValue::Raw` to preserve the original JSON type,
/// which allows array fields (e.g., Labels, MultiSelect) to work correctly.
/// The JSON shape must match Jira's expectations for the field type:
/// - Array field (e.g., Labels): `["value1", "value2"]`
/// - Select field: `{ "value": "option" }`
/// - User field: `{ "emailAddress": "user@example.com" }`
/// - Text/Number: scalar value
pub(super) fn map_custom_fields(
    custom_fields: Option<std::collections::BTreeMap<String, Value>>,
) -> HashMap<String, FieldValue> {
    custom_fields
        .unwrap_or_default()
        .into_iter()
        .map(|(key, value)| (key, FieldValue::Raw(value)))
        .collect()
}

pub(super) fn create_issue_request_from_issue(args: IssueCreateArgs) -> CreateIssueRequestV2 {
    CreateIssueRequestV2 {
        project_key: args.project_key,
        summary: args.summary,
        description: args.description,
        description_adf: args.description_adf,
        issue_type: args.issue_type,
        assignee: args.assignee,
        priority: args.priority,
        labels: args.labels.unwrap_or_default(),
        components: args.components.unwrap_or_default(),
        parent: args.parent,
        fix_versions: args.fix_versions.unwrap_or_default(),
        custom_fields: map_custom_fields(args.custom_fields),
    }
}

pub(super) fn create_issue_request_from_batch_create(entry: BatchCreateOp) -> CreateIssueRequestV2 {
    CreateIssueRequestV2 {
        project_key: entry.project_key,
        summary: entry.summary,
        description: entry.description,
        description_adf: entry.description_adf,
        issue_type: entry.issue_type.unwrap_or_else(|| "Task".to_string()),
        assignee: entry.assignee,
        priority: entry.priority,
        labels: entry.labels.unwrap_or_default(),
        components: entry.components.unwrap_or_default(),
        parent: entry.parent,
        fix_versions: entry.fix_versions.unwrap_or_default(),
        custom_fields: map_custom_fields(entry.custom_fields),
    }
}

pub(super) fn build_update_issue_request_from_issue(
    args: IssueUpdateArgs,
) -> AppResult<UpdateIssueRequest> {
    let custom_fields = map_custom_fields(args.custom_fields);
    let has_changes = args.summary.is_some()
        || args.description.is_some()
        || args.description_adf.is_some()
        || args.assignee.is_some()
        || args.priority.is_some()
        || args.labels.is_some()
        || args.components.is_some()
        || args.parent.is_some()
        || args.fix_versions.is_some()
        || !custom_fields.is_empty();

    if !has_changes {
        return Err(AppError::validation(
            "Provide at least one field to update on the issue",
        ));
    }

    Ok(UpdateIssueRequest {
        summary: args.summary,
        description: args.description,
        description_adf: args.description_adf,
        assignee: args.assignee,
        priority: args.priority,
        labels: args.labels,
        components: args.components,
        fix_versions: args.fix_versions,
        parent: args.parent,
        custom_fields,
        ..Default::default()
    })
}

pub(super) fn build_update_issue_request_from_batch(
    entry: BatchUpdateOp,
) -> AppResult<UpdateIssueRequest> {
    let custom_fields = map_custom_fields(entry.custom_fields);
    let has_changes = entry.summary.is_some()
        || entry.description.is_some()
        || entry.description_adf.is_some()
        || entry.assignee.is_some()
        || entry.priority.is_some()
        || entry.labels.is_some()
        || entry.components.is_some()
        || entry.parent.is_some()
        || entry.fix_versions.is_some()
        || !custom_fields.is_empty();

    if !has_changes {
        return Err(AppError::validation(
            "Provide at least one field to update on the issue",
        ));
    }

    Ok(UpdateIssueRequest {
        summary: entry.summary,
        description: entry.description,
        description_adf: entry.description_adf,
        assignee: entry.assignee,
        priority: entry.priority,
        labels: entry.labels,
        components: entry.components,
        fix_versions: entry.fix_versions,
        parent: entry.parent,
        custom_fields,
        ..Default::default()
    })
}

pub(super) fn issue_status_category(issue: &Issue) -> String {
    issue
        .fields
        .get("status")
        .and_then(|status| status.get("statusCategory"))
        .and_then(|category| category.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_lowercase()
}

pub(super) fn issue_is_blocked(issue: &Issue) -> bool {
    let status = issue.status.to_lowercase();
    status.contains("blocked") || status.contains("on hold") || status.contains("stuck")
}

pub(super) fn issue_updated_at(issue: &Issue) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&issue.updated)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}
