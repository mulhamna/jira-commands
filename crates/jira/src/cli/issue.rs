use anyhow::{Context, Result};
use clap::Subcommand;
use indicatif::{ProgressBar, ProgressStyle};
use inquire::{Select, Text};
use jira_core::{
    model::{CreateIssueRequest, UpdateIssueRequest},
    JiraClient,
};

#[derive(Debug, Subcommand)]
pub enum IssueCommand {
    /// List issues
    List {
        /// Project key filter
        #[arg(short, long)]
        project: Option<String>,
        /// Custom JQL query
        #[arg(long)]
        jql: Option<String>,
        /// Maximum number of results
        #[arg(short, long, default_value = "25")]
        limit: u32,
    },
    /// View issue details
    View {
        /// Issue key (e.g. PROJ-123)
        key: String,
    },
    /// Create a new issue
    Create {
        /// Project key
        #[arg(short, long)]
        project: Option<String>,
        /// Issue summary
        #[arg(short, long)]
        summary: Option<String>,
        /// Issue type (Bug, Story, Task, etc.)
        #[arg(short = 't', long, default_value = "Task")]
        issue_type: String,
        /// Assignee email
        #[arg(short, long)]
        assignee: Option<String>,
        /// Priority (Highest, High, Medium, Low, Lowest)
        #[arg(long)]
        priority: Option<String>,
    },
    /// Update an existing issue
    Update {
        /// Issue key (e.g. PROJ-123)
        key: String,
        /// New summary
        #[arg(short, long)]
        summary: Option<String>,
        /// New assignee email
        #[arg(short, long)]
        assignee: Option<String>,
        /// New priority
        #[arg(long)]
        priority: Option<String>,
    },
    /// Delete an issue
    Delete {
        /// Issue key (e.g. PROJ-123)
        key: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
    /// Transition an issue to a new status
    Transition {
        /// Issue key (e.g. PROJ-123)
        key: String,
        /// Transition name or ID (optional — interactive picker if omitted)
        transition: Option<String>,
    },
}

pub async fn handle(
    cmd: IssueCommand,
    client: JiraClient,
    default_project: Option<String>,
) -> Result<()> {
    match cmd {
        IssueCommand::List {
            project,
            jql,
            limit,
        } => list_issues(client, project.or(default_project), jql, limit).await,
        IssueCommand::View { key } => view_issue(client, key).await,
        IssueCommand::Create {
            project,
            summary,
            issue_type,
            assignee,
            priority,
        } => {
            create_issue(
                client,
                project.or(default_project),
                summary,
                issue_type,
                assignee,
                priority,
            )
            .await
        }
        IssueCommand::Update {
            key,
            summary,
            assignee,
            priority,
        } => update_issue(client, key, summary, assignee, priority).await,
        IssueCommand::Delete { key, force } => delete_issue(client, key, force).await,
        IssueCommand::Transition { key, transition } => {
            transition_issue(client, key, transition).await
        }
    }
}

async fn list_issues(
    client: JiraClient,
    project: Option<String>,
    jql: Option<String>,
    limit: u32,
) -> Result<()> {
    let jql_query = if let Some(jql) = jql {
        jql
    } else if let Some(proj) = &project {
        format!("project = {proj} ORDER BY updated DESC")
    } else {
        "assignee = currentUser() ORDER BY updated DESC".to_string()
    };

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message("Fetching issues...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let result = client
        .search_issues(&jql_query, None, Some(limit))
        .await
        .context("Failed to search issues")?;

    spinner.finish_and_clear();

    if result.issues.is_empty() {
        println!("No issues found.");
        return Ok(());
    }

    // Print header
    println!(
        "{:<12} {:<8} {:<20} {:<40}",
        "KEY", "TYPE", "STATUS", "SUMMARY"
    );
    println!("{}", "─".repeat(82));

    for issue in &result.issues {
        let summary = if issue.summary.len() > 38 {
            format!("{}…", &issue.summary[..37])
        } else {
            issue.summary.clone()
        };
        println!(
            "{:<12} {:<8} {:<20} {}",
            issue.key,
            truncate(&issue.issue_type, 7),
            truncate(&issue.status, 19),
            summary
        );
    }

    if let Some(total) = result.total {
        println!("\nShowing {} of {} issues", result.issues.len(), total);
    }

    Ok(())
}

async fn view_issue(client: JiraClient, key: String) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message(format!("Fetching {key}..."));
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let issue = client
        .get_issue(&key)
        .await
        .context("Failed to fetch issue")?;

    spinner.finish_and_clear();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("  {} — {}", issue.key, issue.summary);
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("  Type:       {}", issue.issue_type);
    println!("  Status:     {}", issue.status);
    println!("  Project:    {}", issue.project_key);
    if let Some(priority) = &issue.priority {
        println!("  Priority:   {priority}");
    }
    if let Some(assignee) = &issue.assignee {
        println!("  Assignee:   {assignee}");
    }
    if let Some(reporter) = &issue.reporter {
        println!("  Reporter:   {reporter}");
    }
    println!(
        "  Created:    {}",
        &issue.created[..10.min(issue.created.len())]
    );
    println!(
        "  Updated:    {}",
        &issue.updated[..10.min(issue.updated.len())]
    );

    if let Some(desc) = &issue.description {
        let text = jira_core::adf::adf_to_text(desc);
        if !text.is_empty() {
            println!();
            println!("  Description:");
            println!("  ───────────────────────────────────────");
            for line in text.lines() {
                println!("  {line}");
            }
        }
    }

    Ok(())
}

async fn create_issue(
    client: JiraClient,
    project: Option<String>,
    summary: Option<String>,
    issue_type: String,
    assignee: Option<String>,
    priority: Option<String>,
) -> Result<()> {
    let project_key = match project {
        Some(p) => p,
        None => Text::new("Project key:")
            .prompt()
            .context("Failed to read project key")?,
    };

    let summary = match summary {
        Some(s) => s,
        None => Text::new("Summary:")
            .prompt()
            .context("Failed to read summary")?,
    };

    let req = CreateIssueRequest {
        project_key,
        summary,
        description: None,
        issue_type,
        assignee,
        priority,
    };

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message("Creating issue...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let issue = client
        .create_issue(req)
        .await
        .context("Failed to create issue")?;

    spinner.finish_and_clear();

    println!("✓ Created: {} — {}", issue.key, issue.summary);

    Ok(())
}

async fn update_issue(
    client: JiraClient,
    key: String,
    summary: Option<String>,
    assignee: Option<String>,
    priority: Option<String>,
) -> Result<()> {
    if summary.is_none() && assignee.is_none() && priority.is_none() {
        println!("No fields to update. Use --summary, --assignee, or --priority.");
        return Ok(());
    }

    let req = UpdateIssueRequest {
        summary,
        assignee,
        priority,
        ..Default::default()
    };

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message(format!("Updating {key}..."));
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    client
        .update_issue(&key, req)
        .await
        .context("Failed to update issue")?;

    spinner.finish_and_clear();
    println!("✓ Updated: {key}");

    Ok(())
}

async fn delete_issue(client: JiraClient, key: String, force: bool) -> Result<()> {
    if !force {
        let confirm = inquire::Confirm::new(&format!("Delete {key}? This cannot be undone."))
            .with_default(false)
            .prompt()
            .context("Failed to read confirmation")?;

        if !confirm {
            println!("Aborted.");
            return Ok(());
        }
    }

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message(format!("Deleting {key}..."));
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    client
        .delete_issue(&key)
        .await
        .context("Failed to delete issue")?;

    spinner.finish_and_clear();
    println!("✓ Deleted: {key}");

    Ok(())
}

async fn transition_issue(
    client: JiraClient,
    key: String,
    transition: Option<String>,
) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message(format!("Fetching transitions for {key}..."));
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let transitions = client
        .get_transitions(&key)
        .await
        .context("Failed to fetch transitions")?;

    spinner.finish_and_clear();

    if transitions.is_empty() {
        println!("No transitions available for {key}.");
        return Ok(());
    }

    let transition_id = if let Some(name_or_id) = transition {
        // Find by name or ID
        transitions
            .iter()
            .find(|t| {
                t.get("id").and_then(|v| v.as_str()) == Some(&name_or_id)
                    || t.get("name").and_then(|v| v.as_str()) == Some(&name_or_id)
            })
            .and_then(|t| t.get("id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Transition '{}' not found", name_or_id))?
    } else {
        // Interactive picker
        let options: Vec<String> = transitions
            .iter()
            .filter_map(|t| {
                let id = t.get("id")?.as_str()?;
                let name = t.get("name")?.as_str()?;
                Some(format!("{name} [{id}]"))
            })
            .collect();

        let selected = Select::new("Select transition:", options.clone())
            .prompt()
            .context("Failed to select transition")?;

        // Extract ID from the selection "Name [ID]"
        selected
            .trim_end_matches(']')
            .rsplit('[')
            .next()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Failed to parse transition ID"))?
    };

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message(format!("Transitioning {key}..."));
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    client
        .transition_issue(&key, &transition_id)
        .await
        .context("Failed to transition issue")?;

    spinner.finish_and_clear();
    println!("✓ Transitioned: {key}");

    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len.saturating_sub(1)])
    }
}
