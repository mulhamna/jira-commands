use serde_json::{json, Value};
use serial_test::serial;
use tempfile::TempDir;
use wiremock::{
    matchers::{method, path, query_param},
    Mock, MockServer, ResponseTemplate,
};

use crate::models::{
    ApiRequestArgs, AuthSetCredentialsArgs, BatchArgs, BulkCommentArgs, IssueDeleteArgs,
    IssueKeyArgs, IssueListArgs, IssueNotificationsArgs, IssueStandupArgs, ProjectKeyArgs,
    ProjectVersionUpdateArgs, SprintListArgs, SprintUpdateArgs,
};

use super::shared::build_api_path;
use super::JiraApp;

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

    let err = JiraApp
        .issue_bulk_comment(BulkCommentArgs {
            jql: Some("project = PROJ".into()),
            keys: None,
            body: "hello".into(),
            confirm: None,
        })
        .await
        .expect_err("missing confirm should fail");

    assert_eq!(err.to_mcp().message, "unsafe_operation");
}

#[tokio::test]
#[serial]
async fn bulk_comment_requires_targets() {
    let err = JiraApp
        .issue_bulk_comment(BulkCommentArgs {
            jql: None,
            keys: None,
            body: "hello".into(),
            confirm: Some(true),
        })
        .await
        .expect_err("missing targets should fail");

    assert_eq!(err.to_mcp().message, "validation_error");
}

#[tokio::test]
#[serial]
async fn issue_batch_allows_empty_operations() {
    let result = JiraApp
        .issue_batch(BatchArgs {
            operations: Vec::new(),
            confirm: Some(true),
        })
        .await
        .expect("empty batch should succeed");

    assert_eq!(result["total"], Value::from(0));
    assert_eq!(result["results"].as_array().map(Vec::len), Some(0));
}

#[tokio::test]
#[serial]
async fn standup_rejects_invalid_since_window() {
    let err = JiraApp
        .issue_standup(IssueStandupArgs {
            project_key: None,
            jql: None,
            since: Some("nope".into()),
            limit: Some(10),
        })
        .await
        .expect_err("invalid since should fail");

    assert_eq!(err.to_mcp().message, "validation_error");
}

#[tokio::test]
#[serial]
async fn notifications_reject_invalid_limit_before_auth() {
    let err = JiraApp
        .issue_notifications(IssueNotificationsArgs {
            project_key: None,
            since: None,
            limit: Some(101),
        })
        .await
        .expect_err("invalid limit should fail");

    assert_eq!(err.to_mcp().message, "validation_error");
}

#[tokio::test]
#[serial]
async fn auth_round_trip_uses_shared_config_file() {
    let temp_dir = TempDir::new().expect("tempdir");
    set_test_env(&temp_dir, None);

    let status = JiraApp
        .auth_set_credentials(AuthSetCredentialsArgs {
            profile: None,
            url: Some("https://example.atlassian.net".into()),
            email: Some("dev@example.com".into()),
            token: Some("secret".into()),
            project: Some("PROJ".into()),
            timeout_secs: Some(45),
            deployment: None,
            auth_type: None,
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

#[tokio::test]
#[serial]
async fn project_component_list_returns_components() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mock_server = MockServer::start().await;
    set_test_env(&temp_dir, Some(&mock_server.uri()));

    Mock::given(method("GET"))
        .and(path("/rest/api/3/project/PROJ/components"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "id": "1001",
                "name": "Platform",
                "description": "Core services"
            }
        ])))
        .mount(&mock_server)
        .await;

    let result = JiraApp
        .project_component_list(ProjectKeyArgs {
            project_key: "PROJ".into(),
        })
        .await
        .expect("component list");

    assert_eq!(result["project_key"], Value::String("PROJ".into()));
    assert_eq!(result["components"].as_array().map(Vec::len), Some(1));

    clear_test_env();
}

#[tokio::test]
#[serial]
async fn sprint_list_uses_requested_states() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mock_server = MockServer::start().await;
    set_test_env(&temp_dir, Some(&mock_server.uri()));

    Mock::given(method("GET"))
        .and(path("/rest/agile/1.0/board"))
        .and(query_param("projectKeyOrId", "PROJ"))
        .and(query_param("maxResults", "100"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "values": [{ "id": 77 }]
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/rest/agile/1.0/board/77/sprint"))
        .and(query_param("state", "active,closed"))
        .and(query_param("maxResults", "200"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "values": [{
                "id": 55,
                "name": "Sprint 55",
                "state": "active",
                "goal": "Ship metadata tools"
            }]
        })))
        .mount(&mock_server)
        .await;

    let result = JiraApp
        .sprint_list(SprintListArgs {
            project_key: "PROJ".into(),
            states: Some(vec!["active".into(), "closed".into()]),
        })
        .await
        .expect("sprint list");

    assert_eq!(result["project_key"], Value::String("PROJ".into()));
    assert_eq!(result["sprints"].as_array().map(Vec::len), Some(1));
    assert_eq!(result["sprints"][0]["id"], Value::Number(55u64.into()));

    clear_test_env();
}

#[tokio::test]
#[serial]
async fn sprint_update_requires_at_least_one_field() {
    let err = JiraApp
        .sprint_update(SprintUpdateArgs {
            sprint_id: 42,
            name: None,
            state: None,
            start_date: None,
            end_date: None,
            goal: None,
        })
        .await
        .expect_err("missing sprint changes should fail");

    assert_eq!(err.to_mcp().message, "validation_error");
}

#[tokio::test]
#[serial]
async fn project_version_update_requires_changes() {
    let err = JiraApp
        .project_version_update(ProjectVersionUpdateArgs {
            version_id: "1000".into(),
            name: None,
            description: None,
            archived: None,
            released: None,
            release_date: None,
            start_date: None,
        })
        .await
        .expect_err("missing version changes should fail");

    assert_eq!(err.to_mcp().message, "validation_error");
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

#[tokio::test]
#[serial]
async fn issue_link_types_list_returns_values() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mock_server = MockServer::start().await;
    set_test_env(&temp_dir, Some(&mock_server.uri()));

    Mock::given(method("GET"))
        .and(path("/rest/api/3/issueLinkType"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "issueLinkTypes": [{
                "id": "10000",
                "name": "Blocks",
                "inward": "is blocked by",
                "outward": "blocks"
            }]
        })))
        .mount(&mock_server)
        .await;

    let result = JiraApp
        .issue_link_types_list()
        .await
        .expect("link type list");

    assert_eq!(result["link_types"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        result["link_types"][0]["name"],
        Value::String("Blocks".into())
    );

    clear_test_env();
}

#[tokio::test]
#[serial]
async fn remote_link_list_returns_links() {
    let temp_dir = TempDir::new().expect("tempdir");
    let mock_server = MockServer::start().await;
    set_test_env(&temp_dir, Some(&mock_server.uri()));

    Mock::given(method("GET"))
        .and(path("/rest/api/3/issue/PROJ-1/remotelink"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
            "id": 321,
            "relationship": "references",
            "object": {
                "title": "Spec",
                "url": "https://example.com/spec"
            }
        }])))
        .mount(&mock_server)
        .await;

    let result = JiraApp
        .remote_link_list(IssueKeyArgs {
            key: "PROJ-1".into(),
        })
        .await
        .expect("remote links");

    assert_eq!(result["key"], Value::String("PROJ-1".into()));
    assert_eq!(result["remote_links"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        result["remote_links"][0]["object"]["title"],
        Value::String("Spec".into())
    );

    clear_test_env();
}
