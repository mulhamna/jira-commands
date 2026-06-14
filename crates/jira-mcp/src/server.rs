use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::{
        stdio,
        streamable_http_server::{
            session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
        },
    },
    ErrorData, Json, ServerHandler, ServiceExt,
};
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::{
    app::JiraApp,
    error::AppResult,
    models::{
        ApiRequestArgs, ArchiveArgs, AuthSetCredentialsArgs, BatchArgs, BulkCommentArgs,
        BulkTransitionArgs, BulkUpdateArgs, CommentAddArgs, IssueAttachArgs, IssueCloneArgs,
        IssueCreateArgs, IssueDeleteArgs, IssueFieldsArgs, IssueKeyArgs, IssueLinkCreateArgs,
        IssueLinkDeleteArgs, IssueListArgs, IssueNotificationsArgs, IssueStandupArgs,
        IssueTransitionArgs, IssueTypesListArgs, IssueUpdateArgs, ProjectKeyArgs,
        ProjectVersionCreateArgs, ProjectVersionUpdateArgs, RemoteLinkAddArgs,
        RemoteLinkDeleteArgs, SprintAddIssueArgs, SprintCreateArgs, SprintDeleteArgs,
        SprintListArgs, SprintSummaryArgs, SprintUpdateArgs, ToolResponse, WatcherAddArgs,
        WatcherRemoveArgs, WorklogAddArgs, WorklogDeleteArgs,
    },
};

#[derive(Clone)]
pub struct JiraMcpServer {
    app: JiraApp,
    tool_router: ToolRouter<Self>,
}

impl JiraMcpServer {
    pub fn new() -> Self {
        Self {
            app: JiraApp,
            tool_router: Self::tool_router(),
        }
    }

    fn respond(&self, result: AppResult<Value>) -> Result<Json<ToolResponse>, ErrorData> {
        result
            .map(|value| Json(ToolResponse { result: value }))
            .map_err(|err| err.to_mcp())
    }
}

impl Default for JiraMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for JiraMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(
                Implementation::new("jira-commands", env!("CARGO_PKG_VERSION"))
                    .with_title("jira-commands MCP")
                    .with_description("Typed Jira tools for MCP clients powered by jira-core"),
            )
            .with_instructions(
                "Use the jira_* tools for Jira issue operations, worklogs, plans, auth, and raw REST access. Destructive tools require confirm=true.",
            )
    }
}

