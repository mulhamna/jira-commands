use std::{collections::HashMap, path::PathBuf};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use jira_core::{
    config::{config_file_path, JiraConfig},
    model::{
        field::{Field, FieldValue},
        CreateIssueRequestV2, UpdateIssueRequest,
    },
    JiraClient,
};
use serde::Serialize;
use serde_json::{json, Value};
use url::form_urlencoded;

use crate::{
    error::{AppError, AppResult},
    models::{
        ApiRequestArgs, ArchiveArgs, AttachmentInput, AuthSetCredentialsArgs, BulkTransitionArgs,
        BulkUpdateArgs, CommentAddArgs, IssueAttachArgs, IssueCreateArgs, IssueDeleteArgs,
        IssueFieldsArgs, IssueKeyArgs, IssueListArgs, IssueTransitionArgs, IssueTypesListArgs,
        IssueUpdateArgs, WorklogAddArgs, WorklogDeleteArgs,
    },
};

#[derive(Debug, Clone, Default)]
pub struct JiraApp;

impl JiraApp {
    pub fn auth_status(&self) -> AppResult<Value> {
        let config = self.load_config()?;
        Ok(json!({
            "configured": !config.base_url.is_empty() && !config.email.is_empty(),
            "url": value_or_null(config.base_url),
            "email": value_or_null(config.email),
            "token_present": config.token.is_some(),
            "project": config.project,
            "timeout_secs": config.timeout_secs,
            "config_path": config_file_path().display().to_string()
        }))
    }

    pub fn auth_set_credentials(&self, args: AuthSetCredentialsArgs) -> AppResult<Value> {
        if args.url.is_none()
            && args.email.is_none()
            && args.token.is_none()
            && args.project.is_none()
            && args.timeout_secs.is_none()
        {
            return Err(AppError::validation(
                "Provide at least one of url, email, token, project, or timeout_secs",
            ));
        }

        let mut config = self.load_config().unwrap_or_default();

        if let Some(url) = args.url {
            config.base_url = url.trim().to_string();
        }
        if let Some(email) = args.email {
            config.email = email.trim().to_string();
        }
        if let Some(token) = args.token {
            config.token = Some(token);
        }
        if let Some(project) = args.project {
            config.project = if project.trim().is_empty() {
                None
            } else {
                Some(project.trim().to_string())
            };
        }
        if let Some(timeout_secs) = args.timeout_secs {
            config.timeout_secs = timeout_secs;
        }

        config.save()?;
        self.auth_status()
    }

    pub fn auth_logout(&self) -> AppResult<Value> {
        let mut config = self.load_config().unwrap_or_default();
        config.token = None;
        config.save()?;
        self.auth_status()
    }

    pub async fn issue_list(&self, args: IssueListArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let limit = args.limit.unwrap_or(25);
        if !(1..=100).contains(&limit) {
            return Err(AppError::validation("limit must be between 1 and 100"));
        }

        let jql = if let Some(jql) = args.jql {
            jql
        } else if let Some(project_key) = args.project_key {
            format!("project = {project_key} ORDER BY updated DESC")
        } else {
            "assignee = currentUser() ORDER BY updated DESC".to_string()
        };

        let result = client.search_issues(&jql, None, Some(limit)).await?;
        Ok(json!({
            "jql": jql,
            "issues": result.issues,
            "next_page_token": result.next_page_token,
            "total": result.total
        }))
    }

    pub async fn issue_view(&self, args: IssueKeyArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let issue = client.get_issue(&args.key).await?;
        to_value(issue)
    }

    pub async fn issue_types_list(&self, args: IssueTypesListArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let issue_types = client.get_issue_types(&args.project_key).await?;
        Ok(json!({
            "project_key": args.project_key,
            "issue_types": issue_types
        }))
    }

