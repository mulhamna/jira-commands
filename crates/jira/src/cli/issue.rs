use std::collections::HashMap;

use anyhow::{Context, Result};
use clap::Subcommand;
use indicatif::{ProgressBar, ProgressStyle};
use inquire::{MultiSelect, Select, Text};
use jira_core::{
    model::{
        field::{FieldKind, FieldValue},
        CreateIssueRequest, CreateIssueRequestV2, UpdateIssueRequest,
    },
    FieldCache, IssueType, JiraClient,
};
use serde_json;
use serde_json::Value;

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
    /// Create a new issue (interactive — prompts for required custom fields)
    Create {
        /// Project key
        #[arg(short, long)]
        project: Option<String>,
        /// Issue summary
        #[arg(short, long)]
        summary: Option<String>,
        /// Issue type (Bug, Story, Task, etc.) — omit for interactive picker
        #[arg(short = 't', long)]
        issue_type: Option<String>,
        /// Assignee email
        #[arg(short, long)]
        assignee: Option<String>,
        /// Priority (Highest, High, Medium, Low, Lowest)
        #[arg(long)]
        priority: Option<String>,
        /// Skip custom field prompts (use defaults/omit optional fields)
        #[arg(long)]
        no_custom_fields: bool,
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
    /// Attach file(s) to an issue
    Attach {
        /// Issue key (e.g. PROJ-123)
        key: String,
        /// Path(s) to file(s) to attach
        #[arg(required = true)]
        files: Vec<std::path::PathBuf>,
    },
    /// List available fields for a project / issue type
    Fields {
        /// Project key
        #[arg(short, long)]
        project: Option<String>,
        /// Issue type name or ID filter
        #[arg(long)]
        issue_type: Option<String>,
        /// Show only required fields
        #[arg(long)]
        required_only: bool,
    },
    /// Worklog management (log time, list, delete)
    Worklog {
        /// Issue key (e.g. PROJ-123)
        key: String,
        #[command(subcommand)]
        command: WorklogCommand,
    },
    /// Transition all issues matching a JQL query in bulk
    BulkTransition {
        /// JQL query selecting the issues
        #[arg(long)]
        jql: String,
        /// Transition name or ID to apply
        #[arg(long)]
        to: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
    /// Update fields on all issues matching a JQL query in bulk
    BulkUpdate {
        /// JQL query selecting the issues
        #[arg(long)]
        jql: String,
        /// New assignee email
        #[arg(long)]
        assignee: Option<String>,
        /// New priority
        #[arg(long)]
        priority: Option<String>,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
    /// Archive all issues matching a JQL query
    Archive {
        /// JQL query selecting issues to archive
        #[arg(long)]
        jql: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
    /// Guided interactive JQL query builder
    Jql {
        /// Run the generated JQL immediately
        #[arg(long)]
        run: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum WorklogCommand {
    /// List worklogs for an issue
    List,
    /// Log time on an issue
    Add {
        /// Time spent (Jira format: "2h 30m", "1d", "45m")
        #[arg(short, long)]
        time: String,
        /// Optional comment
        #[arg(short, long)]
        comment: Option<String>,
    },
    /// Delete a worklog entry
    Delete {
        /// Worklog ID
        id: String,
        /// Skip confirmation
        #[arg(short, long)]
        force: bool,
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
            no_custom_fields,
        } => {
            create_issue(
                client,
                project.or(default_project),
                summary,
                issue_type,
                assignee,
                priority,
                no_custom_fields,
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
        IssueCommand::Attach { key, files } => attach_files(client, key, files).await,
        IssueCommand::Fields {
            project,
            issue_type,
            required_only,
        } => {
            list_fields(
                client,
                project.or(default_project),
                issue_type,
                required_only,
            )
            .await
        }
        IssueCommand::Worklog { key, command } => worklog(client, key, command).await,
        IssueCommand::BulkTransition { jql, to, force } => {
            bulk_transition(client, jql, to, force).await
        }
        IssueCommand::BulkUpdate {
            jql,
            assignee,
            priority,
            force,
        } => bulk_update(client, jql, assignee, priority, force).await,
        IssueCommand::Archive { jql, force } => archive(client, jql, force).await,
        IssueCommand::Jql { run } => jql_builder(client, run).await,
    }
}

// ─── list ────────────────────────────────────────────────────────────────────

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

    let spinner = spinner_new("Fetching issues...");
    let result = client
        .search_issues(&jql_query, None, Some(limit))
        .await
        .context("Failed to search issues")?;
    spinner.finish_and_clear();

    if result.issues.is_empty() {
        println!("No issues found.");
        return Ok(());
    }

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

// ─── view ────────────────────────────────────────────────────────────────────

async fn view_issue(client: JiraClient, key: String) -> Result<()> {
    let spinner = spinner_new(format!("Fetching {key}..."));
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

    if !issue.attachments.is_empty() {
        println!();
        println!("  Attachments ({}):", issue.attachments.len());
        for a in &issue.attachments {
            println!("    • {} ({}, {} bytes)", a.filename, a.mime_type, a.size);
        }
    }

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

// ─── create ──────────────────────────────────────────────────────────────────

async fn create_issue(
    client: JiraClient,
    project: Option<String>,
    summary: Option<String>,
    issue_type: Option<String>,
    assignee: Option<String>,
    priority: Option<String>,
    no_custom_fields: bool,
) -> Result<()> {
    // 1. Project key
    let project_key = match project {
        Some(p) => p,
        None => Text::new("Project key:")
            .prompt()
            .context("Failed to read project key")?,
    };

    // 2. Issue type — interactive picker if not supplied
    let (issue_type_name, issue_type_id) =
        resolve_issue_type(&client, &project_key, issue_type).await?;

    // 3. Summary
    let summary = match summary {
        Some(s) => s,
        None => Text::new("Summary:")
            .prompt()
            .context("Failed to read summary")?,
    };

    // 4. Custom fields — only if not disabled
    let custom_fields = if no_custom_fields {
        HashMap::new()
    } else {
        collect_custom_fields(&client, &project_key, &issue_type_id).await?
    };

    let req = CreateIssueRequestV2 {
        project_key,
        summary,
        description: None,
        issue_type: issue_type_name,
        assignee,
        priority,
        custom_fields,
    };

    let spinner = spinner_new("Creating issue...");
    let issue = client
        .create_issue_v2(req)
        .await
        .context("Failed to create issue")?;
    spinner.finish_and_clear();

    println!("✓ Created: {} — {}", issue.key, issue.summary);
    Ok(())
}

/// Resolve issue type: use the provided name directly (skip API call) or show a picker.
async fn resolve_issue_type(
    client: &JiraClient,
    project_key: &str,
    issue_type: Option<String>,
) -> Result<(String, String)> {
    // If user gave a name, we still need the ID for field fetching — try to look it up
    let spinner = spinner_new(format!("Fetching issue types for {project_key}..."));
    let types_result = client.get_issue_types(project_key).await;
    spinner.finish_and_clear();

    match types_result {
        Ok(types) if !types.is_empty() => {
            if let Some(name) = issue_type {
                // Find matching type by name (case-insensitive)
                if let Some(t) = types
                    .iter()
                    .find(|t| t.name.to_lowercase() == name.to_lowercase())
                {
                    return Ok((t.name.clone(), t.id.clone()));
                }
                // Not found — use name as-is with empty ID (will skip custom field prompts)
                return Ok((name, String::new()));
            }

            // Interactive picker
            let options: Vec<String> = types.iter().map(|t| t.name.clone()).collect();
            let selected = Select::new("Issue type:", options)
                .prompt()
                .context("Failed to select issue type")?;

            let id = types
                .iter()
                .find(|t| t.name == selected)
                .map(|t| t.id.clone())
                .unwrap_or_default();

            Ok((selected, id))
        }
        _ => {
            // API call failed or returned empty — fall back gracefully
            let name = match issue_type {
                Some(n) => n,
                None => Text::new("Issue type (e.g. Task, Bug, Story):")
                    .with_default("Task")
                    .prompt()
                    .context("Failed to read issue type")?,
            };
            Ok((name, String::new()))
        }
    }
}

/// Prompt for required custom fields that are not standard (summary/assignee/priority/type).
async fn collect_custom_fields(
    client: &JiraClient,
    project_key: &str,
    issue_type_id: &str,
) -> Result<HashMap<String, FieldValue>> {
    if issue_type_id.is_empty() {
        return Ok(HashMap::new());
    }

    let mut cache = FieldCache::new();
    let fields = cache.get_or_fetch(client, project_key, issue_type_id).await;

    let fields = match fields {
        Ok(f) => f,
        Err(_) => return Ok(HashMap::new()), // soft fail — don't block issue creation
    };

    // Standard fields handled by CLI flags — skip them
    const SKIP_IDS: &[&str] = &[
        "summary",
        "description",
        "issuetype",
        "project",
        "assignee",
        "reporter",
        "priority",
        "status",
        "created",
        "updated",
        "comment",
        "attachment",
        "labels",
        "fixVersions",
        "versions",
        "components",
    ];

    let custom: Vec<_> = fields
        .iter()
        .filter(|f| f.required && !SKIP_IDS.contains(&f.id.as_str()))
        .collect();

    if custom.is_empty() {
        return Ok(HashMap::new());
    }

    println!("\nRequired custom fields:");
    println!("{}", "─".repeat(40));

    let mut result = HashMap::new();

    for field in custom {
        let kind = field.kind();
        let value = match kind {
            FieldKind::Text | FieldKind::Url => {
                let v = Text::new(&format!("{}:", field.name))
                    .prompt()
                    .context("Failed to read field")?;
                FieldValue::Text(v)
            }
            FieldKind::Number => {
                let raw = Text::new(&format!("{} (number):", field.name))
                    .prompt()
                    .context("Failed to read field")?;
                let n: f64 = raw
                    .trim()
                    .parse()
                    .context(format!("'{}' must be a number", field.name))?;
                FieldValue::Number(n)
            }
            FieldKind::DateTime => {
                let v = Text::new(&format!("{} (YYYY-MM-DD):", field.name))
                    .prompt()
                    .context("Failed to read field")?;
                FieldValue::Date(v)
            }
            FieldKind::Select => {
                let options = select_options(field.allowed_values.as_deref());
                if options.is_empty() {
                    let v = Text::new(&format!("{}:", field.name))
                        .prompt()
                        .context("Failed to read field")?;
                    FieldValue::SelectName(v)
                } else {
                    let selected = Select::new(&format!("{}:", field.name), options)
                        .prompt()
                        .context("Failed to select")?;
                    FieldValue::SelectName(selected)
                }
            }
            FieldKind::MultiSelect => {
                let options = select_options(field.allowed_values.as_deref());
                if options.is_empty() {
                    let raw = Text::new(&format!("{} (comma-separated):", field.name))
                        .prompt()
                        .context("Failed to read field")?;
                    let vs: Vec<String> = raw.split(',').map(|s| s.trim().to_string()).collect();
                    FieldValue::MultiSelect(vs)
                } else {
                    let selected = MultiSelect::new(&format!("{}:", field.name), options)
                        .prompt()
                        .context("Failed to select")?;
                    FieldValue::MultiSelect(selected)
                }
            }
            FieldKind::User | FieldKind::UserArray => {
                let v = Text::new(&format!("{} (email):", field.name))
                    .prompt()
                    .context("Failed to read field")?;
                FieldValue::UserEmail(v)
            }
            FieldKind::Labels => {
                let raw = Text::new(&format!("{} (space-separated labels):", field.name))
                    .prompt()
                    .context("Failed to read field")?;
                let ls: Vec<String> = raw.split_whitespace().map(|s| s.to_string()).collect();
                FieldValue::Labels(ls)
            }
            // Skip checkbox, cascading, and unknown in required prompts
            _ => continue,
        };

        result.insert(field.id.clone(), value);
    }

    Ok(result)
}

/// Extract display strings from `allowedValues`.
fn select_options(allowed: Option<&[serde_json::Value]>) -> Vec<String> {
    allowed
        .map(|vals: &[serde_json::Value]| {
            vals.iter()
                .filter_map(|v: &serde_json::Value| {
                    v.get("value")
                        .or_else(|| v.get("name"))
                        .and_then(|s: &serde_json::Value| s.as_str())
                        .map(|s: &str| s.to_string())
                })
                .collect::<Vec<String>>()
        })
        .unwrap_or_default()
}

// ─── update ──────────────────────────────────────────────────────────────────

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

    let spinner = spinner_new(format!("Updating {key}..."));
    client
        .update_issue(&key, req)
        .await
        .context("Failed to update issue")?;
    spinner.finish_and_clear();
    println!("✓ Updated: {key}");
    Ok(())
}

// ─── delete ──────────────────────────────────────────────────────────────────

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

    let spinner = spinner_new(format!("Deleting {key}..."));
    client
        .delete_issue(&key)
        .await
        .context("Failed to delete issue")?;
    spinner.finish_and_clear();
    println!("✓ Deleted: {key}");
    Ok(())
}

// ─── transition ──────────────────────────────────────────────────────────────

async fn transition_issue(
    client: JiraClient,
    key: String,
    transition: Option<String>,
) -> Result<()> {
    let spinner = spinner_new(format!("Fetching transitions for {key}..."));
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

        selected
            .trim_end_matches(']')
            .rsplit('[')
            .next()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Failed to parse transition ID"))?
    };

    let spinner = spinner_new(format!("Transitioning {key}..."));
    client
        .transition_issue(&key, &transition_id)
        .await
        .context("Failed to transition issue")?;
    spinner.finish_and_clear();
    println!("✓ Transitioned: {key}");
    Ok(())
}

// ─── attach ──────────────────────────────────────────────────────────────────

async fn attach_files(
    client: JiraClient,
    key: String,
    files: Vec<std::path::PathBuf>,
) -> Result<()> {
    for path in &files {
        if !path.exists() {
            anyhow::bail!("File not found: {}", path.display());
        }
    }

    for path in &files {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let spinner = spinner_new(format!("Uploading {name}..."));
        let attachments = client
            .upload_attachment(&key, path)
            .await
            .with_context(|| format!("Failed to upload {}", path.display()))?;
        spinner.finish_and_clear();

        for a in &attachments {
            println!("✓ Attached: {} ({} bytes)", a.filename, a.size);
        }
    }

    Ok(())
}

// ─── fields ──────────────────────────────────────────────────────────────────

async fn list_fields(
    client: JiraClient,
    project: Option<String>,
    issue_type_filter: Option<String>,
    required_only: bool,
) -> Result<()> {
    let project_key = match project {
        Some(p) => p,
        None => Text::new("Project key:")
            .prompt()
            .context("Failed to read project key")?,
    };

    // Get issue types to resolve the ID
    let spinner = spinner_new(format!("Fetching issue types for {project_key}..."));
    let types = client
        .get_issue_types(&project_key)
        .await
        .context("Failed to fetch issue types")?;
    spinner.finish_and_clear();

    let issue_type: IssueType = if let Some(filter) = issue_type_filter {
        types
            .into_iter()
            .find(|t| t.name.to_lowercase() == filter.to_lowercase())
            .ok_or_else(|| {
                anyhow::anyhow!("Issue type '{}' not found in {}", filter, project_key)
            })?
    } else {
        let options: Vec<String> = types.iter().map(|t| t.name.clone()).collect();
        let selected = Select::new("Issue type:", options)
            .prompt()
            .context("Failed to select issue type")?;
        types
            .into_iter()
            .find(|t| t.name == selected)
            .expect("selected issue type must exist")
    };

    let spinner = spinner_new(format!(
        "Fetching fields for {} / {}...",
        project_key, issue_type.name
    ));
    let mut fields = client
        .get_fields_for_issue_type(&project_key, &issue_type.id)
        .await
        .context("Failed to fetch fields")?;
    spinner.finish_and_clear();

    if required_only {
        fields.retain(|f| f.required);
    }

    // Sort: required first, then by name
    fields.sort_by(|a, b| b.required.cmp(&a.required).then(a.name.cmp(&b.name)));

    println!(
        "\nFields for {} / {} ({} total):\n",
        project_key,
        issue_type.name,
        fields.len()
    );
    println!("{:<30} {:<20} {:<12} REQUIRED", "NAME", "ID", "TYPE");
    println!("{}", "─".repeat(72));

    for f in &fields {
        println!(
            "{:<30} {:<20} {:<12} {}",
            truncate(&f.name, 29),
            truncate(&f.id, 19),
            truncate(&f.field_type, 11),
            if f.required { "✓" } else { "" }
        );
    }

    Ok(())
}

// ─── helpers ─────────────────────────────────────────────────────────────────

fn spinner_new(msg: impl Into<String>) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    pb.set_message(msg.into());
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    pb
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len.saturating_sub(1)])
    }
}