#[tool_router(router = tool_router)]
impl JiraMcpServer {
    #[tool(
        name = "jira_auth_status",
        description = "Show Jira auth configuration, token presence, and config path"
    )]
    pub async fn jira_auth_status(&self) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.auth_status())
    }

    #[tool(
        name = "jira_auth_set_credentials",
        description = "Write Jira credentials and shared config fields without prompting"
    )]
    pub async fn jira_auth_set_credentials(
        &self,
        Parameters(args): Parameters<AuthSetCredentialsArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.auth_set_credentials(args))
    }

    #[tool(
        name = "jira_auth_logout",
        description = "Remove the stored Jira API token"
    )]
    pub async fn jira_auth_logout(&self) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.auth_logout())
    }

    #[tool(
        name = "jira_issue_list",
        description = "List Jira issues by project, JQL, or the current assignee"
    )]
    pub async fn jira_issue_list(
        &self,
        Parameters(args): Parameters<IssueListArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.issue_list(args).await)
    }

    #[tool(
        name = "jira_issue_standup",
        description = "Generate a structured daily standup summary from current-user Jira issues"
    )]
    pub async fn jira_issue_standup(
        &self,
        Parameters(args): Parameters<IssueStandupArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.issue_standup(args).await)
    }

    #[tool(
        name = "jira_issue_sprint_summary",
        description = "Summarize sprint issues by status and assignee for a project"
    )]
    pub async fn jira_issue_sprint_summary(
        &self,
        Parameters(args): Parameters<SprintSummaryArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.issue_sprint_summary(args).await)
    }

    #[tool(
        name = "jira_issue_notifications",
        description = "Scan recent Jira mentions from issue descriptions and comments"
    )]
    pub async fn jira_issue_notifications(
        &self,
        Parameters(args): Parameters<IssueNotificationsArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.issue_notifications(args).await)
    }

    #[tool(
        name = "jira_issue_view",
        description = "Fetch full details for a Jira issue"
    )]
    pub async fn jira_issue_view(
        &self,
        Parameters(args): Parameters<IssueKeyArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.issue_view(args).await)
    }

    #[tool(
        name = "jira_issue_types_list",
        description = "List available issue types for a Jira project"
    )]
    pub async fn jira_issue_types_list(
        &self,
        Parameters(args): Parameters<IssueTypesListArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.issue_types_list(args).await)
    }

    #[tool(
        name = "jira_issue_fields",
        description = "List available Jira fields for a project and optional issue type"
    )]
    pub async fn jira_issue_fields(
        &self,
        Parameters(args): Parameters<IssueFieldsArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.issue_fields(args).await)
    }

    #[tool(
        name = "jira_sprint_list",
        description = "List Jira sprints for a project, optionally filtered by sprint state"
    )]
    pub async fn jira_sprint_list(
        &self,
        Parameters(args): Parameters<SprintListArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.sprint_list(args).await)
    }

    #[tool(
        name = "jira_sprint_create",
        description = "Create a sprint on a Jira board"
    )]
    pub async fn jira_sprint_create(
        &self,
        Parameters(args): Parameters<SprintCreateArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.sprint_create(args).await)
    }

    #[tool(
        name = "jira_sprint_update",
        description = "Update Jira sprint metadata or lifecycle state"
    )]
    pub async fn jira_sprint_update(
        &self,
        Parameters(args): Parameters<SprintUpdateArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.sprint_update(args).await)
    }

    #[tool(
        name = "jira_sprint_delete",
        description = "Delete a Jira sprint; requires confirm=true"
    )]
    pub async fn jira_sprint_delete(
        &self,
        Parameters(args): Parameters<SprintDeleteArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.sprint_delete(args).await)
    }

    #[tool(
        name = "jira_sprint_add_issue",
        description = "Add an issue to a Jira sprint"
    )]
    pub async fn jira_sprint_add_issue(
        &self,
        Parameters(args): Parameters<SprintAddIssueArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.sprint_add_issue(args).await)
    }

    #[tool(name = "jira_watcher_list", description = "List watchers on a Jira issue")]
    pub async fn jira_watcher_list(
        &self,
        Parameters(args): Parameters<IssueKeyArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.watcher_list(args).await)
    }

    #[tool(
        name = "jira_watcher_add",
        description = "Add a watcher to a Jira issue (defaults to the current authenticated user)"
    )]
    pub async fn jira_watcher_add(
        &self,
        Parameters(args): Parameters<WatcherAddArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.watcher_add(args).await)
    }

    #[tool(
        name = "jira_watcher_remove",
        description = "Remove a watcher from a Jira issue (destructive, requires confirm=true)"
    )]
    pub async fn jira_watcher_remove(
        &self,
        Parameters(args): Parameters<WatcherRemoveArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.watcher_remove(args).await)
    }

    #[tool(
        name = "jira_project_components_list",
        description = "List Jira components for a project"
    )]
    pub async fn jira_project_components_list(
        &self,
        Parameters(args): Parameters<ProjectKeyArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.project_component_list(args).await)
    }

    #[tool(
        name = "jira_project_versions_list",
        description = "List Jira project versions / fix versions"
    )]
    pub async fn jira_project_versions_list(
        &self,
        Parameters(args): Parameters<ProjectKeyArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.project_version_list(args).await)
    }

    #[tool(
        name = "jira_project_version_create",
        description = "Create a Jira project version"
    )]
    pub async fn jira_project_version_create(
        &self,
        Parameters(args): Parameters<ProjectVersionCreateArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.project_version_create(args).await)
    }

    #[tool(
        name = "jira_project_version_update",
        description = "Update Jira project version metadata"
    )]
    pub async fn jira_project_version_update(
        &self,
        Parameters(args): Parameters<ProjectVersionUpdateArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.project_version_update(args).await)
    }

    #[tool(
        name = "jira_issue_transitions_list",
        description = "List available workflow transitions for a Jira issue"
    )]
    pub async fn jira_issue_transitions_list(
        &self,
        Parameters(args): Parameters<IssueKeyArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.issue_transitions_list(args).await)
    }

    #[tool(name = "jira_issue_create", description = "Create a Jira issue")]
    pub async fn jira_issue_create(
        &self,
        Parameters(args): Parameters<IssueCreateArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.issue_create(args).await)
    }

    #[tool(
        name = "jira_issue_update",
        description = "Update fields on a Jira issue"
    )]
    pub async fn jira_issue_update(
        &self,
        Parameters(args): Parameters<IssueUpdateArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.issue_update(args).await)
    }

    #[tool(
        name = "jira_issue_delete",
        description = "Delete a Jira issue permanently; requires confirm=true"
    )]
    pub async fn jira_issue_delete(
        &self,
        Parameters(args): Parameters<IssueDeleteArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.issue_delete(args).await)
    }

    #[tool(
        name = "jira_issue_clone",
        description = "Clone a Jira issue into the same or another project; deleting the original requires move_original=true and confirm=true"
    )]
    pub async fn jira_issue_clone(
        &self,
        Parameters(args): Parameters<IssueCloneArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.issue_clone(args).await)
    }

    #[tool(
        name = "jira_issue_transition",
        description = "Transition a Jira issue by workflow transition name or ID"
    )]
    pub async fn jira_issue_transition(
        &self,
        Parameters(args): Parameters<IssueTransitionArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.issue_transition(args).await)
    }

    #[tool(
        name = "jira_issue_attach",
        description = "Attach local files or inline base64 payloads to a Jira issue"
    )]
    pub async fn jira_issue_attach(
        &self,
        Parameters(args): Parameters<IssueAttachArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.issue_attach(args).await)
    }

    #[tool(
        name = "jira_comment_list",
        description = "List comments on a Jira issue"
    )]
    pub async fn jira_comment_list(
        &self,
        Parameters(args): Parameters<IssueKeyArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.comment_list(args).await)
    }

    #[tool(
        name = "jira_comment_add",
        description = "Add a Markdown comment to a Jira issue"
    )]
    pub async fn jira_comment_add(
        &self,
        Parameters(args): Parameters<CommentAddArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.comment_add(args).await)
    }

    #[tool(
        name = "jira_issue_bulk_comment",
        description = "Add the same comment to multiple Jira issues selected by JQL or explicit keys; requires confirm=true"
    )]
    pub async fn jira_issue_bulk_comment(
        &self,
        Parameters(args): Parameters<BulkCommentArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.issue_bulk_comment(args).await)
    }

    #[tool(
        name = "jira_issue_link_types_list",
        description = "List available Jira issue link types such as blocks, relates to, and duplicates"
    )]
    pub async fn jira_issue_link_types_list(&self) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.issue_link_types_list().await)
    }

    #[tool(
        name = "jira_issue_link_create",
        description = "Create a link between two Jira issues using a Jira link type name"
    )]
    pub async fn jira_issue_link_create(
        &self,
        Parameters(args): Parameters<IssueLinkCreateArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.issue_link_create(args).await)
    }

    #[tool(
        name = "jira_issue_link_delete",
        description = "Delete a Jira issue link by link ID; requires confirm=true"
    )]
    pub async fn jira_issue_link_delete(
        &self,
        Parameters(args): Parameters<IssueLinkDeleteArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.issue_link_delete(args).await)
    }

    #[tool(
        name = "jira_remote_link_list",
        description = "List remote links attached to a Jira issue"
    )]
    pub async fn jira_remote_link_list(
        &self,
        Parameters(args): Parameters<IssueKeyArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.remote_link_list(args).await)
    }

    #[tool(
        name = "jira_remote_link_add",
        description = "Attach a remote URL link to a Jira issue"
    )]
    pub async fn jira_remote_link_add(
        &self,
        Parameters(args): Parameters<RemoteLinkAddArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.remote_link_add(args).await)
    }

    #[tool(
        name = "jira_remote_link_delete",
        description = "Delete a remote link from a Jira issue; requires confirm=true"
    )]
    pub async fn jira_remote_link_delete(
        &self,
        Parameters(args): Parameters<RemoteLinkDeleteArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.remote_link_delete(args).await)
    }

    #[tool(
        name = "jira_worklog_list",
        description = "List worklogs on a Jira issue"
    )]
    pub async fn jira_worklog_list(
        &self,
        Parameters(args): Parameters<IssueKeyArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.worklog_list(args).await)
    }

    #[tool(
        name = "jira_worklog_add",
        description = "Add a worklog entry to a Jira issue"
    )]
    pub async fn jira_worklog_add(
        &self,
        Parameters(args): Parameters<WorklogAddArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.worklog_add(args).await)
    }

    #[tool(
        name = "jira_worklog_delete",
        description = "Delete a worklog entry from a Jira issue"
    )]
    pub async fn jira_worklog_delete(
        &self,
        Parameters(args): Parameters<WorklogDeleteArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.worklog_delete(args).await)
    }

    #[tool(
        name = "jira_issue_bulk_transition",
        description = "Transition all issues matching a JQL query; requires confirm=true"
    )]
    pub async fn jira_issue_bulk_transition(
        &self,
        Parameters(args): Parameters<BulkTransitionArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.issue_bulk_transition(args).await)
    }

    #[tool(
        name = "jira_issue_bulk_update",
        description = "Bulk-update assignee and/or priority for issues matching a JQL query; requires confirm=true"
    )]
    pub async fn jira_issue_bulk_update(
        &self,
        Parameters(args): Parameters<BulkUpdateArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.issue_bulk_update(args).await)
    }

    #[tool(
        name = "jira_issue_batch",
        description = "Run typed create, update, transition, and archive issue operations in sequence; requires confirm=true"
    )]
    pub async fn jira_issue_batch(
        &self,
        Parameters(args): Parameters<BatchArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.issue_batch(args).await)
    }

    #[tool(
        name = "jira_issue_archive",
        description = "Archive all issues matching a JQL query; requires confirm=true"
    )]
    pub async fn jira_issue_archive(
        &self,
        Parameters(args): Parameters<ArchiveArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.issue_archive(args).await)
    }

    #[tool(
        name = "jira_plan_list",
        description = "List Jira Plans / Advanced Roadmaps plans"
    )]
    pub async fn jira_plan_list(&self) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.plan_list().await)
    }

    #[tool(
        name = "jira_api_request",
        description = "Execute a raw Jira REST API request with a typed JSON body and query parameters"
    )]
    pub async fn jira_api_request(
        &self,
        Parameters(args): Parameters<ApiRequestArgs>,
    ) -> Result<Json<ToolResponse>, ErrorData> {
        self.respond(self.app.api_request(args).await)
    }
}