    pub async fn issue_fields(&self, args: IssueFieldsArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let mut fields: Vec<Field> = if let Some(issue_type_id) = args.issue_type_id {
            client
                .get_fields_for_issue_type(&args.project_key, &issue_type_id)
                .await?
        } else {
            client.get_project_fields(&args.project_key).await?
        };

        if args.required_only.unwrap_or(false) {
            fields.retain(|field| field.required);
        }

        Ok(json!({
            "project_key": args.project_key,
            "fields": fields
        }))
    }

    pub async fn issue_transitions_list(&self, args: IssueKeyArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let transitions = client.get_transitions(&args.key).await?;
        Ok(json!({
            "key": args.key,
            "transitions": transitions
        }))
    }

    pub async fn issue_create(&self, args: IssueCreateArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let issue = client
            .create_issue_v2(CreateIssueRequestV2 {
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
            })
            .await?;

        to_value(issue)
    }

    pub async fn issue_update(&self, args: IssueUpdateArgs) -> AppResult<Value> {
        let client = self.build_client()?;
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

        let key = args.key;
        client
            .update_issue(
                &key,
                UpdateIssueRequest {
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
                },
            )
            .await?;
        let issue = client.get_issue(&key).await?;
        to_value(issue)
    }

    pub async fn issue_delete(&self, args: IssueDeleteArgs) -> AppResult<Value> {
        require_confirm(args.confirm)?;
        let client = self.build_client()?;
        client.delete_issue(&args.key).await?;
        Ok(json!({
            "key": args.key,
            "deleted": true
        }))
    }

    pub async fn issue_transition(&self, args: IssueTransitionArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let resolved = resolve_transition(&client, &args.key, &args.transition).await?;
        client.transition_issue(&args.key, &resolved.id).await?;
        let issue = client.get_issue(&args.key).await?;
        Ok(json!({
            "transition": {
                "id": resolved.id,
                "name": resolved.name
            },
            "issue": issue
        }))
    }

    pub async fn issue_attach(&self, args: IssueAttachArgs) -> AppResult<Value> {
        if args.attachments.is_empty() {
            return Err(AppError::validation("attachments must not be empty"));
        }

        let client = self.build_client()?;
        let mut uploaded = Vec::new();

        for attachment in args.attachments {
            let mut result = match attachment {
                AttachmentInput::Path { path } => {
                    let path = PathBuf::from(path);
                    client.upload_attachment(&args.key, path.as_path()).await?
                }
                AttachmentInput::Inline {
                    filename,
                    media_type,
                    base64,
                } => {
                    let bytes = STANDARD.decode(base64)?;
                    client
                        .upload_attachment_bytes(&args.key, &filename, bytes, media_type.as_deref())
                        .await?
                }
            };
            uploaded.append(&mut result);
        }

        Ok(json!({
            "key": args.key,
            "attachments": uploaded
        }))
    }

    pub async fn comment_list(&self, args: IssueKeyArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let comments = client.get_comments(&args.key).await?;
        Ok(json!({
            "key": args.key,
            "comments": comments
        }))
    }

    pub async fn comment_add(&self, args: CommentAddArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let comment = client.add_comment(&args.key, &args.body).await?;
        to_value(comment)
    }

    pub async fn worklog_list(&self, args: IssueKeyArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let worklogs = client.get_worklogs(&args.key).await?;
        Ok(json!({
            "key": args.key,
            "worklogs": worklogs
        }))
    }

    pub async fn worklog_add(&self, args: WorklogAddArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let worklog = client
            .add_worklog(
                &args.key,
                &args.time_spent,
                args.comment.as_deref(),
                args.started.as_deref(),
            )
            .await?;
        to_value(worklog)
    }

    pub async fn worklog_delete(&self, args: WorklogDeleteArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        client.delete_worklog(&args.key, &args.id).await?;
        Ok(json!({
            "key": args.key,
            "id": args.id,
            "deleted": true
        }))
    }

