use anyhow::{Context, Result};
use clap::Subcommand;
use jira_core::JiraClient;

#[derive(Debug, Subcommand)]
pub enum ApiCommand {
    /// Send a GET request and print the JSON response
    ///
    /// Useful for exploring the Jira API or fetching data not yet exposed
    /// by built-in commands. Response is pretty-printed JSON.
    /// Returns nothing (no output) on 204 No Content responses.
    ///
    /// Common paths:
    ///   /rest/api/3/serverInfo                      server info & tier
    ///   /rest/api/3/myself                          current user
    ///   /rest/api/3/issue/PROJ-123                  issue detail (raw)
    ///   /rest/api/3/issue/PROJ-123/transitions      available transitions
    ///   /rest/api/3/project/PROJ                    project metadata
    ///   /rest/agile/1.0/board                       all boards
    ///   /rest/agile/1.0/board/42/sprint             sprints for a board
    ///
    /// Examples:
    ///   jira api get /rest/api/3/serverInfo
    ///   jira api get /rest/api/3/myself
    ///   jira api get /rest/api/3/issue/PROJ-123/transitions
    #[command(name = "get")]
    Get {
        /// API path starting with /rest/... (e.g. /rest/api/3/serverInfo)
        path: String,
    },

    /// Send a POST request with a JSON body
    ///
    /// Prints the response JSON on success, or nothing on 201/204 No Content.
    ///
    /// Examples:
    ///   jira api post /rest/api/3/issue \
    ///     --body '{"fields":{"project":{"key":"PROJ"},"summary":"Test","issuetype":{"name":"Task"}}}'
    ///
    ///   jira api post /rest/api/3/issue/PROJ-123/comment \
    ///     --body '{"body":{"type":"doc","version":1,"content":[{"type":"paragraph","content":[{"type":"text","text":"Hello"}]}]}}'
    #[command(name = "post")]
    Post {
        /// API path starting with /rest/...
        path: String,
        /// Request body as a JSON string
        #[arg(long, value_name = "JSON")]
        body: Option<String>,
    },

    /// Send a PUT request with a JSON body (full resource replacement)
    ///
    /// PUT replaces the resource entirely. For partial field updates, use PATCH.
    ///
    /// Examples:
    ///   jira api put /rest/api/3/issue/PROJ-123 \
    ///     --body '{"fields":{"summary":"New title","priority":{"name":"High"}}}'
    #[command(name = "put")]
    Put {
        /// API path starting with /rest/...
        path: String,
        /// Request body as a JSON string
        #[arg(long, value_name = "JSON")]
        body: Option<String>,
    },

    /// Send a DELETE request — no output on success (204 No Content)
    ///
    /// Examples:
    ///   jira api delete /rest/api/3/issue/PROJ-123
    ///   jira api delete /rest/api/3/issue/PROJ-123/worklog/12345
    ///   jira api delete /rest/api/3/issue/PROJ-123/attachments/67890
    #[command(name = "delete")]
    Delete {
        /// API path starting with /rest/...
        path: String,
    },

    /// Send a PATCH request with a JSON body (partial update)
    ///
    /// PATCH applies partial updates — only the fields in the body are changed.
    ///
    /// Examples:
    ///   jira api patch /rest/api/3/issue/PROJ-123 \
    ///     --body '{"fields":{"priority":{"name":"High"}}}'
    #[command(name = "patch")]
    Patch {
        /// API path starting with /rest/...
        path: String,
        /// Request body as a JSON string
        #[arg(long, value_name = "JSON")]
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

    if let Some(value) = client
        .raw_request(method, &path, body)
        .await
        .with_context(|| format!("{method} {path} failed"))?
    {
        // 204 No Content → value is None, print nothing
        println!("{}", serde_json::to_string_pretty(&value)?);
    }
    Ok(())
}
