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
    /// List issues — by project, JQL, or your assigned issues
    ///
    /// Without flags, shows issues assigned to you (assignee = currentUser()).
    /// Use --project for a project overview, or --jql for full control.
    ///
    /// Examples:
    ///   jira issue list                              # your assigned issues
    ///   jira issue list -p PROJ                      # all issues in project
    ///   jira issue list -p PROJ -l 50                # up to 50 results
    ///   jira issue list --jql 'status = "In Progress" AND project = PROJ'
    ///   jira issue list --jql 'sprint = openSprints() AND assignee = me'
    List {
        /// Project key (e.g. PROJ). Overrides default project from config.
        #[arg(short, long, value_name = "PROJECT")]
        project: Option<String>,
        /// Raw JQL query — overrides --project when both are provided
        #[arg(long, value_name = "JQL")]
        jql: Option<String>,
        /// Maximum number of issues to return (default: 25, max: 100)
        #[arg(short, long, default_value = "25", value_name = "N")]
        limit: u32,
        /// Output results as JSON array
        #[arg(long)]
        json: bool,
    },

    /// View full issue details — description, attachments, and metadata
    ///
    /// Displays: type, status, project, priority, assignee, reporter,
    /// created/updated timestamps, attachment list, and rendered description.
    ///
    /// Examples:
    ///   jira issue view PROJ-123
    ///   jira issue view PROJ-123 --json
    View {
        /// Issue key (e.g. PROJ-123)
        key: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Create a new issue — interactive or fully non-interactive
    ///
    /// Without flags, prompts for: project key, issue type, summary, and
    /// any required custom fields (fetched dynamically from the Jira schema).
    ///
    /// Provide flags to skip individual prompts. All flags are optional —
    /// missing ones will be prompted interactively.
    ///
    /// Use --no-custom-fields to skip required custom field prompts entirely.
    /// --field takes any field ID (including customfield_XXXXX) as key=value.
    ///
    /// To discover available fields and their IDs for a project, run:
    ///   jira issue fields -p PROJ --issue-type Bug
    ///
    /// Examples:
    ///   jira issue create                                         # fully interactive
    ///   jira issue create -p PROJ -s "Fix login bug" -t Bug
    ///   jira issue create -p PROJ -s "API story" -t Story --assignee me --labels "backend,api"
    ///   jira issue create -p PROJ -s "Sub-task" -t Sub-task --parent PROJ-100
    ///   jira issue create -p PROJ -s "Feat" --description-file description.md
    ///   jira issue create -p PROJ -s "Fix" --field story_points=5 --field customfield_10020=sprint1
    Create {
        /// Project key (e.g. PROJ)
        #[arg(short, long, value_name = "PROJECT")]
        project: Option<String>,
        /// Issue summary / title
        #[arg(short, long, value_name = "TEXT")]
        summary: Option<String>,
        /// Issue type name (e.g. Bug, Story, Task, Epic) — interactive picker if omitted
        #[arg(short = 't', long, value_name = "TYPE")]
        issue_type: Option<String>,
        /// Assignee email address, or "me" for the current user
        #[arg(short, long, value_name = "EMAIL|me")]
        assignee: Option<String>,
        /// Priority level: Highest, High, Medium, Low, Lowest
        #[arg(long, value_name = "PRIORITY")]
        priority: Option<String>,
        /// Read description from a file
        #[arg(long, value_name = "FILE")]
        description_file: Option<std::path::PathBuf>,
        /// Format of --description-file: markdown (default), adf, text
        #[arg(long, value_name = "FORMAT", default_value = "markdown")]
        description_format: String,
        /// Labels to set (comma-separated, e.g. "bug,backend")
        #[arg(long, value_name = "LABELS")]
        labels: Option<String>,
        /// Component names to set (comma-separated, e.g. "auth,api")
        #[arg(long, value_name = "COMPONENTS")]
        components: Option<String>,
        /// Parent issue key for sub-tasks (e.g. PROJ-100)
        #[arg(long, value_name = "KEY")]
        parent: Option<String>,
        /// Fix version name(s) to set (comma-separated, e.g. "v1.0,v1.1")
        #[arg(long, value_name = "VERSIONS")]
        fix_version: Option<String>,
        /// Attach file(s) after creating the issue
        #[arg(long, value_name = "FILE")]
        attachments: Vec<std::path::PathBuf>,
        /// Set any field by ID — repeatable. Value is parsed as JSON if valid,
        /// otherwise treated as a plain string.
        ///
        /// Standard fields:  --field story_points=5
        /// Custom fields:    --field customfield_10016=5
        /// Select fields:    --field customfield_10020='{"value":"Option A"}'
        /// Multi-select:     --field customfield_10021='[{"value":"A"},{"value":"B"}]'
        ///
        /// Run `jira issue fields -p PROJ --issue-type Bug` to list all field IDs.
        #[arg(long, value_name = "FIELD_ID=VALUE")]
        field: Vec<String>,
        /// Skip required custom field prompts (fields will be omitted)
        #[arg(long)]
        no_custom_fields: bool,
        /// Output the created issue as JSON
        #[arg(long)]
        json: bool,
    },

    /// Update fields on an existing issue
    ///
    /// At least one field flag must be provided. Only supplied flags are changed.
    /// Assignee can be an email address or "me" (resolves to current user's accountId).
    ///
    /// Examples:
    ///   jira issue update PROJ-123 --summary "Updated title"
    ///   jira issue update PROJ-123 --assignee me --priority High
    ///   jira issue update PROJ-123 --description-file updated.md
    ///   jira issue update PROJ-123 --labels "bug,backend" --components "auth"
    ///   jira issue update PROJ-123 --field story_points=8
    Update {
        /// Issue key (e.g. PROJ-123)
        key: String,
        /// New summary / title
        #[arg(short, long, value_name = "TEXT")]
        summary: Option<String>,
        /// New assignee — email address or "me" for the current user
        #[arg(short, long, value_name = "EMAIL|me")]
        assignee: Option<String>,
        /// New priority: Highest, High, Medium, Low, Lowest
        #[arg(long, value_name = "PRIORITY")]
        priority: Option<String>,
        /// Read new description from a file
        #[arg(long, value_name = "FILE")]
        description_file: Option<std::path::PathBuf>,
        /// Format of --description-file: markdown (default), adf, text
        #[arg(long, value_name = "FORMAT", default_value = "markdown")]
        description_format: String,
        /// Replace labels (comma-separated, e.g. "bug,backend")
        #[arg(long, value_name = "LABELS")]
        labels: Option<String>,
        /// Replace components (comma-separated, e.g. "auth,api")
        #[arg(long, value_name = "COMPONENTS")]
        components: Option<String>,
        /// Replace fix versions (comma-separated, e.g. "v1.0,v1.1")
        #[arg(long, value_name = "VERSIONS")]
        fix_version: Option<String>,
        /// Set parent issue key (e.g. PROJ-100)
        #[arg(long, value_name = "KEY")]
        parent: Option<String>,
        /// Set any field by ID — repeatable. Value is parsed as JSON if valid,
        /// otherwise treated as a plain string.
        ///
        /// Standard fields:  --field story_points=5
        /// Custom fields:    --field customfield_10016=5
        /// Select fields:    --field customfield_10020='{"value":"Option A"}'
        ///
        /// Run `jira issue fields -p PROJ --issue-type Bug` to list all field IDs.
        #[arg(long, value_name = "FIELD_ID=VALUE")]
        field: Vec<String>,
        /// Re-fetch and output the updated issue as JSON
        #[arg(long)]
        json: bool,
    },

    /// Delete an issue permanently — this cannot be undone
    ///
    /// Prompts for confirmation unless --force is used.
    /// Subtasks are also deleted along with the parent issue.
    ///
    /// Examples:
    ///   jira issue delete PROJ-123
    ///   jira issue delete PROJ-123 --force      # skip confirmation prompt
    Delete {
        /// Issue key (e.g. PROJ-123)
        key: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },

    /// Transition an issue to a new workflow status
    ///
    /// Without a transition argument, shows an interactive picker of all
    /// available transitions for the issue.
    ///
    /// The transition argument accepts a name (case-insensitive) or numeric ID.
    /// To see available transitions and IDs for an issue:
    ///   jira api get /rest/api/3/issue/PROJ-123/transitions
    ///
    /// Examples:
    ///   jira issue transition PROJ-123                 # interactive picker
    ///   jira issue transition PROJ-123 "In Progress"
    ///   jira issue transition PROJ-123 Done
    ///   jira issue transition PROJ-123 31              # by transition ID
    Transition {
        /// Issue key (e.g. PROJ-123)
        key: String,
        /// Transition name (e.g. "In Progress", "Done") or numeric ID — interactive if omitted
        transition: Option<String>,
        /// Re-fetch and output the transitioned issue as JSON
        #[arg(long)]
        json: bool,
    },

    /// Attach one or more files to an issue
    ///
    /// Uploads via multipart/form-data. MIME type is detected automatically
    /// from the file extension. Multiple files can be attached in one command.
    ///
    /// Examples:
    ///   jira issue attach PROJ-123 screenshot.png
    ///   jira issue attach PROJ-123 report.pdf logs.txt dump.zip
    ///   jira issue attach PROJ-123 ~/Downloads/output.json
    Attach {
        /// Issue key (e.g. PROJ-123)
        key: String,
        /// One or more file paths to upload as attachments
        #[arg(required = true, value_name = "FILE")]
        files: Vec<std::path::PathBuf>,
    },

    /// List available fields for a project and issue type
    ///
    /// Shows field name, ID, type (text, select, number, user, etc.),
    /// and whether the field is required (marked ✓).
    ///
    /// Use this to discover field IDs before using --field key=value in
    /// create/update commands. Custom fields have IDs like customfield_10016.
    ///
    /// Examples:
    ///   jira issue fields -p PROJ               # interactive issue type picker
    ///   jira issue fields -p PROJ --issue-type Bug
    ///   jira issue fields -p PROJ --issue-type Story --required-only
    Fields {
        /// Project key (e.g. PROJ) — interactive prompt if omitted
        #[arg(short, long, value_name = "PROJECT")]
        project: Option<String>,
        /// Filter by issue type name (e.g. Bug, Story, Task) — interactive picker if omitted
        #[arg(long, value_name = "TYPE")]
        issue_type: Option<String>,
        /// Show only required fields
        #[arg(long)]
        required_only: bool,
        /// Output fields as JSON array
        #[arg(long)]
        json: bool,
    },

    /// Manage time tracking (worklogs) on an issue
    ///
    /// Log time, list existing entries, or delete a worklog.
    ///
    /// Time format: Jira duration syntax — "2h", "30m", "1d", "1h 30m"
    /// Note: 1d = 8 working hours (default Jira configuration).
    ///
    /// Examples:
    ///   jira issue worklog list PROJ-123
    ///   jira issue worklog add PROJ-123 --time "2h 30m"
    ///   jira issue worklog add PROJ-123 --time 1d --comment "Implemented auth"
    ///   jira issue worklog delete PROJ-123 <worklog-id>
    Worklog {
        /// Issue key (e.g. PROJ-123)
        key: String,
        #[command(subcommand)]
        command: WorklogCommand,
    },

    /// Transition all issues matching a JQL query to a new status
    ///
    /// Fetches all matching issues (no pagination limit), confirms unless --force,
    /// then transitions each one. Progress bar shows per-issue status.
    /// Failed issues are listed at the end — success count is always reported.
    ///
    /// Transition can be name (case-insensitive) or numeric ID.
    /// The transition is validated against the first matching issue.
    ///
    /// Examples:
    ///   jira issue bulk-transition --jql 'project = PROJ AND status = "To Do"' --to "In Progress"
    ///   jira issue bulk-transition --jql 'assignee = me AND sprint = openSprints()' --to Done --force
    BulkTransition {
        /// JQL query to select issues (use quotes for values with spaces)
        #[arg(long, value_name = "JQL")]
        jql: String,
        /// Transition name (e.g. "In Progress", "Done") or numeric ID
        #[arg(long, value_name = "TRANSITION")]
        to: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
        /// Output result summary as JSON
        #[arg(long)]
        json: bool,
    },

    /// Update fields on all issues matching a JQL query
    ///
    /// Supports bulk reassign and bulk priority change.
    /// At least one of --assignee or --priority must be provided.
    /// Prompts for confirmation unless --force is used.
    ///
    /// Examples:
    ///   jira issue bulk-update --jql 'project = PROJ AND assignee = EMPTY' --assignee me
    ///   jira issue bulk-update --jql 'project = PROJ AND priority = Low' --priority High --force
    BulkUpdate {
        /// JQL query to select issues
        #[arg(long, value_name = "JQL")]
        jql: String,
        /// New assignee — email address or "me" for the current user
        #[arg(long, value_name = "EMAIL|me")]
        assignee: Option<String>,
        /// New priority: Highest, High, Medium, Low, Lowest
        #[arg(long, value_name = "PRIORITY")]
        priority: Option<String>,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
        /// Output result summary as JSON
        #[arg(long)]
        json: bool,
    },

    /// Archive all issues matching a JQL query
    ///
    /// Archived issues are hidden from default searches but not permanently deleted.
    /// Uses Jira's async archive task API. Requires project admin permissions.
    ///
    /// Note: this action cannot be reversed from this CLI.
    ///
    /// Examples:
    ///   jira issue archive --jql 'project = PROJ AND status = Done AND updated < -1y'
    ///   jira issue archive --jql 'project = PROJ AND status = Done' --force
    Archive {
        /// JQL query to select issues to archive
        #[arg(long, value_name = "JQL")]
        jql: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },

    /// Clone an issue — create a copy, optionally in a different project
    ///
    /// Copies: summary, description, type, priority, labels, components,
    /// and fix versions. Assignee is NOT copied by default.
    ///
    /// Use --move to delete the original after cloning (move semantics).
    /// Use --assignee to set a new assignee on the clone.
    /// Use --json to output the created clone as JSON.
    ///
    /// Examples:
    ///   jira issue clone PROJ-123                      # clone in same project
    ///   jira issue clone PROJ-123 --project NEWPROJ    # clone to another project
    ///   jira issue clone PROJ-123 --summary "Copy: original title"
    ///   jira issue clone PROJ-123 --move               # clone then delete original
    ///   jira issue clone PROJ-123 --project OTHER --json
    Clone {
        /// Source issue key (e.g. PROJ-123)
        key: String,
        /// Target project key — defaults to same project as source
        #[arg(long, value_name = "PROJECT")]
        project: Option<String>,
        /// Override the summary on the clone (defaults to source summary)
        #[arg(long, value_name = "TEXT")]
        summary: Option<String>,
        /// Set assignee on the clone (email or "me") — source assignee not copied
        #[arg(long, value_name = "EMAIL|me")]
        assignee: Option<String>,
        /// Delete the original issue after cloning
        #[arg(long)]
        r#move: bool,
        /// Output the cloned issue as JSON
        #[arg(long)]
        json: bool,
    },

    /// Interactive JQL query builder — guided filters with generated query
    ///
    /// Walks through common JQL filters (project, status, assignee, priority,
    /// sort order) and generates a valid JQL string.
    ///
    /// The generated JQL is printed so you can copy it to other commands.
    /// Use --run to immediately execute the query and display results.
    ///
    /// Examples:
    ///   jira issue jql              # build query, print it
    ///   jira issue jql --run        # build and run immediately
    ///
    /// ── JQL Quick Reference ────────────────────────────────────────────────
    ///
    /// Operators:
    ///   =   !=   >   <   >=   <=   in (...)   not in (...)   is EMPTY   ~
    ///
    /// Common fields:
    ///   project = PROJ
    ///   assignee = currentUser()
    ///   assignee = "email@example.com"
    ///   status = "In Progress"
    ///   status in ("To Do", "In Progress")
    ///   priority = High
    ///   issuetype = Bug
    ///   sprint = openSprints()
    ///   sprint = closedSprints()
    ///   labels = backend
    ///   component = "auth-service"
    ///   fixVersion = "v2.0"
    ///   reporter = currentUser()
    ///   parent = PROJ-100
    ///
    /// Date filters:
    ///   created >= -7d               created in last 7 days
    ///   updated >= -30d              updated in last 30 days
    ///   created >= "2024-01-01"      on or after a date
    ///   updated < -90d               not updated in 90+ days
    ///
    /// Text search:
    ///   text ~ "login bug"           full-text search (summary + description)
    ///   summary ~ "payment"          summary only
    ///
    /// Combining:
    ///   project = PROJ AND status = "In Progress"
    ///   assignee = currentUser() OR assignee = "teammate@org.com"
    ///   project = PROJ AND NOT status = Done
    ///
    /// Sorting:
    ///   ORDER BY updated DESC
    ///   ORDER BY priority DESC, created ASC
    ///
    /// Full examples:
    ///   project = PROJ AND assignee = currentUser() AND sprint = openSprints() ORDER BY priority DESC
    ///   status in ("To Do", "In Progress") AND updated >= -7d ORDER BY updated DESC
    ///   project = PROJ AND issuetype = Bug AND priority in (High, Critical) ORDER BY created DESC
    Jql {
        /// Execute the generated JQL immediately (shows up to 25 results)
        #[arg(long)]
        run: bool,
    },

    /// Create multiple issues from a JSON manifest file
    ///
    /// The manifest is a JSON array of issue objects. Each object supports
    /// the same fields as `jira issue create` flags.
    ///
    /// Manifest format (JSON array):
    /// [
    ///   {
    ///     "project": "PROJ",           (required)
    ///     "summary": "Issue title",    (required)
    ///     "type": "Task",              (default: "Task")
    ///     "assignee": "user@org.com",  (email or "me")
    ///     "priority": "High",
    ///     "labels": ["bug", "backend"],
    ///     "components": ["auth"],
    ///     "parent": "PROJ-100",
    ///     "fix_versions": ["v1.0"],
    ///     "description": "Markdown description",
    ///     "fields": { "customfield_10016": 5 }
    ///   }
    /// ]
    ///
    /// Output: prints each created issue key and summary.
    ///
    /// Examples:
    ///   jira issue bulk-create --manifest issues.json
    /// Run mixed operations from a JSON manifest file
    ///
    /// Each entry in the manifest is an object with an "op" field specifying
    /// the operation, plus the fields relevant to that operation.
    ///
    /// Supported ops:
    ///   "create"     — create a new issue (same fields as bulk-create manifest)
    ///   "update"     — update an existing issue by key
    ///   "transition" — transition an issue to a new status
    ///   "archive"    — archive an issue by key
    ///
    /// Manifest format:
    /// [
    ///   { "op": "create",     "project": "PROJ", "summary": "New task", "type": "Task" },
    ///   { "op": "update",     "key": "PROJ-10", "priority": "High", "assignee": "me" },
    ///   { "op": "transition", "key": "PROJ-11", "to": "Done" },
    ///   { "op": "archive",    "key": "PROJ-12" }
    /// ]
    ///
    /// Output: per-op result summary. Use --json for machine-readable output.
    ///
    /// Examples:
    ///   jira issue batch --manifest ops.json
    ///   jira issue batch --manifest ops.json --json
    Batch {
        /// Path to the JSON manifest file (array of op objects)
        #[arg(long, value_name = "FILE")]
        manifest: std::path::PathBuf,
        /// Output results as JSON array
        #[arg(long)]
        json: bool,
    },

    #[command(name = "bulk-create")]
    BulkCreate {
        /// Path to the JSON manifest file (array of issue objects)
        #[arg(long, value_name = "FILE")]
        manifest: std::path::PathBuf,
        /// Output created issues as JSON array
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum WorklogCommand {
    /// List all worklog entries for the issue
    ///
    /// Shows worklog ID, author, time spent, start date, and comment.
    /// The worklog ID is needed to delete a specific entry.
    List,

    /// Log time on an issue
    ///
    /// Time format: Jira duration syntax.
    /// Examples: "2h", "30m", "1d", "1h 30m", "3d 4h 30m"
    /// Note: 1d = 8 working hours in default Jira configuration.
    ///
    /// Examples:
    ///   jira issue worklog add PROJ-123 --time "2h 30m"
    ///   jira issue worklog add PROJ-123 --time 1d --comment "Implemented login"
    Add {
        /// Time spent in Jira duration format (e.g. "2h", "30m", "1d", "1h 30m")
        #[arg(short, long, value_name = "DURATION")]
        time: String,
        /// Optional comment describing the work done
        #[arg(short, long, value_name = "TEXT")]
        comment: Option<String>,
    },

    /// Delete a worklog entry
    ///
    /// Use `jira issue worklog list KEY` to find the worklog ID.
    /// Prompts for confirmation unless --force is used.
    ///
    /// Examples:
    ///   jira issue worklog delete PROJ-123 12345
    ///   jira issue worklog delete PROJ-123 12345 --force
    Delete {
        /// Worklog ID (see: jira issue worklog list PROJ-123)
        id: String,
        /// Skip confirmation prompt
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
            json,
        } => list_issues(client, project.or(default_project), jql, limit, json).await,
        IssueCommand::View { key, json } => view_issue(client, key, json).await,
        IssueCommand::Create {
            project,
            summary,
            issue_type,
            assignee,
            priority,
            description_file,
            description_format,
            labels,
            components,
            parent,
            fix_version,
            attachments,
            field,
            no_custom_fields,
            json,
        } => {
            create_issue(
                client,
                project.or(default_project),
                summary,
                issue_type,
                assignee,
                priority,
                description_file,
                description_format,
                labels,
                components,
                parent,
                fix_version,
                attachments,
                field,
                no_custom_fields,
                json,
            )
            .await
        }
        IssueCommand::Update {
            key,
            summary,
            assignee,
            priority,
            description_file,
            description_format,
            labels,
            components,
            fix_version,
            parent,
            field,
            json,
        } => {
            update_issue(
                client,
                key,
                summary,
                assignee,
                priority,
                description_file,
                description_format,
                labels,
                components,
                fix_version,
                parent,
                field,
                json,
            )
            .await
        }
        IssueCommand::Delete { key, force } => delete_issue(client, key, force).await,
        IssueCommand::Transition {
            key,
            transition,
            json,
        } => transition_issue(client, key, transition, json).await,
        IssueCommand::Attach { key, files } => attach_files(client, key, files).await,
        IssueCommand::Fields {
            project,
            issue_type,
            required_only,
            json,
        } => {
            list_fields(
                client,
                project.or(default_project),
                issue_type,
                required_only,
                json,
            )
            .await
        }
        IssueCommand::Worklog { key, command } => worklog(client, key, command).await,
        IssueCommand::BulkTransition {
            jql,
            to,
            force,
            json,
        } => bulk_transition(client, jql, to, force, json).await,
        IssueCommand::BulkUpdate {
            jql,
            assignee,
            priority,
            force,
            json,
        } => bulk_update(client, jql, assignee, priority, force, json).await,
        IssueCommand::Archive { jql, force } => archive(client, jql, force).await,
        IssueCommand::Jql { run } => jql_builder(client, run).await,
        IssueCommand::BulkCreate { manifest, json } => bulk_create(client, manifest, json).await,
        IssueCommand::Clone {
            key,
            project,
            summary,
            assignee,
            r#move,
            json,
        } => clone_issue(client, key, project, summary, assignee, r#move, json).await,
        IssueCommand::Batch { manifest, json } => batch_manifest(client, manifest, json).await,
    }
}

// ─── list ────────────────────────────────────────────────────────────────────

async fn list_issues(
    client: JiraClient,
    project: Option<String>,
    jql: Option<String>,
    limit: u32,
    json: bool,
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

    if json {
        println!("{}", serde_json::to_string_pretty(&result.issues)?);
        return Ok(());
    }

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

async fn view_issue(client: JiraClient, key: String, json: bool) -> Result<()> {
    let spinner = spinner_new(format!("Fetching {key}..."));
    let issue = client
        .get_issue(&key)
        .await
        .context("Failed to fetch issue")?;
    spinner.finish_and_clear();

    if json {
        println!("{}", serde_json::to_string_pretty(&issue)?);
        return Ok(());
    }

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

#[allow(clippy::too_many_arguments)]
async fn create_issue(
    client: JiraClient,
    project: Option<String>,
    summary: Option<String>,
    issue_type: Option<String>,
    assignee: Option<String>,
    priority: Option<String>,
    description_file: Option<std::path::PathBuf>,
    description_format: String,
    labels: Option<String>,
    components: Option<String>,
    parent: Option<String>,
    fix_version: Option<String>,
    attachments: Vec<std::path::PathBuf>,
    field: Vec<String>,
    no_custom_fields: bool,
    json: bool,
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

    // 4. Description from file
    let (description, description_adf) =
        read_description_file(description_file.as_deref(), &description_format)?;

    // 5. Custom fields — combine --field flags + interactive prompts
    let mut custom_fields = parse_field_flags(&field)?;
    if !no_custom_fields {
        let interactive = collect_custom_fields(&client, &project_key, &issue_type_id).await?;
        for (k, v) in interactive {
            custom_fields.entry(k).or_insert(v);
        }
    }

    let req = CreateIssueRequestV2 {
        project_key: project_key.clone(),
        summary,
        description,
        description_adf,
        issue_type: issue_type_name,
        assignee,
        priority,
        labels: parse_csv(labels.as_deref()),
        components: parse_csv(components.as_deref()),
        parent,
        fix_versions: parse_csv(fix_version.as_deref()),
        custom_fields,
    };

    let spinner = spinner_new("Creating issue...");
    let issue = client
        .create_issue_v2(req)
        .await
        .context("Failed to create issue")?;
    spinner.finish_and_clear();

    // Attach files if provided
    let had_attachments = !attachments.is_empty();
    if had_attachments {
        attach_files(client.clone(), issue.key.clone(), attachments).await?;
    }

    if json {
        // Re-fetch to include any attachment metadata
        let full = if had_attachments {
            client.get_issue(&issue.key).await.unwrap_or(issue)
        } else {
            issue
        };
        println!("{}", serde_json::to_string_pretty(&full)?);
    } else {
        println!("✓ Created: {} — {}", issue.key, issue.summary);
    }

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

#[allow(clippy::too_many_arguments)]
async fn update_issue(
    client: JiraClient,
    key: String,
    summary: Option<String>,
    assignee: Option<String>,
    priority: Option<String>,
    description_file: Option<std::path::PathBuf>,
    description_format: String,
    labels: Option<String>,
    components: Option<String>,
    fix_version: Option<String>,
    parent: Option<String>,
    field: Vec<String>,
    json: bool,
) -> Result<()> {
    let (description, description_adf) =
        read_description_file(description_file.as_deref(), &description_format)?;

    let custom_fields = parse_field_flags(&field)?;
    let labels_vec = labels.as_deref().map(|s| parse_csv(Some(s)));
    let components_vec = components.as_deref().map(|s| parse_csv(Some(s)));
    let fix_versions_vec = fix_version.as_deref().map(|s| parse_csv(Some(s)));

    let has_changes = summary.is_some()
        || assignee.is_some()
        || priority.is_some()
        || description.is_some()
        || description_adf.is_some()
        || labels_vec.is_some()
        || components_vec.is_some()
        || fix_versions_vec.is_some()
        || parent.is_some()
        || !custom_fields.is_empty();

    if !has_changes {
        println!(
            "No fields to update. Use --summary, --assignee, --priority, --description-file, --labels, --components, --fix-version, --parent, or --field."
        );
        return Ok(());
    }

    let req = UpdateIssueRequest {
        summary,
        description,
        description_adf,
        assignee,
        priority,
        labels: labels_vec,
        components: components_vec,
        fix_versions: fix_versions_vec,
        parent,
        custom_fields,
        ..Default::default()
    };

    let spinner = spinner_new(format!("Updating {key}..."));
    client
        .update_issue(&key, req)
        .await
        .context("Failed to update issue")?;
    spinner.finish_and_clear();

    if json {
        let issue = client
            .get_issue(&key)
            .await
            .context("Failed to fetch updated issue")?;
        println!("{}", serde_json::to_string_pretty(&issue)?);
    } else {
        println!("✓ Updated: {key}");
    }
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
    json: bool,
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

    if json {
        let issue = client
            .get_issue(&key)
            .await
            .context("Failed to fetch transitioned issue")?;
        println!("{}", serde_json::to_string_pretty(&issue)?);
    } else {
        println!("✓ Transitioned: {key}");
    }
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
    json: bool,
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

    if json {
        println!("{}", serde_json::to_string_pretty(&fields)?);
        return Ok(());
    }

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
    use std::io::IsTerminal;
    if !std::io::stdout().is_terminal() {
        return ProgressBar::hidden();
    }
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

fn progress_bar(len: u64) -> ProgressBar {
    use std::io::IsTerminal;
    if !std::io::stdout().is_terminal() {
        return ProgressBar::hidden();
    }
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.cyan} [{bar:40}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=> "),
    );
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

async fn bulk_transition(
    client: JiraClient,
    jql: String,
    to: String,
    force: bool,
    json: bool,
) -> Result<()> {
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

    let pb = progress_bar(issues.len() as u64);

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

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "total": issues.len(),
                "succeeded": ok,
                "failed_count": failed.len(),
                "failed": failed,
            }))?
        );
    } else {
        println!("✓ Transitioned {ok}/{} issues to '{to}'", issues.len());
        if !failed.is_empty() {
            println!("✗ Failed ({}):", failed.len());
            for f in &failed {
                println!("  {f}");
            }
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
    json: bool,
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

    let pb = progress_bar(issues.len() as u64);

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

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "total": issues.len(),
                "succeeded": ok,
                "failed_count": failed.len(),
                "failed": failed,
            }))?
        );
    } else {
        println!("✓ Updated {ok}/{} issues", issues.len());
        if !failed.is_empty() {
            println!("✗ Failed ({}):", failed.len());
            for f in &failed {
                println!("  {f}");
            }
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

// ─── batch manifest runner ───────────────────────────────────────────────────

async fn batch_manifest(
    client: JiraClient,
    manifest: std::path::PathBuf,
    json: bool,
) -> Result<()> {
    let content = std::fs::read_to_string(&manifest)
        .with_context(|| format!("Failed to read manifest: {}", manifest.display()))?;

    let entries: Vec<Value> =
        serde_json::from_str(&content).context("Manifest must be a JSON array of op objects")?;

    if entries.is_empty() {
        println!("Manifest is empty — nothing to run.");
        return Ok(());
    }

    println!("Running {} operations...", entries.len());
    let pb = progress_bar(entries.len() as u64);

    // Each result: {"op":..., "key":..., "status":..., "error": null|"..."}
    let mut results: Vec<Value> = Vec::new();

    for entry in &entries {
        let op = entry
            .get("op")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        pb.set_message(op.to_string());

        let result = match op {
            "create" => {
                let project = entry
                    .get("project")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let summary = entry
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let issue_type = entry
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Task")
                    .to_string();
                let assignee = entry
                    .get("assignee")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let priority = entry
                    .get("priority")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let labels: Vec<String> = entry
                    .get("labels")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|s| s.as_str())
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();
                let components: Vec<String> = entry
                    .get("components")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|s| s.as_str())
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();
                let fix_versions: Vec<String> = entry
                    .get("fix_versions")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|s| s.as_str())
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();
                let parent = entry
                    .get("parent")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let description = entry
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let custom_fields: HashMap<String, FieldValue> = entry
                    .get("fields")
                    .and_then(|v| v.as_object())
                    .map(|obj| {
                        obj.iter()
                            .map(|(k, v)| (k.clone(), FieldValue::Raw(v.clone())))
                            .collect()
                    })
                    .unwrap_or_default();

                let req = CreateIssueRequestV2 {
                    project_key: project,
                    summary,
                    description,
                    description_adf: None,
                    issue_type,
                    assignee,
                    priority,
                    labels,
                    components,
                    fix_versions,
                    parent,
                    custom_fields,
                };
                match client.create_issue_v2(req).await {
                    Ok(issue) => {
                        serde_json::json!({ "op": op, "key": issue.key, "status": "created" })
                    }
                    Err(e) => {
                        serde_json::json!({ "op": op, "key": "", "status": "failed", "error": e.to_string() })
                    }
                }
            }
            "update" => {
                let key = entry
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let req = UpdateIssueRequest {
                    summary: entry
                        .get("summary")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    assignee: entry
                        .get("assignee")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    priority: entry
                        .get("priority")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    labels: entry.get("labels").and_then(|v| v.as_array()).map(|a| {
                        a.iter()
                            .filter_map(|s| s.as_str())
                            .map(String::from)
                            .collect()
                    }),
                    components: entry.get("components").and_then(|v| v.as_array()).map(|a| {
                        a.iter()
                            .filter_map(|s| s.as_str())
                            .map(String::from)
                            .collect()
                    }),
                    ..Default::default()
                };
                match client.update_issue(&key, req).await {
                    Ok(_) => serde_json::json!({ "op": op, "key": key, "status": "updated" }),
                    Err(e) => {
                        serde_json::json!({ "op": op, "key": key, "status": "failed", "error": e.to_string() })
                    }
                }
            }
            "transition" => {
                let key = entry
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let to = entry
                    .get("to")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let trans_result: anyhow::Result<()> = async {
                    let transitions = client
                        .get_transitions(&key)
                        .await
                        .map_err(|e| anyhow::anyhow!(e))?;
                    let tid = transitions
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
                    client
                        .transition_issue(&key, &tid)
                        .await
                        .map_err(|e| anyhow::anyhow!(e))
                }
                .await;

                match trans_result {
                    Ok(_) => {
                        serde_json::json!({ "op": op, "key": key, "status": format!("transitioned to '{to}'") })
                    }
                    Err(e) => {
                        serde_json::json!({ "op": op, "key": key, "status": "failed", "error": e.to_string() })
                    }
                }
            }
            "archive" => {
                let key = entry
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                match client.archive_issues(std::slice::from_ref(&key)).await {
                    Ok(_) => serde_json::json!({ "op": op, "key": key, "status": "archived" }),
                    Err(e) => {
                        serde_json::json!({ "op": op, "key": key, "status": "failed", "error": e.to_string() })
                    }
                }
            }
            _ => {
                serde_json::json!({ "op": op, "key": "", "status": "skipped", "error": format!("Unknown op: '{op}'") })
            }
        };

        results.push(result);
        pb.inc(1);
    }

    pb.finish_and_clear();

    if json {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        let succeeded = results
            .iter()
            .filter(|r| r.get("error").map(|e| e.is_null()).unwrap_or(true))
            .count();
        println!("✓ {succeeded}/{} operations completed", results.len());
        for r in &results {
            let op_str = r.get("op").and_then(|v| v.as_str()).unwrap_or("?");
            let key_str = r.get("key").and_then(|v| v.as_str()).unwrap_or("");
            let status_str = r.get("status").and_then(|v| v.as_str()).unwrap_or("?");
            let key_display = if key_str.is_empty() {
                String::new()
            } else {
                format!(" {key_str}")
            };
            if let Some(err) = r.get("error").and_then(|v| v.as_str()) {
                println!("  ✗ {op_str}{key_display}: {err}");
            } else {
                println!("  ✓ {op_str}{key_display}: {status_str}");
            }
        }
    }

    Ok(())
}