    pub async fn issue_bulk_transition(&self, args: BulkTransitionArgs) -> AppResult<Value> {
        require_confirm(args.confirm)?;
        let client = self.build_client()?;
        let issues = client.get_all_issues(&args.jql).await?;
        if issues.is_empty() {
            return Ok(json!({
                "jql": args.jql,
                "total": 0,
                "succeeded": 0,
                "failed_count": 0,
                "failed": []
            }));
        }

        let transition = resolve_transition(&client, &issues[0].key, &args.to).await?;
        let total = issues.len();
        let mut succeeded = 0usize;
        let mut failed = Vec::new();

        for issue in issues {
            match client.transition_issue(&issue.key, &transition.id).await {
                Ok(_) => succeeded += 1,
                Err(err) => failed.push(json!({
                    "key": issue.key,
                    "error": err.to_string()
                })),
            }
        }

        Ok(json!({
            "jql": args.jql,
            "transition": {
                "id": transition.id,
                "name": transition.name
            },
            "total": total,
            "succeeded": succeeded,
            "failed_count": failed.len(),
            "failed": failed
        }))
    }

    pub async fn issue_bulk_update(&self, args: BulkUpdateArgs) -> AppResult<Value> {
        require_confirm(args.confirm)?;
        if args.assignee.is_none() && args.priority.is_none() {
            return Err(AppError::validation(
                "Provide assignee and/or priority for a bulk update",
            ));
        }

        let client = self.build_client()?;
        let issues = client.get_all_issues(&args.jql).await?;
        if issues.is_empty() {
            return Ok(json!({
                "jql": args.jql,
                "total": 0,
                "succeeded": 0,
                "failed_count": 0,
                "failed": []
            }));
        }

        let total = issues.len();
        let request = UpdateIssueRequest {
            assignee: args.assignee,
            priority: args.priority,
            ..Default::default()
        };
        let mut succeeded = 0usize;
        let mut failed = Vec::new();

        for issue in issues {
            match client.update_issue(&issue.key, request.clone()).await {
                Ok(_) => succeeded += 1,
                Err(err) => failed.push(json!({
                    "key": issue.key,
                    "error": err.to_string()
                })),
            }
        }

        Ok(json!({
            "jql": args.jql,
            "total": total,
            "succeeded": succeeded,
            "failed_count": failed.len(),
            "failed": failed
        }))
    }

    pub async fn issue_archive(&self, args: ArchiveArgs) -> AppResult<Value> {
        require_confirm(args.confirm)?;
        let client = self.build_client()?;
        let issues = client.get_all_issues(&args.jql).await?;
        let keys: Vec<String> = issues.into_iter().map(|issue| issue.key).collect();

        if keys.is_empty() {
            return Ok(json!({
                "jql": args.jql,
                "total": 0,
                "archived": 0,
                "keys": []
            }));
        }

        client.archive_issues(&keys).await?;
        Ok(json!({
            "jql": args.jql,
            "total": keys.len(),
            "archived": keys.len(),
            "keys": keys
        }))
    }

    pub async fn plan_list(&self) -> AppResult<Value> {
        let client = self.build_client()?;
        let plans = client.get_plans().await?;
        Ok(json!({ "plans": plans }))
    }

    pub async fn api_request(&self, args: ApiRequestArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let method = normalize_method(&args.method)?;
        let path = build_api_path(args.path, args.query)?;
        let body = client.raw_request(&method, &path, args.body).await?;
        Ok(json!({
            "method": method,
            "path": path,
            "body": body
        }))
    }

    fn load_config(&self) -> AppResult<JiraConfig> {
        JiraConfig::load().map_err(Into::into)
    }

    fn build_client(&self) -> AppResult<JiraClient> {
        let config = self.load_config()?;

        if config.base_url.trim().is_empty() {
            return Err(AppError::auth_missing(
                "Jira URL not configured. Set JIRA_URL or save credentials first.",
            ));
        }
        if config.email.trim().is_empty() {
            return Err(AppError::auth_missing(
                "Jira email not configured. Set JIRA_EMAIL or save credentials first.",
            ));
        }
        if config.token.as_deref().unwrap_or("").trim().is_empty() {
            return Err(AppError::auth_missing(
                "Jira API token not configured. Set JIRA_TOKEN or save credentials first.",
            ));
        }

        Ok(JiraClient::new(config))
    }
}

