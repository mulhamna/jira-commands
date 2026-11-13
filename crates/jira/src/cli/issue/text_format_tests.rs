use clap::Parser;
use jira_core::{
    adf::{adf_to_wiki, markdown_to_adf, plain_text_to_adf},
    config::{JiraAuthType, JiraConfig, JiraDeployment, TextFormat},
};
use serde_json::json;
use wiremock::{
    matchers::{body_partial_json, method, path},
    Mock, MockServer, ResponseTemplate,
};

use super::*;

const WIKI: &str =
    "h2. Plan\n\n* item *bold* {{mono}}\n{code}\nx = {y}\n{code}\n!shot.png|thumbnail!\n";

#[derive(Parser)]
struct TestCli {
    #[command(subcommand)]
    command: IssueCommand,
}

async fn run(server: &MockServer, api_version: u8, text_format: TextFormat, args: &[&str]) {
    try_run(server, api_version, text_format, args)
        .await
        .unwrap_or_else(|e| panic!("{args:?}: {e:#}"));
}

async fn try_run(
    server: &MockServer,
    api_version: u8,
    text_format: TextFormat,
    args: &[&str],
) -> Result<()> {
    let argv = std::iter::once("issue").chain(args.iter().copied());
    let command = TestCli::try_parse_from(argv)?.command;
    let client = JiraClient::new(JiraConfig {
        base_url: server.uri(),
        token: Some("token".into()),
        deployment: JiraDeployment::DataCenter,
        auth_type: JiraAuthType::DataCenterPat,
        api_version,
        text_format,
        ..JiraConfig::default()
    });
    handle(command, client, None, None).await
}

fn wiki_file(name: &str) -> String {
    let path = std::env::temp_dir().join(format!("jirac-{name}-{}.wiki", std::process::id()));
    std::fs::write(&path, WIKI).expect("write description file");
    path.display().to_string()
}

#[tokio::test]
async fn update_description_follows_profile_and_description_format() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/rest/api/2/issue/P-1"))
        .respond_with(ResponseTemplate::new(204))
        .expect(4)
        .mount(&server)
        .await;
    let file = wiki_file("update");
    let update = ["update", "P-1", "--description-file", &file];

    run(&server, 2, TextFormat::Wiki, &update).await;
    let wiki_flag = [&update[..], &["--description-format", "wiki"]].concat();
    run(&server, 2, TextFormat::Markdown, &wiki_flag).await;
    let md_flag = [&update[..], &["--description-format", "md"]].concat();
    run(&server, 2, TextFormat::Wiki, &md_flag).await;
    let text_flag = [&update[..], &["--description-format", "text"]].concat();
    run(&server, 2, TextFormat::Wiki, &text_flag).await;

    let dc = JiraDeployment::DataCenter;
    let expected: Vec<_> = [
        WIKI.to_string(),
        WIKI.to_string(),
        adf_to_wiki(&markdown_to_adf(WIKI), &dc).unwrap(),
        adf_to_wiki(&plain_text_to_adf(WIKI), &dc).unwrap(),
    ]
    .into_iter()
    .map(|description| json!({ "fields": { "description": description } }))
    .collect();
    let sent: Vec<serde_json::Value> = server
        .received_requests()
        .await
        .expect("recording")
        .iter()
        .map(|request| serde_json::from_slice(&request.body).expect("json body"))
        .collect();
    assert_eq!(sent, expected);
    let _ = std::fs::remove_file(file);
}

#[tokio::test]
async fn create_description_follows_description_format() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/issue/createmeta/P/issuetypes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "issueTypes": [{ "id": "10001", "name": "Task" }]
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/rest/api/2/issue"))
        .and(body_partial_json(
            json!({ "fields": { "description": WIKI } }),
        ))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({ "key": "P-1" })))
        .expect(1)
        .mount(&server)
        .await;
    let file = wiki_file("create");

    run(
        &server,
        2,
        TextFormat::Markdown,
        &[
            "create",
            "-p",
            "P",
            "-s",
            "S",
            "-t",
            "Task",
            "--no-custom-fields",
            "--description-file",
            &file,
            "--description-format",
            "wiki",
        ],
    )
    .await;
    let _ = std::fs::remove_file(file);
}

#[tokio::test]
async fn wiki_on_api_v3_fails_before_the_jql_search() {
    let server = MockServer::start().await;
    let args = [
        "bulk-comment",
        "--jql",
        "project = P",
        "--body",
        "h2. Plan",
        "--force",
    ];

    let err = try_run(&server, 3, TextFormat::Wiki, &args)
        .await
        .expect_err("wiki on API v3");
    assert!(
        format!("{err:#}").contains("requires Jira API v2"),
        "{err:#}"
    );
    let requests = server.received_requests().await.expect("recording");
    assert!(requests.is_empty(), "{requests:?}");
}
