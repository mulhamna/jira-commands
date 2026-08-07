use serde::Serialize;
use serde_json::{json, Value};
use url::form_urlencoded;

use jira_core::{adf::adf_to_text, config::JiraConfig, model::Issue, JiraClient};

use crate::error::{AppError, AppResult};

use super::JiraApp;

impl JiraApp {
    pub(crate) fn load_config(&self) -> AppResult<JiraConfig> {
        JiraConfig::load().map_err(Into::into)
    }

    pub(crate) fn build_client(&self) -> AppResult<JiraClient> {
        let config = self.load_config()?;

        if config.base_url.trim().is_empty() {
            return Err(AppError::auth_missing(
                "Jira URL not configured. Set JIRA_URL or save credentials first.",
            ));
        }
        if config.requires_user_identity() && config.email.trim().is_empty() {
            return Err(AppError::auth_missing(
                "Jira user identity not configured. Set JIRA_EMAIL or save credentials first.",
            ));
        }
        if !config.token_present() {
            return Err(AppError::auth_missing(
                "Jira API token not configured. Set JIRA_TOKEN or save credentials first.",
            ));
        }

        Ok(JiraClient::new(config))
    }

    /// Resolve the effective limit for a list-like call.
    ///
    /// Priority: explicit tool arg > `default_issue_limit` from config >
    /// `fallback`. The value is clamped to the Jira API upper bound (5000)
    /// and a minimum of 1.
    pub(crate) fn resolve_limit(&self, arg: Option<u32>, fallback: u32) -> AppResult<u32> {
        const MAX: u32 = 5_000;
        let value = arg
            .or_else(|| self.load_config().ok().and_then(|c| c.default_issue_limit))
            .unwrap_or(fallback);
        if value == 0 {
            return Err(AppError::validation("limit must be at least 1"));
        }
        Ok(value.clamp(1, MAX))
    }
}

pub(super) fn require_confirm(confirm: Option<bool>) -> AppResult<()> {
    if confirm == Some(true) {
        Ok(())
    } else {
        Err(AppError::unsafe_operation(
            "This operation requires confirm=true",
        ))
    }
}

pub(super) fn normalize_method(method: &str) -> AppResult<String> {
    let method = method.trim().to_ascii_uppercase();
    match method.as_str() {
        "GET" | "POST" | "PUT" | "PATCH" | "DELETE" => Ok(method),
        _ => Err(AppError::validation(
            "method must be one of GET, POST, PUT, PATCH, or DELETE",
        )),
    }
}

pub(super) fn build_api_path(
    path: String,
    query: Option<std::collections::BTreeMap<String, Value>>,
) -> AppResult<String> {
    if !path.starts_with('/') {
        return Err(AppError::validation("path must start with '/'"));
    }
    if path.contains('?') && query.is_some() {
        return Err(AppError::validation(
            "path already contains a query string; omit the query argument",
        ));
    }

    let Some(query) = query else {
        return Ok(path);
    };
    if query.is_empty() {
        return Ok(path);
    }

    let mut serializer = form_urlencoded::Serializer::new(String::new());
    for (key, value) in query {
        serializer.append_pair(&key, &query_value_to_string(key.as_str(), value)?);
    }
    let encoded = serializer.finish();
    Ok(format!("{path}?{encoded}"))
}

pub(super) fn query_value_to_string(key: &str, value: Value) -> AppResult<String> {
    match value {
        Value::Null => Err(AppError::validation(format!(
            "query parameter '{key}' cannot be null"
        ))),
        Value::Bool(value) => Ok(value.to_string()),
        Value::Number(value) => Ok(value.to_string()),
        Value::String(value) => Ok(value),
        Value::Array(_) | Value::Object(_) => Err(AppError::validation(format!(
            "query parameter '{key}' must be a string, number, or boolean"
        ))),
    }
}

pub(super) fn to_value<T>(value: T) -> AppResult<Value>
where
    T: Serialize,
{
    serde_json::to_value(value).map_err(Into::into)
}

/// Max characters of a flattened ADF description kept in a slimmed issue.
const DESCRIPTION_EXCERPT_LEN: usize = 500;

/// Project a full [`Issue`] down to the LLM-useful fields, replacing the heavy
/// ADF `description` (and the raw `fields` map) with a short plain-text excerpt.
///
/// MCP stdio responses are newline-delimited JSON on a single line; many clients
/// cap line length (e.g. Python `asyncio.StreamReader` defaults to 64 KB), so a
/// list of full issues with ADF bodies can overflow and surface as a truncated
/// `Unexpected EOF` parse error. Slimming keeps responses well under that cap.
pub(super) fn slim_issue(issue: &Issue) -> Value {
    let excerpt = issue.description.as_ref().map(|adf| {
        let text = adf_to_text(adf);
        truncate_chars(text.trim(), DESCRIPTION_EXCERPT_LEN)
    });

    json!({
        "id": issue.id,
        "key": issue.key,
        "summary": issue.summary,
        "status": issue.status,
        "issue_type": issue.issue_type,
        "project_key": issue.project_key,
        "assignee": issue.assignee,
        "priority": issue.priority,
        "created": issue.created,
        "updated": issue.updated,
        "description_excerpt": excerpt,
    })
}

/// Truncate a string to at most `max` characters (not bytes), appending an
/// ellipsis marker when content was dropped.
pub(super) fn truncate_chars(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let kept: String = text.chars().take(max).collect();
    format!("{kept}…[truncated]")
}

pub(super) fn value_or_null(value: String) -> Value {
    if value.trim().is_empty() {
        Value::Null
    } else {
        Value::String(value)
    }
}

pub(super) fn normalize_issue_keys(raw_keys: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for raw in raw_keys {
        for key in raw
            .split(|c: char| c == ',' || c.is_whitespace())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            if !out.iter().any(|existing| existing == key) {
                out.push(key.to_string());
            }
        }
    }
    out
}