// ─── worklog ─────────────────────────────────────────────────────────────────

async fn worklog(client: JiraClient, key: String, cmd: WorklogCommand) -> Result<()> {
    match cmd {
        WorklogCommand::List => worklog_list(client, key).await,
        WorklogCommand::Add { time, comment } => worklog_add(client, key, time, comment).await,
        WorklogCommand::Delete { id, force } => worklog_delete(client, key, id, force).await,
    }
}

async fn worklog_list(client: JiraClient, key: String) -> Result<()> {
    let spinner = spinner_new(format!("Fetching worklogs for {key}..."));
    let logs = client
        .get_worklogs(&key)
        .await
        .context("Failed to fetch worklogs")?;
    spinner.finish_and_clear();

    if logs.is_empty() {
        println!("No worklogs found for {key}.");
        return Ok(());
    }

    println!("{:<10} {:<20} {:<12} STARTED", "ID", "AUTHOR", "TIME");
    println!("{}", "─".repeat(60));
    for w in &logs {
        println!(
            "{:<10} {:<20} {:<12} {}",
            w.id,
            truncate(w.author.as_deref().unwrap_or("—"), 19),
            w.time_spent,
            &w.started[..10.min(w.started.len())]
        );
        if let Some(c) = &w.comment {
            println!("           {}", c);
        }
    }
    Ok(())
}