pub async fn run_stdio() -> anyhow::Result<()> {
    let server = JiraMcpServer::new().serve(stdio()).await?;
    server.waiting().await?;
    Ok(())
}

pub async fn run_streamable_http(host: &str, port: u16, path: &str) -> anyhow::Result<()> {
    let bind_address = format!("{host}:{port}");
    let cancellation_token = CancellationToken::new();
    let allowed_hosts = [
        format!("{host}:{port}"),
        host.to_string(),
        format!("127.0.0.1:{port}"),
        "127.0.0.1".to_string(),
        format!("localhost:{port}"),
        "localhost".to_string(),
    ];
    let service = StreamableHttpService::new(
        || Ok(JiraMcpServer::new()),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default()
            .with_cancellation_token(cancellation_token.child_token())
            .with_allowed_hosts(allowed_hosts),
    );

    let router = axum::Router::new().nest_service(path, service);
    let listener = tokio::net::TcpListener::bind(&bind_address).await?;

    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            let _ = tokio::signal::ctrl_c().await;
            cancellation_token.cancel();
        })
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use rmcp::{
        model::{CallToolRequestParams, ClientCapabilities, ClientInfo, Implementation},
        transport::StreamableHttpClientTransport,
        ClientHandler, ServiceExt,
    };
    use serial_test::serial;
    use tempfile::TempDir;

    use super::*;

    #[derive(Default, Clone)]
    struct TestClient;

    impl ClientHandler for TestClient {}

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

    fn set_test_env(temp_dir: &TempDir) {
        set_config_home_vars(temp_dir);
        std::env::remove_var("JIRA_URL");
        std::env::remove_var("JIRA_EMAIL");
        std::env::remove_var("JIRA_TOKEN");
    }

    fn clear_test_env() {
        clear_config_home_vars();
        std::env::remove_var("JIRA_URL");
        std::env::remove_var("JIRA_EMAIL");
        std::env::remove_var("JIRA_TOKEN");
    }

    fn credentials_args() -> serde_json::Map<String, serde_json::Value> {
        serde_json::json!({
            "url": "https://example.atlassian.net",
            "email": "dev@example.com",
            "token": "secret"
        })
        .as_object()
        .cloned()
        .expect("object")
    }

    #[tokio::test]
    #[serial]
    async fn stdio_transport_smoke_test() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        set_test_env(&temp_dir);

        let (server_transport, client_transport) = tokio::io::duplex(64 * 1024);
        let server_task: tokio::task::JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
            let service = JiraMcpServer::new().serve(server_transport).await?;
            service.waiting().await?;
            Ok(())
        });

        let client = TestClient.serve(client_transport).await?;
        let tools = client.list_all_tools().await?;
        assert!(tools.iter().any(|tool| tool.name == "jira_auth_status"));
        assert!(tools.iter().any(|tool| tool.name == "jira_issue_clone"));
        assert!(tools
            .iter()
            .any(|tool| tool.name == "jira_issue_bulk_comment"));
        assert!(tools.iter().any(|tool| tool.name == "jira_issue_batch"));
        assert!(tools.iter().any(|tool| tool.name == "jira_issue_standup"));
        assert!(tools
            .iter()
            .any(|tool| tool.name == "jira_issue_sprint_summary"));
        assert!(tools
            .iter()
            .any(|tool| tool.name == "jira_issue_notifications"));

        client
            .call_tool(CallToolRequestParams::new("jira_auth_status"))
            .await?;
        client
            .call_tool(
                CallToolRequestParams::new("jira_auth_set_credentials")
                    .with_arguments(credentials_args()),
            )
            .await?;

        client.cancel().await?;
        server_task.await??;
        clear_test_env();
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn streamable_http_transport_smoke_test() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        set_test_env(&temp_dir);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let service = StreamableHttpService::new(
            || Ok(JiraMcpServer::new()),
            LocalSessionManager::default().into(),
            StreamableHttpServerConfig::default()
                .with_allowed_hosts([format!("127.0.0.1:{}", addr.port()), "127.0.0.1".into()]),
        );
        let router = axum::Router::new().nest_service("/mcp", service);
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        let server_task = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await
                .expect("http server");
        });

        let client_info = ClientInfo::new(
            ClientCapabilities::default(),
            Implementation::new("test-client", "0.1.0"),
        );
        let client = client_info
            .serve(StreamableHttpClientTransport::from_uri(format!(
                "http://127.0.0.1:{}/mcp",
                addr.port()
            )))
            .await?;

        let tools = client.list_all_tools().await?;
        assert!(tools
            .iter()
            .any(|tool| tool.name == "jira_auth_set_credentials"));

        client
            .call_tool(CallToolRequestParams::new("jira_auth_status"))
            .await?;
        client
            .call_tool(
                CallToolRequestParams::new("jira_auth_set_credentials")
                    .with_arguments(credentials_args()),
            )
            .await?;

        client.cancel().await?;
        let _ = shutdown_tx.send(());
        let _ = server_task.await;
        clear_test_env();
        Ok(())
    }
}
