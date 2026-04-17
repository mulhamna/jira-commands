use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use jira_core::{config::JiraConfig, JiraClient};
use tracing_subscriber::{fmt, EnvFilter};

mod cli;
mod tui;

#[derive(Debug, Parser)]
#[command(
    name = "jira",
    about = "Jira CLI — terminal client for Atlassian Jira",
    long_about = "A fast Jira terminal client built in Rust.\n\nQuick start:\n  jira auth login                      Set up credentials\n  jira issue list                      List your assigned issues\n  jira issue list -p MYPROJ            List issues by project\n  jira issue view PROJ-123             View issue detail\n  jira issue create -p MYPROJ          Create an issue (interactive)\n  jira issue transition PROJ-123       Transition an issue (interactive)\n  jira issue attach PROJ-123 file.png  Upload attachment\n  jira issue worklog list PROJ-123     List worklogs on an issue\n  jira issue bulk-transition PROJ -q 'status = \"To Do\"'\n  jira plan list                       List Jira Plans (Premium)\n  jira api get /rest/api/3/serverInfo  Raw API passthrough\n  jira tui -p MYPROJ                   Launch interactive TUI\n\nConfig file: ~/.config/jira/config.toml\nEnv vars:    JIRA_URL, JIRA_EMAIL, JIRA_TOKEN",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging (sets RUST_LOG=debug)
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Create, list, view, update, delete, transition, attach, and bulk-operate on issues
    Issue {
        #[command(subcommand)]
        command: Box<cli::issue::IssueCommand>,
    },
    /// Manage credentials — login, logout, status, update
    Auth {
        #[command(subcommand)]
        command: cli::auth::AuthCommand,
    },

    /// Launch the interactive TUI — browse, search, and transition issues
    ///
    /// Keyboard shortcuts:
    ///   j / k  or  ↑ / ↓    Navigate the issue list
    ///   /                    Enter search mode — type a JQL query and press Enter
    ///   Esc                  Cancel search / go back
    ///   Enter                Open issue detail view
    ///   t                    Transition the selected issue (interactive picker)
    ///   o                    Open the selected issue in your browser
    ///   r                    Refresh the issue list
    ///   ?                    Show keyboard help overlay
    ///   q                    Quit
    ///
    /// Examples:
    ///   jira tui                   # uses default project from config, or your assigned issues
    ///   jira tui -p PROJ           # start filtered to a specific project
    Tui {
        /// Project key to filter issues (e.g. PROJ). Falls back to config default, then assignee = currentUser()
        #[arg(short, long, value_name = "PROJECT")]
        project: Option<String>,
    },

    /// Execute raw Jira REST API calls — GET, POST, PUT, PATCH, DELETE
    Api {
        #[command(subcommand)]
        command: cli::api::ApiCommand,
    },

    /// Manage Jira Plans / Advanced Roadmaps (requires Jira Premium)
    Plan {
        #[command(subcommand)]
        command: cli::plan::PlanCommand,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"))
    };

    fmt().with_env_filter(filter).with_target(false).init();

    match cli.command {
        Commands::Auth { command } => {
            cli::auth::handle(command).await?;
        }
        Commands::Issue { command } => {
            let client = build_client().context("Failed to initialize Jira client")?;
            let config = JiraConfig::load().unwrap_or_default();
            cli::issue::handle(*command, client, config.project).await?;
        }
        Commands::Tui { project } => {
            let client = build_client().context("Failed to initialize Jira client")?;
            let config = JiraConfig::load().unwrap_or_default();
            let effective_project = project.or(config.project);
            tui::run_tui(client, effective_project)
                .await
                .context("TUI error")?;
        }
        Commands::Api { command } => {
            let client = build_client().context("Failed to initialize Jira client")?;
            cli::api::handle(command, client).await?;
        }
        Commands::Plan { command } => {
            let client = build_client().context("Failed to initialize Jira client")?;
            cli::plan::handle(command, client).await?;
        }
    }

    Ok(())
}

fn build_client() -> Result<JiraClient> {
    let config = JiraConfig::load().unwrap_or_default();

    if config.base_url.is_empty() {
        anyhow::bail!(
            "Jira URL not configured. Run `jira auth login` or set JIRA_URL environment variable."
        );
    }

    if config.email.is_empty() {
        anyhow::bail!(
            "Email not configured. Run `jira auth login` or set JIRA_EMAIL environment variable."
        );
    }

    if config.token.is_none() {
        anyhow::bail!(
            "API token not found. Run `jira auth login` or set JIRA_TOKEN environment variable."
        );
    }

    Ok(JiraClient::new(config))
}
