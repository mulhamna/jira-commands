use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use jira_commands::{cli, tui, version_check};
use jira_core::{config::JiraConfig, JiraClient};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Debug, Parser)]
#[command(
    name = "jirac",
    about = "jirac — terminal client for the Jira ecosystem",
    long_about = "A fast Jira terminal client built in Rust.\n\nQuick start:\n  jirac auth login                      Set up credentials\n  jirac auth profiles                   List saved profiles\n  jirac issue list                      List your assigned issues\n  jirac issue list -p MYPROJ            List issues by project\n  jirac issue view PROJ-123             View issue detail\n  jirac issue create -p MYPROJ          Create an issue (interactive)\n  jirac issue transition PROJ-123       Transition an issue (interactive)\n  jirac issue comment add PROJ-123      Add a comment to an issue\n  jirac issue attach PROJ-123 file.png  Upload attachment\n  jirac issue worklog list PROJ-123     List worklogs on an issue\n  jirac issue bulk-transition PROJ -q 'status = \"To Do\"'\n  jirac plan list                       List Jira Plans (Premium)\n  jirac api get /rest/api/3/serverInfo  Raw API passthrough\n  jirac tui -p MYPROJ                   Launch interactive TUI with split detail and popups\n\nInstallation:\n  See README.md for the install matrix\n  See INSTALL.md for detailed install instructions\n\nConfig file: ~/.config/jira/config.toml\nEnv vars:    JIRA_PROFILE, JIRA_URL, JIRA_EMAIL, JIRA_TOKEN",
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
    Issue {
        #[command(subcommand)]
        command: Box<cli::issue::IssueCommand>,
    },
    Auth {
        #[command(subcommand)]
        command: cli::auth::AuthCommand,
    },
    #[command(
        long_about = "Launch the interactive TUI to browse, search, update, and transition issues.\n\nKeyboard shortcuts:\n  j / k or ↑ / ↓   Navigate the issue list\n  Enter            Open split detail view\n  p                Open saved JQL queries\n  T                Open theme picker\n  S                Show server summary\n  g                Show config summary\n  t                Transition the selected issue\n  C                Pick visible table columns and save preference\n  c                Create a new issue\n  e                Edit summary / description\n  y                Change issue type in a modal (native Jira move semantics)\n  M                Move issue to another project in a modal (native move, not clone+delete)\n  a                Open native assignee popup with searchable picker\n  ;                Add a comment\n  w                Add a worklog\n  l                Set labels\n  m                Open native component popup with searchable multi-select\n  v                Open native fix version popup with searchable multi-select\n  s                Open sprint picker\n  u                Upload an attachment\n  o                Open the selected issue in your browser\n  r                Refresh the issue list\n  /                Enter search mode and run JQL\n  ?                Show keyboard help overlay\n  Esc              Cancel search / go back\n  q                Quit\n\nThe TUI keeps these actions inside overlays and modals. It does not exit to the shell for type changes or project moves.\n\nExamples:\n  jirac tui\n      Uses the default project from config, or your assigned issues\n\n  jirac tui -p PROJ\n      Start filtered to a specific project"
    )]
    Tui {
        #[arg(short, long, value_name = "PROJECT")]
        project: Option<String>,
    },
    Api {
        #[command(subcommand)]
        command: cli::api::ApiCommand,
    },
    Plan {
        #[command(subcommand)]
        command: cli::plan::PlanCommand,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
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
    let update_notice = version_check::check_for_update().await;

    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"))
    };

    fmt().with_env_filter(filter).with_target(false).init();

    match cli.command {
        Commands::Auth { command } => {
            cli::auth::handle(command).await?;
            if let Some(notice) = &update_notice {
                eprintln!("{}", version_check::cli_message(notice));
            }
        }
        Commands::Issue { command } => {
            let client = build_client().context("Failed to initialize Jira client")?;
            let config = JiraConfig::load().unwrap_or_default();
            cli::issue::handle(*command, client, config.project).await?;
            if let Some(notice) = &update_notice {
                eprintln!("{}", version_check::cli_message(notice));
            }
        }
        Commands::Tui { project } => {
            let client = build_client().context("Failed to initialize Jira client")?;
            let config = JiraConfig::load().unwrap_or_default();
            let effective_project = project.or(config.project);
            tui::run_tui(client, effective_project, update_notice)
                .await
                .context("TUI error")?;
        }
        Commands::Api { command } => {
            let client = build_client().context("Failed to initialize Jira client")?;
            cli::api::handle(command, client).await?;
            if let Some(notice) = &update_notice {
                eprintln!("{}", version_check::cli_message(notice));
            }
        }
        Commands::Plan { command } => {
            let client = build_client().context("Failed to initialize Jira client")?;
            cli::plan::handle(command, client).await?;
            if let Some(notice) = &update_notice {
                eprintln!("{}", version_check::cli_message(notice));
            }
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

    if config.requires_user_identity() && config.email.trim().is_empty() {
        anyhow::bail!(
            "User identity not configured. Run `jirac auth login` or set JIRA_EMAIL environment variable."
        );
    }

    if !config.token_present() {
        anyhow::bail!(
            "API token not found. Run `jirac auth login` or set JIRA_TOKEN environment variable."
        );
    }

    Ok(JiraClient::new(config))
}