#[derive(Debug, Clone)]
struct ResolvedTransition {
    id: String,
    name: String,
}

async fn resolve_transition(
    client: &JiraClient,
    key: &str,
    name_or_id: &str,
) -> AppResult<ResolvedTransition> {
    let transitions = client.get_transitions(key).await?;
    let found = transitions
        .iter()
        .find(|transition| {
            transition.get("id").and_then(|value| value.as_str()) == Some(name_or_id)
                || transition
                    .get("name")
                    .and_then(|value| value.as_str())
                    .is_some_and(|name| name.eq_ignore_ascii_case(name_or_id))
        })
        .ok_or_else(|| {
            AppError::not_found(
                format!("Transition '{name_or_id}' not found for {key}"),
                Some(json!({ "key": key })),
            )
        })?;

    Ok(ResolvedTransition {
        id: found
            .get("id")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        name: found
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or(name_or_id)
            .to_string(),
    })
}

fn map_custom_fields(
    custom_fields: Option<std::collections::BTreeMap<String, Value>>,
) -> HashMap<String, FieldValue> {
    custom_fields
        .unwrap_or_default()
        .into_iter()
        .map(|(key, value)| (key, FieldValue::Raw(value)))
        .collect()
}

fn require_confirm(confirm: Option<bool>) -> AppResult<()> {
    if confirm == Some(true) {
        Ok(())
    } else {
        Err(AppError::unsafe_operation(
            "This operation requires confirm=true",
        ))
    }
}

fn normalize_method(method: &str) -> AppResult<String> {
    let method = method.trim().to_ascii_uppercase();
    match method.as_str() {
        "GET" | "POST" | "PUT" | "PATCH" | "DELETE" => Ok(method),
        _ => Err(AppError::validation(
            "method must be one of GET, POST, PUT, PATCH, or DELETE",
        )),
    }
}

