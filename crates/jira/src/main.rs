use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use jira_commands::{cli, help_text, tui, version_check};
use jira_core::{config::JiraConfig, JiraClient};
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Debug, Parser)]
#[command(
    name = "jirac",
    about = "jirac — terminal client for the Jira ecosystem",
    long_about = help_text::ROOT_LONG_ABOUT,
    version,
    args_conflicts_with_subcommands = true
)]
struct Cli {
    /// Issue key shortcut. Use with --web to open in Jira.
    #[arg(value_name = "ISSUE")]
    issue: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,

    /// Open the issue shortcut in your browser (for example: jirac PROJ-123 --web)
    #[arg(long, requires = "issue")]
    web: bool,

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
    /// Manage Jira auth profiles, credentials, and active profile selection
    #[command(long_about = help_text::AUTH_LONG_ABOUT)]
    Auth {
        #[command(subcommand)]
        command: cli::auth::AuthCommand,
    },
    #[command(
        long_about = "Launch the interactive TUI to browse, search, update, and transition issues.\n\nKeyboard shortcuts:\n  j / k or ↑ / ↓   Navigate the issue list\n  Enter            Open split detail view (Summary / Versions / Comments / Worklog / Attachments / Subtasks / Links)\n  p                Open saved JQL queries\n  V                Browse project fix versions, preview backlog, press n to create, and e to edit metadata\n  T                Open theme picker\n  S                Show server summary\n  g                Show config summary\n  t                Transition the selected issue\n  C                Pick visible table columns and save preference\n  c                Create a new issue\n  e                Edit summary / description\n  y                Change issue type in a modal (native Jira move semantics)\n  M                Move issue to another project in a modal (native move, not clone+delete)\n  a                Open native assignee popup with searchable picker\n  ;                Add a comment\n  :                Add the same comment to many issues (JQL or explicit keys)\n  w                Add a single worklog\n  b                Add a bulk/range worklog with confirmation\n  l                Set labels\n  m                Open native component popup with searchable multi-select\n  v                Open native fix version popup with searchable multi-select\n  s                Open sprint picker\n  u                Upload an attachment\n  o                Open the selected issue in your browser\n  r                Refresh the issue list\n  n                Scan and open Jira mention notifications\n  R                Mark the selected notification issue as read\n  /                Enter search mode and run JQL\n  ?                Show keyboard help overlay\n  Esc              Cancel search / go back\n  q                Quit\n\nThe TUI keeps these actions inside overlays and modals. It does not exit to the shell for type changes or project moves. Bulk worklog and bulk comment submission use an in-modal confirmation step.\n\nExamples:\n  jirac tui\n      Uses the default project from config, or your assigned issues\n\n  jirac tui -p PROJ\n      Start filtered to a specific project"
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
    /// Register jirac-mcp with supported MCP clients like Claude, Cursor, Gemini CLI, OpenCode, or Codex
    Mcp {
        #[command(subcommand)]
        command: cli::mcp::McpCommand,
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

    if let Some(issue) = cli.issue {
        if cli.web {
            open_issue_shortcut(&issue)?;
            if let Some(notice) = &update_notice {
                eprintln!("{}", version_check::cli_message(notice));
            }
            return Ok(());
        }
    }

    let Some(command) = cli.command else {
        anyhow::bail!("No command provided. Run `jirac --help` for usage.");
    };

    match command {
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
        Commands::Mcp { command } => {
            cli::mcp::handle(command)?;
            if let Some(notice) = &update_notice {
                eprintln!("{}", version_check::cli_message(notice));
            }
        }
    }

    Ok(())
}

fn open_issue_shortcut(issue: &str) -> Result<()> {
    let config = JiraConfig::load().unwrap_or_default();

    if config.base_url.is_empty() {
        anyhow::bail!(
            "Jira URL not configured. Run `jirac auth login` or set JIRA_URL environment variable."
        );
    }

    let issue = issue.trim();
    if issue.is_empty() {
        anyhow::bail!("Issue key cannot be empty.");
    }
    if !issue.contains('-') {
        anyhow::bail!("Issue shortcut expects a full issue key like `PROJ-123`.");
    }

    let base = config.base_url.trim_end_matches('/');
    let url = format!("{base}/browse/{issue}");
    open::that(&url).with_context(|| format!("Failed to open browser for {issue}"))?;
    println!("Opened {url}");
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