// ─── clone / move ────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn clone_issue(
    client: JiraClient,
    key: String,
    project: Option<String>,
    summary_override: Option<String>,
    assignee: Option<String>,
    move_issue: bool,
    json: bool,
) -> Result<()> {
    // Fetch source issue
    let spinner = spinner_new(format!("Fetching {key}..."));
    let source = client
        .get_issue(&key)
        .await
        .context("Failed to fetch source issue")?;
    spinner.finish_and_clear();

    let target_project = project.unwrap_or_else(|| source.project_key.clone());
    let summary = summary_override.unwrap_or_else(|| source.summary.clone());

    // Resolve labels and components from raw fields
    let labels: Vec<String> = source
        .fields
        .get("labels")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s.as_str())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    let components: Vec<String> = source
        .fields
        .get("components")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|c| c.get("name").and_then(|n| n.as_str()))
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    let fix_versions: Vec<String> = source
        .fields
        .get("fixVersions")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.get("name").and_then(|n| n.as_str()))
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    let req = CreateIssueRequestV2 {
        project_key: target_project,
        summary,
        description: None,
        description_adf: source.description.clone(),
        issue_type: source.issue_type.clone(),
        assignee,
        priority: source.priority.clone(),
        labels,
        components,
        fix_versions,
        parent: None,
        custom_fields: HashMap::new(),
    };

    let spinner = spinner_new("Cloning issue...");
    let clone = client
        .create_issue_v2(req)
        .await
        .context("Failed to clone issue")?;
    spinner.finish_and_clear();

    if move_issue {
        // Confirm before deleting original
        let confirm = inquire::Confirm::new(&format!(
            "Delete original {key} after cloning to {}?",
            clone.key
        ))
        .with_default(false)
        .prompt()
        .context("Failed to read confirmation")?;

        if confirm {
            let spinner = spinner_new(format!("Deleting {key}..."));
            client
                .delete_issue(&key)
                .await
                .context("Failed to delete original issue")?;
            spinner.finish_and_clear();
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&clone)?);
    } else if move_issue {
        println!("✓ Moved: {} → {}", key, clone.key);
    } else {
        println!("✓ Cloned: {} → {} — {}", key, clone.key, clone.summary);
    }

    Ok(())
}

