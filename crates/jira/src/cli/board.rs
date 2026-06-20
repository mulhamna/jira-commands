use anyhow::{Context, Result};
use clap::Subcommand;
use jira_core::{model::Issue, JiraClient};

use crate::cli::progress::spinner_new;

#[derive(Debug, Subcommand)]
pub enum BoardCommand {
    /// List Agile boards (optionally filtered by project key and type)
    List {
        /// Project key filter (e.g. PROJ)
        #[arg(short, long, value_name = "PROJECT")]
        project: Option<String>,
        /// Board type filter: scrum, kanban, simple
        #[arg(short = 't', long, value_name = "TYPE")]
        board_type: Option<String>,
        /// Output as JSON array
        #[arg(long)]
        json: bool,
    },

    /// Show a single board by ID
    Get {
        /// Board ID
        id: u64,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// List issues currently on a board
    Issues {
        /// Board ID
        id: u64,
        /// Optional JQL filter
        #[arg(long)]
        jql: Option<String>,
        /// Maximum number of issues to return (default: 50)
        #[arg(long, value_name = "N")]
        max: Option<u32>,
        /// Output as JSON array
        #[arg(long)]
        json: bool,
    },

    /// List backlog issues on a board (issues not in an active or future sprint)
    Backlog {
        /// Board ID
        id: u64,
        /// Optional JQL filter
        #[arg(long)]
        jql: Option<String>,
        /// Maximum number of issues to return (default: 50)
        #[arg(long, value_name = "N")]
        max: Option<u32>,
        /// Output as JSON array
        #[arg(long)]
        json: bool,
    },
}

pub async fn handle(cmd: BoardCommand, client: JiraClient) -> Result<()> {
    match cmd {
        BoardCommand::List {
            project,
            board_type,
            json,
        } => list_boards(client, project, board_type, json).await,
        BoardCommand::Get { id, json } => get_board(client, id, json).await,
        BoardCommand::Issues { id, jql, max, json } => {
            board_issues(client, id, jql, max, json, /*backlog=*/ false).await
        }
        BoardCommand::Backlog { id, jql, max, json } => {
            board_issues(client, id, jql, max, json, /*backlog=*/ true).await
        }
    }
}

async fn list_boards(
    client: JiraClient,
    project: Option<String>,
    board_type: Option<String>,
    json: bool,
) -> Result<()> {
    let spinner = spinner_new("Fetching boards...");
    let boards = client
        .list_boards(project.as_deref(), board_type.as_deref())
        .await
        .context("Failed to list boards")?;
    spinner.finish_and_clear();

    if json {
        println!("{}", serde_json::to_string_pretty(&boards)?);
        return Ok(());
    }

    if boards.is_empty() {
        println!("No boards found.");
        return Ok(());
    }

    for b in &boards {
        println!(
            "{:<8} {:<10} {:<10} {}",
            b.id,
            b.board_type,
            b.project_key.as_deref().unwrap_or("-"),
            b.name
        );
    }
    Ok(())
}

async fn get_board(client: JiraClient, id: u64, json: bool) -> Result<()> {
    let spinner = spinner_new(format!("Fetching board {id}..."));
    let board = client
        .get_board(id)
        .await
        .with_context(|| format!("Failed to fetch board {id}"))?;
    spinner.finish_and_clear();

    if json {
        println!("{}", serde_json::to_string_pretty(&board)?);
        return Ok(());
    }
    println!("ID:           {}", board.id);
    println!("Name:         {}", board.name);
    println!("Type:         {}", board.board_type);
    if let Some(p) = &board.project_key {
        println!("Project key:  {p}");
    }
    if let Some(pid) = board.project_id {
        println!("Project ID:   {pid}");
    }
    if !board.self_url.is_empty() {
        println!("URL:          {}", board.self_url);
    }
    Ok(())
}

async fn board_issues(
    client: JiraClient,
    id: u64,
    jql: Option<String>,
    max: Option<u32>,
    json: bool,
    backlog: bool,
) -> Result<()> {
    let label = if backlog { "backlog" } else { "issues" };
    let spinner = spinner_new(format!("Fetching {label} for board {id}..."));
    let issues: Vec<Issue> = if backlog {
        client.board_backlog(id, jql.as_deref(), max).await
    } else {
        client.board_issues(id, jql.as_deref(), max).await
    }
    .with_context(|| format!("Failed to fetch {label} for board {id}"))?;
    spinner.finish_and_clear();

    if json {
        println!("{}", serde_json::to_string_pretty(&issues)?);
        return Ok(());
    }

    if issues.is_empty() {
        println!("No issues.");
        return Ok(());
    }

    for i in &issues {
        println!("{:<14} {:<14} {}", i.key, i.status, i.summary);
    }
    Ok(())
}
