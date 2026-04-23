use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use jira_core::{config::JiraConfig, JiraClient};
use tracing_subscriber::{fmt, EnvFilter};

mod cli;
mod datetime;
mod tui;

#[derive(Debug, Parser)]
#[command(
    name = "jirac",
    about = "jirac — terminal client for the Jira ecosystem",
    long_about = "A fast Jira terminal client built in Rust.\n\nQuick start:\n  jirac auth login                      Set up credentials\n  jirac issue list                      List your assigned issues\n  jirac issue list -p MYPROJ            List issues by project\n  jirac issue view PROJ-123             View issue detail\n  jirac issue create -p MYPROJ          Create an issue (interactive)\n  jirac issue transition PROJ-123       Transition an issue (interactive)\n  jirac issue comment add PROJ-123      Add a comment to an issue\n  jirac issue attach PROJ-123 file.png  Upload attachment\n  jirac issue worklog list PROJ-123     List worklogs on an issue\n  jirac issue bulk-transition PROJ -q 'status = \"To Do\"'\n  jirac plan list                       List Jira Plans (Premium)\n  jirac api get /rest/api/3/serverInfo  Raw API passthrough\n  jirac tui -p MYPROJ                   Launch interactive TUI\n\nConfig file: ~/.config/jira/config.toml\nEnv vars:    JIRA_URL, JIRA_EMAIL, JIRA_TOKEN",
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
    #[command(
        long_about = "Launch the interactive TUI to browse, search, update, and transition issues.\n\nKeyboard shortcuts:\n  j / k or ↑ / ↓   Navigate the issue list\n  Enter            Open issue detail view\n  t                Transition the selected issue\n  C                Pick visible table columns and save preference\n  c                Create a new issue\n  e                Edit summary / assignee / priority\n  a                Assign the selected issue with searchable picker\n  ;                Add a comment\n  w                Add a worklog\n  l                Set labels\n  m                Set project-scoped components via searchable picker\n  u                Upload an attachment\n  o                Open the selected issue in your browser\n  r                Refresh the issue list\n  /                Enter search mode and run JQL\n  ?                Show keyboard help overlay\n  Esc              Cancel search / go back\n  q                Quit\n\nExamples:\n  jirac tui\n      Uses the default project from config, or your assigned issues\n\n  jirac tui -p PROJ\n      Start filtered to a specific project"
    )]
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
    // Detect if invoked via the legacy 'jira' binary name and warn the user.
    if std::env::args().next().is_some_and(|a| {
        let name = std::path::Path::new(&a)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        name == "jira"
    }) {
        eprintln!(
            "warning: the 'jira' binary is deprecated and will be removed in a future release."
        );
        eprintln!("         Please switch to 'jirac'. Everything else works the same.");
        eprintln!();
    }

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
            "Jira URL not configured. Run `jirac auth login` or set JIRA_URL environment variable."
        );
    }

    if config.email.is_empty() {
        anyhow::bail!(
            "Email not configured. Run `jirac auth login` or set JIRA_EMAIL environment variable."
        );
    }

    if config.token.is_none() {
        anyhow::bail!(
            "API token not found. Run `jirac auth login` or set JIRA_TOKEN environment variable."
        );
    }

    Ok(JiraClient::new(config))
}