fn build_api_path(
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

fn query_value_to_string(key: &str, value: Value) -> AppResult<String> {
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

fn to_value<T>(value: T) -> AppResult<Value>
where
    T: Serialize,
{
    serde_json::to_value(value).map_err(Into::into)
}

fn value_or_null(value: String) -> Value {
    if value.trim().is_empty() {
        Value::Null
    } else {
        Value::String(value)
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;
    use tempfile::TempDir;
    use wiremock::{
        matchers::{method, path, query_param},
        Mock, MockServer, ResponseTemplate,
    };

    use super::*;

    fn set_config_home_vars(temp_dir: &TempDir) {
        std::env::set_var("XDG_CONFIG_HOME", temp_dir.path());
        std::env::set_var("HOME", temp_dir.path());
        std::env::set_var("USERPROFILE", temp_dir.path());
        std::env::set_var("APPDATA", temp_dir.path());
        std::env::set_var("LOCALAPPDATA", temp_dir.path());
    }

    fn clear_config_home_vars() {
        std::env::remove_var("XDG_CONFIG_HOME");
        std::env::remove_var("HOME");
        std::env::remove_var("USERPROFILE");
        std::env::remove_var("APPDATA");
        std::env::remove_var("LOCALAPPDATA");
    }

    fn set_test_env(temp_dir: &TempDir, base_url: Option<&str>) {
        set_config_home_vars(temp_dir);
        match base_url {
            Some(base_url) => {
                std::env::set_var("JIRA_URL", base_url);
                std::env::set_var("JIRA_EMAIL", "dev@example.com");
                std::env::set_var("JIRA_TOKEN", "token-123");
            }
            None => {
                std::env::remove_var("JIRA_URL");
                std::env::remove_var("JIRA_EMAIL");
                std::env::remove_var("JIRA_TOKEN");
            }
        }
    }

    fn clear_test_env() {
        clear_config_home_vars();
        std::env::remove_var("JIRA_URL");
        std::env::remove_var("JIRA_EMAIL");
        std::env::remove_var("JIRA_TOKEN");
    }

    fn sample_issue() -> Value {
        json!({
            "id": "10001",
            "key": "PROJ-1",
            "fields": {
                "summary": "Sample issue",
                "description": null,
                "status": { "name": "To Do" },
                "assignee": { "displayName": "Dev User" },
                "reporter": { "displayName": "Reporter User" },
                "priority": { "name": "High" },
                "issuetype": { "name": "Task" },
                "project": { "key": "PROJ" },
                "created": "2026-04-19T00:00:00.000+0000",
                "updated": "2026-04-19T00:00:00.000+0000",
                "attachment": []
            }
        })
    }

    #[tokio::test]
    #[serial]
    async fn destructive_actions_require_confirm() {
        let err = JiraApp
            .issue_delete(IssueDeleteArgs {
                key: "PROJ-1".into(),
                confirm: None,
            })
            .await
            .expect_err("missing confirm should fail");

        assert_eq!(err.to_mcp().message, "unsafe_operation");
    }

    #[tokio::test]
    #[serial]
    async fn auth_round_trip_uses_shared_config_file() {
        let temp_dir = TempDir::new().expect("tempdir");
        set_test_env(&temp_dir, None);

        let status = JiraApp
            .auth_set_credentials(AuthSetCredentialsArgs {
                url: Some("https://example.atlassian.net".into()),
                email: Some("dev@example.com".into()),
                token: Some("secret".into()),
                project: Some("PROJ".into()),
                timeout_secs: Some(45),
            })
            .expect("set credentials");
        assert_eq!(status["token_present"], Value::Bool(true));
        assert_eq!(status["project"], Value::String("PROJ".into()));

        let status = JiraApp.auth_logout().expect("logout");
        assert_eq!(status["token_present"], Value::Bool(false));

        clear_test_env();
    }

    #[tokio::test]
    #[serial]
    async fn issue_list_defaults_to_current_user_jql() {
        let temp_dir = TempDir::new().expect("tempdir");
        let mock_server = MockServer::start().await;
        set_test_env(&temp_dir, Some(&mock_server.uri()));

        Mock::given(method("POST"))
            .and(path("/rest/api/3/search/jql"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "issues": [sample_issue()],
                "nextPageToken": null,
                "total": 1
            })))
            .mount(&mock_server)
            .await;

        let result = JiraApp
            .issue_list(IssueListArgs {
                project_key: None,
                jql: None,
                limit: Some(10),
            })
            .await
            .expect("issue list");

        assert_eq!(
            result["jql"],
            Value::String("assignee = currentUser() ORDER BY updated DESC".into())
        );
        assert_eq!(result["issues"].as_array().map(Vec::len), Some(1));

        clear_test_env();
    }

    #[tokio::test]
    #[serial]
    async fn api_request_serializes_query_parameters() {
        let temp_dir = TempDir::new().expect("tempdir");
        let mock_server = MockServer::start().await;
        set_test_env(&temp_dir, Some(&mock_server.uri()));

        Mock::given(method("GET"))
            .and(path("/rest/api/3/project"))
            .and(query_param("expand", "lead"))
            .and(query_param("startAt", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "ok": true
            })))
            .mount(&mock_server)
            .await;

        let response = JiraApp
            .api_request(ApiRequestArgs {
                method: "get".into(),
                path: "/rest/api/3/project".into(),
                query: Some(
                    [
                        ("expand".to_string(), Value::String("lead".into())),
                        ("startAt".to_string(), Value::Number(1.into())),
                    ]
                    .into_iter()
                    .collect(),
                ),
                body: None,
            })
            .await
            .expect("api request");

        assert_eq!(response["method"], Value::String("GET".into()));
        assert_eq!(response["body"]["ok"], Value::Bool(true));

        clear_test_env();
    }

    #[test]
    fn build_api_path_rejects_duplicate_query_sources() {
        let err = build_api_path(
            "/rest/api/3/project?expand=lead".into(),
            Some(
                [("startAt".to_string(), Value::Number(1.into()))]
                    .into_iter()
                    .collect(),
            ),
        )
        .expect_err("duplicate query should fail");

        assert_eq!(err.to_mcp().message, "validation_error");
    }
}