// ─── bulk create ─────────────────────────────────────────────────────────────

async fn bulk_create(client: JiraClient, manifest: std::path::PathBuf, json: bool) -> Result<()> {
    let content = std::fs::read_to_string(&manifest)
        .with_context(|| format!("Failed to read manifest: {}", manifest.display()))?;

    let entries: Vec<Value> =
        serde_json::from_str(&content).context("Manifest must be a JSON array of issue objects")?;

    if entries.is_empty() {
        println!("Manifest is empty — nothing to create.");
        return Ok(());
    }

    println!("Creating {} issues from manifest...", entries.len());
    let pb = progress_bar(entries.len() as u64);

    let mut created_issues: Vec<jira_core::model::Issue> = Vec::new();
    let mut created: Vec<String> = Vec::new();
    let mut failed: Vec<String> = Vec::new();

    for entry in &entries {
        let project_key = entry
            .get("project")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Each manifest entry must have a \"project\" field"))?
            .to_string();

        let summary = entry
            .get("summary")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Each manifest entry must have a \"summary\" field"))?
            .to_string();

        let issue_type = entry
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("Task")
            .to_string();

        let assignee = entry
            .get("assignee")
            .and_then(|v| v.as_str())
            .map(String::from);
        let priority = entry
            .get("priority")
            .and_then(|v| v.as_str())
            .map(String::from);
        let parent = entry
            .get("parent")
            .and_then(|v| v.as_str())
            .map(String::from);

        let description = entry
            .get("description")
            .and_then(|v| v.as_str())
            .map(String::from);

        let labels: Vec<String> = entry
            .get("labels")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let components: Vec<String> = entry
            .get("components")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let fix_versions: Vec<String> = entry
            .get("fix_versions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        // Custom fields from "fields" object
        let custom_fields: HashMap<String, FieldValue> = entry
            .get("fields")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), FieldValue::Raw(v.clone())))
                    .collect()
            })
            .unwrap_or_default();

        pb.set_message(summary.clone());

        let req = CreateIssueRequestV2 {
            project_key,
            summary: summary.clone(),
            description,
            description_adf: None,
            issue_type,
            assignee,
            priority,
            labels,
            components,
            parent,
            fix_versions,
            custom_fields,
        };

        match client.create_issue_v2(req).await {
            Ok(issue) => {
                created.push(format!("{} — {}", issue.key, issue.summary));
                created_issues.push(issue);
            }
            Err(e) => failed.push(format!("\"{}\" failed: {}", summary, e)),
        }
        pb.inc(1);
    }

    pb.finish_and_clear();

    if json {
        println!("{}", serde_json::to_string_pretty(&created_issues)?);
    } else {
        println!("✓ Created {}/{} issues:", created.len(), entries.len());
        for c in &created {
            println!("  {c}");
        }
        if !failed.is_empty() {
            println!("✗ Failed ({}):", failed.len());
            for f in &failed {
                println!("  {f}");
            }
        }
    }
    Ok(())
}