async fn worklog_add(
    client: JiraClient,
    key: String,
    time: String,
    comment: Option<String>,
) -> Result<()> {
    let spinner = spinner_new(format!("Logging {time} on {key}..."));
    let log = client
        .add_worklog(&key, &time, comment.as_deref(), None)
        .await
        .context("Failed to add worklog")?;
    spinner.finish_and_clear();
    println!(
        "✓ Logged {} on {} (worklog id: {})",
        log.time_spent, key, log.id
    );
    Ok(())
}

async fn worklog_delete(client: JiraClient, key: String, id: String, force: bool) -> Result<()> {
    if !force {
        let confirm = inquire::Confirm::new(&format!("Delete worklog {id} on {key}?"))
            .with_default(false)
            .prompt()
            .context("Failed to read confirmation")?;
        if !confirm {
            println!("Aborted.");
            return Ok(());
        }
    }

    let spinner = spinner_new(format!("Deleting worklog {id}..."));
    client
        .delete_worklog(&key, &id)
        .await
        .context("Failed to delete worklog")?;
    spinner.finish_and_clear();
    println!("✓ Deleted worklog {id} from {key}");
    Ok(())
}

// ─── bulk transition ──────────────────────────────────────────────────────────

async fn bulk_transition(client: JiraClient, jql: String, to: String, force: bool) -> Result<()> {
    let spinner = spinner_new("Fetching issues...");
    let issues = client
        .get_all_issues(&jql)
        .await
        .context("Failed to fetch issues")?;
    spinner.finish_and_clear();

    if issues.is_empty() {
        println!("No issues found matching JQL.");
        return Ok(());
    }

    println!("Found {} issues.", issues.len());

    if !force {
        let confirm = inquire::Confirm::new(&format!(
            "Transition all {} issues to '{to}'?",
            issues.len()
        ))
        .with_default(false)
        .prompt()
        .context("Failed to read confirmation")?;
        if !confirm {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Fetch available transitions from the first issue
    let transitions = client
        .get_transitions(&issues[0].key)
        .await
        .context("Failed to fetch transitions")?;

    let transition_id = transitions
        .iter()
        .find(|t| {
            t.get("id").and_then(|v| v.as_str()) == Some(&to)
                || t.get("name")
                    .and_then(|v| v.as_str())
                    .map(|n| n.to_lowercase())
                    == Some(to.to_lowercase())
        })
        .and_then(|t| t.get("id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Transition '{}' not found", to))?;

    let pb = ProgressBar::new(issues.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.cyan} [{bar:40}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=> "),
    );

    let mut ok = 0u64;
    let mut failed: Vec<String> = Vec::new();

    for issue in &issues {
        pb.set_message(issue.key.clone());
        match client.transition_issue(&issue.key, &transition_id).await {
            Ok(_) => ok += 1,
            Err(e) => failed.push(format!("{}: {}", issue.key, e)),
        }
        pb.inc(1);
    }

    pb.finish_and_clear();
    println!("✓ Transitioned {ok}/{} issues to '{to}'", issues.len());
    if !failed.is_empty() {
        println!("✗ Failed ({}):", failed.len());
        for f in &failed {
            println!("  {f}");
        }
    }

    Ok(())
}

// ─── bulk update ─────────────────────────────────────────────────────────────

async fn bulk_update(
    client: JiraClient,
    jql: String,
    assignee: Option<String>,
    priority: Option<String>,
    force: bool,
) -> Result<()> {
    if assignee.is_none() && priority.is_none() {
        anyhow::bail!("Nothing to update. Use --assignee or --priority.");
    }

    let spinner = spinner_new("Fetching issues...");
    let issues = client
        .get_all_issues(&jql)
        .await
        .context("Failed to fetch issues")?;
    spinner.finish_and_clear();

    if issues.is_empty() {
        println!("No issues found.");
        return Ok(());
    }

    println!("Found {} issues.", issues.len());

    if !force {
        let confirm = inquire::Confirm::new(&format!("Update {} issues?", issues.len()))
            .with_default(false)
            .prompt()
            .context("Failed to read confirmation")?;
        if !confirm {
            println!("Aborted.");
            return Ok(());
        }
    }

    let req = UpdateIssueRequest {
        assignee: assignee.clone(),
        priority: priority.clone(),
        ..Default::default()
    };

    let pb = ProgressBar::new(issues.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.cyan} [{bar:40}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=> "),
    );

    let mut ok = 0u64;
    let mut failed: Vec<String> = Vec::new();

    for issue in &issues {
        pb.set_message(issue.key.clone());
        match client.update_issue(&issue.key, req.clone()).await {
            Ok(_) => ok += 1,
            Err(e) => failed.push(format!("{}: {}", issue.key, e)),
        }
        pb.inc(1);
    }

    pb.finish_and_clear();
    println!("✓ Updated {ok}/{} issues", issues.len());
    if !failed.is_empty() {
        println!("✗ Failed ({}):", failed.len());
        for f in &failed {
            println!("  {f}");
        }
    }

    Ok(())
}

// ─── archive ─────────────────────────────────────────────────────────────────

async fn archive(client: JiraClient, jql: String, force: bool) -> Result<()> {
    let spinner = spinner_new("Fetching issues...");
    let issues = client
        .get_all_issues(&jql)
        .await
        .context("Failed to fetch issues")?;
    spinner.finish_and_clear();

    if issues.is_empty() {
        println!("No issues found.");
        return Ok(());
    }

    println!("Found {} issues.", issues.len());

    if !force {
        let confirm = inquire::Confirm::new(&format!(
            "Archive {} issues? This cannot be undone.",
            issues.len()
        ))
        .with_default(false)
        .prompt()
        .context("Failed to read confirmation")?;
        if !confirm {
            println!("Aborted.");
            return Ok(());
        }
    }

    let keys: Vec<String> = issues.iter().map(|i| i.key.clone()).collect();

    let spinner = spinner_new(format!("Archiving {} issues...", keys.len()));
    client
        .archive_issues(&keys)
        .await
        .context("Failed to archive issues")?;
    spinner.finish_and_clear();
    println!("✓ Archived {} issues", keys.len());

    Ok(())
}

// ─── jql builder ─────────────────────────────────────────────────────────────

async fn jql_builder(client: JiraClient, run: bool) -> Result<()> {
    println!("JQL Builder — press Enter to skip any field\n");

    let project = Text::new("Project key (e.g. PROJ):")
        .prompt_skippable()
        .context("Failed to read project")?
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string());

    let status_opts = vec![
        "To Do",
        "In Progress",
        "In Review",
        "Done",
        "Blocked",
        "(any)",
    ];
    let status_sel = Select::new("Status:", status_opts)
        .prompt()
        .context("Failed to read status")?;
    let status = if status_sel == "(any)" {
        None
    } else {
        Some(status_sel.to_string())
    };

    let assignee_opts = vec!["Me (currentUser)", "Unassigned", "Custom email", "(any)"];
    let assignee_sel = Select::new("Assignee:", assignee_opts)
        .prompt()
        .context("Failed to read assignee")?;
    let assignee = match assignee_sel {
        "Me (currentUser)" => Some("currentUser()".to_string()),
        "Unassigned" => Some("EMPTY".to_string()),
        "Custom email" => {
            let email = Text::new("Email:")
                .prompt()
                .context("Failed to read email")?;
            Some(format!("\"{email}\""))
        }
        _ => None,
    };

    let priority_opts = vec!["Highest", "High", "Medium", "Low", "Lowest", "(any)"];
    let priority_sel = Select::new("Priority:", priority_opts)
        .prompt()
        .context("Failed to read priority")?;
    let priority = if priority_sel == "(any)" {
        None
    } else {
        Some(priority_sel.to_string())
    };

    let order_opts = vec!["updated DESC", "created DESC", "priority DESC", "key ASC"];
    let order = Select::new("Order by:", order_opts)
        .prompt()
        .context("Failed to read order")?;

    // Build JQL
    let mut parts: Vec<String> = Vec::new();
    if let Some(p) = project {
        parts.push(format!("project = {p}"));
    }
    if let Some(s) = status {
        parts.push(format!("status = \"{s}\""));
    }
    if let Some(a) = assignee {
        parts.push(format!("assignee = {a}"));
    }
    if let Some(p) = priority {
        parts.push(format!("priority = \"{p}\""));
    }

    if parts.is_empty() {
        parts.push("assignee = currentUser()".to_string());
    }

    let jql = format!("{} ORDER BY {}", parts.join(" AND "), order);

    println!("\nGenerated JQL:\n  {jql}\n");

    if run {
        let spinner = spinner_new("Searching...");
        let result = client
            .search_issues(&jql, None, Some(25))
            .await
            .context("Search failed")?;
        spinner.finish_and_clear();

        if result.issues.is_empty() {
            println!("No issues found.");
            return Ok(());
        }

        println!("{:<12} {:<8} {:<20} SUMMARY", "KEY", "TYPE", "STATUS");
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
            println!("\nShowing {} of {total}", result.issues.len());
        }
    }

    Ok(())
}

// Keep old CreateIssueRequest available for any other callers
#[allow(dead_code)]
fn _use_old_request() {
    let _ = CreateIssueRequest::default();
    let _: Option<Value> = None;
}
