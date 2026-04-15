use anyhow::{Context, Result};
use clap::Subcommand;
use jira_core::JiraClient;

#[derive(Debug, Subcommand)]
pub enum ApiCommand {
    /// Execute a raw Jira REST API call and print the JSON response.
    ///
    /// Examples:
    ///   jira api get /rest/api/3/serverInfo
    ///   jira api get /rest/api/3/issue/PROJ-123
    ///   jira api post /rest/api/3/issue --body '{"fields":{"project":{"key":"PROJ"},...}}'
    #[command(name = "get")]
    Get {
        /// API path (e.g. /rest/api/3/serverInfo)
        path: String,
    },
    #[command(name = "post")]
    Post {
        path: String,
        /// JSON body
        #[arg(long)]
        body: Option<String>,
    },
    #[command(name = "put")]
    Put {
        path: String,
        #[arg(long)]
        body: Option<String>,
    },
    #[command(name = "delete")]
    Delete { path: String },
    #[command(name = "patch")]
    Patch {
        path: String,
        #[arg(long)]
        body: Option<String>,
    },
}

pub async fn handle(cmd: ApiCommand, client: JiraClient) -> Result<()> {
    let (method, path, body_str) = match cmd {
        ApiCommand::Get { path } => ("GET", path, None),
        ApiCommand::Post { path, body } => ("POST", path, body),
        ApiCommand::Put { path, body } => ("PUT", path, body),
        ApiCommand::Delete { path } => ("DELETE", path, None),
        ApiCommand::Patch { path, body } => ("PATCH", path, body),
    };

    let body = body_str
        .map(|s| serde_json::from_str(&s).context("--body is not valid JSON"))
        .transpose()?;

    let result = client
        .raw_request(method, &path, body)
        .await
        .with_context(|| format!("{method} {path} failed"))?;

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