// ─── helpers ─────────────────────────────────────────────────────────────────

/// Parse comma-separated string into a Vec<String>. Returns empty vec for None.
fn parse_csv(input: Option<&str>) -> Vec<String> {
    match input {
        Some(s) if !s.trim().is_empty() => s
            .split(',')
            .map(|p| p.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

/// Parse `--field key=value` flags into a FieldValue map.
/// Value is parsed as JSON if valid, otherwise treated as a plain string.
fn parse_field_flags(fields: &[String]) -> Result<HashMap<String, FieldValue>> {
    let mut result = HashMap::new();
    for kv in fields {
        let (key, value) = kv.split_once('=').ok_or_else(|| {
            anyhow::anyhow!("Invalid --field format '{}': expected key=value", kv)
        })?;
        let field_value = if let Ok(json_val) = serde_json::from_str::<Value>(value) {
            FieldValue::Raw(json_val)
        } else {
            FieldValue::Text(value.to_string())
        };
        result.insert(key.to_string(), field_value);
    }
    Ok(result)
}

/// Read description from a file and convert to the right format.
/// Returns `(markdown_str, adf_value)` — at most one is Some.
fn read_description_file(
    path: Option<&std::path::Path>,
    format: &str,
) -> Result<(Option<String>, Option<Value>)> {
    let Some(p) = path else {
        return Ok((None, None));
    };
    let content = std::fs::read_to_string(p)
        .with_context(|| format!("Failed to read description file: {}", p.display()))?;
    match format {
        "adf" => {
            let adf: Value = serde_json::from_str(&content)
                .context("--description-format adf requires valid JSON ADF content")?;
            Ok((None, Some(adf)))
        }
        "text" => Ok((None, Some(jira_core::adf::plain_text_to_adf(&content)))),
        _ => Ok((Some(content), None)), // markdown (default)
    }
}

// Keep old CreateIssueRequest available for any other callers
#[allow(dead_code)]
fn _use_old_request() {
    let _ = CreateIssueRequest::default();
    let _: Option<Value> = None;
}
