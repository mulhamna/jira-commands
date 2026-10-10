use std::{collections::HashMap, path::PathBuf};

use crate::cli::interactive::require_interactive;
use crate::cli::progress::{progress_bar, spinner_new};
use crate::{
    datetime::{build_worklog_range_dates, build_worklog_started, build_worklog_started_for_date},
    notifications::scan_mention_notifications,
    version_insights::{extract_fix_versions, load_issue_version_insight},
};
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use clap::{Subcommand, ValueEnum};
use inquire::{Confirm, MultiSelect, Select, Text};
use jira_core::{
    model::{
        field::{FieldKind, FieldValue},
        CreateIssueRequestV2, CreateProjectVersionRequest, Issue, Sprint, UpdateIssueRequest,
        UpdateProjectVersionRequest,
    },
    FieldCache, IssueType, JiraClient,
};
use serde_json;
use serde_json::Value;

mod browse;
mod bulk;
mod collaboration;
mod manage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ExportFormat {
    Json,
    Csv,
}

/// Resolved request bounds for a list-like operation.
#[derive(Debug, Clone, Copy, Default)]
struct LimitOptions {
    /// Explicit `--limit` (overrides configured default).
    limit: Option<u32>,
    /// `--all` forces fetching every matching issue.
    all: bool,
    /// Configured `default_issue_limit`, used when no explicit limit is given.
    configured: Option<u32>,
}

impl LimitOptions {
    /// Effective cap: `--all` → `None` (fetch everything), otherwise the
    /// explicit `--limit`, the configured default, or `None` (fetch all).
    fn effective(self) -> Option<u32> {
        if self.all {
            None
        } else {
            self.limit.or(self.configured)
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum IssueCommand {
    /// List issues — by project, JQL, or your assigned issues
    ///
    /// Without flags, shows issues assigned to you (assignee = currentUser()).
    /// Use --project for a project overview, or --jql for full control.
    ///
    /// By default the result count follows, in priority order: the explicit
    /// `--limit`, then the configured `default_issue_limit`, then "all".
    /// "All" fetches every matching issue via pagination unless capped.
    ///
    /// Examples:
    ///   jirac issue list                              # your assigned issues
    ///   jirac issue list -p PROJ                      # all issues in project
    ///   jirac issue list -p PROJ -l 50                # up to 50 results
    ///   jirac issue list --all -p PROJ                # every issue (paged)
    ///   jirac issue list --jql 'status = "In Progress" AND project = PROJ'
    ///   jirac issue list --jql 'sprint = openSprints() AND assignee = me'
    List {
        /// Project key (e.g. PROJ). Overrides default project from config.
        #[arg(short, long, value_name = "PROJECT")]
        project: Option<String>,
        /// Raw JQL query — overrides --project when both are provided
        #[arg(long, value_name = "JQL")]
        jql: Option<String>,
        /// Maximum number of issues to return. Overrides the configured
        /// default_issue_limit when both are present.
        #[arg(short, long, value_name = "N")]
        limit: Option<u32>,
        /// Fetch every matching issue, ignoring --limit and configured limits.
        #[arg(long)]
        all: bool,
        /// Output results as JSON array
        #[arg(long)]
        json: bool,
    },

    /// Export matching issues to a file in JSON or CSV format
    ///
    /// Intended for large, machine-readable result sets (scripts, reporting,
    /// pipelines). By default fetches every matching issue via pagination;
    /// use --limit to cap the result.
    ///
    /// Examples:
    ///   jirac issue export -p PROJ --format json                 # all issues, JSON
    ///   jirac issue export -p PROJ --format csv -o issues.csv    # all issues, CSV
    ///   jirac issue export -p PROJ --limit 500 --format json -o out.json
    Export {
        /// Project key (e.g. PROJ). Overrides default project from config.
        #[arg(short, long, value_name = "PROJECT")]
        project: Option<String>,
        /// Raw JQL query — overrides --project when both are provided
        #[arg(long, value_name = "JQL")]
        jql: Option<String>,
        /// Fetch every matching issue (ignores --limit and configured limits)
        #[arg(long)]
        all: bool,
        /// Maximum number of issues to export. Omitting it exports everything.
        #[arg(short = 'l', long, value_name = "N")]
        limit: Option<u32>,
        /// Output format
        #[arg(long, value_enum, default_value = "json")]
        format: ExportFormat,
        /// Output file path. Defaults to stdout when omitted.
        #[arg(short = 'o', long, value_name = "PATH")]
        output: Option<PathBuf>,
    },

    /// Generate a daily standup summary from your assigned issues
    ///
    /// By default this inspects issues assigned to the current user and groups
    /// them into recently done, in progress, next up, and blocked buckets.
    /// Use --project to scope the report, or --jql for a custom source query.
    ///
    /// Examples:
    ///   jirac issue standup
    ///   jirac issue standup -p PROJ
    ///   jirac issue standup --jql 'assignee = currentUser() AND project = PROJ ORDER BY updated DESC'
    Standup {
        /// Project key (e.g. PROJ). Overrides default project from config.
        #[arg(short, long, value_name = "PROJECT")]
        project: Option<String>,
        /// Raw JQL query — overrides --project when both are provided
        #[arg(long, value_name = "JQL")]
        jql: Option<String>,
        /// Lookback window for the "recently done" bucket (for example 2d, 36h, 1w)
        #[arg(long, default_value = "2d", value_name = "WINDOW")]
        since: String,
        /// Maximum number of issues to inspect (default: 50, max: 100)
        #[arg(short, long, default_value = "50", value_name = "N")]
        limit: u32,
        /// Output the standup data as JSON
        #[arg(long)]
        json: bool,
    },

    /// Summarize the current or named sprint for a project
    ///
    /// Without --sprint, targets openSprints() for the project.
    ///
    /// Examples:
    ///   jirac issue sprint-summary -p PROJ
    ///   jirac issue sprint-summary -p PROJ --sprint "Sprint 24"
    #[command(name = "sprint-summary")]
    SprintSummary {
        /// Project key (e.g. PROJ). Defaults to configured project when present.
        #[arg(short, long, value_name = "PROJECT")]
        project: Option<String>,
        /// Sprint name or numeric sprint ID. Defaults to openSprints().
        #[arg(long, value_name = "SPRINT")]
        sprint: Option<String>,
        /// Maximum number of sprint issues to inspect (default: 100)
        #[arg(short, long, default_value = "100", value_name = "N")]
        limit: u32,
        /// Output the summary as JSON
        #[arg(long)]
        json: bool,
    },

    /// List project sprints and their lifecycle state
    ///
    /// Examples:
    ///   jirac issue sprints -p PROJ
    ///   jirac issue sprints -p PROJ --state active,future,closed
    #[command(name = "sprints")]
    Sprints {
        /// Project key (e.g. PROJ). Defaults to configured project when present.
        #[arg(short, long, value_name = "PROJECT")]
        project: Option<String>,
        /// Comma-separated sprint states: active,future,closed
        #[arg(long, default_value = "active,future,closed", value_name = "STATES")]
        state: String,
        /// Output sprints as JSON
        #[arg(long)]
        json: bool,
    },

    /// Create a new sprint on a scrum board for the project
    ///
    /// Examples:
    ///   jirac issue sprint-create -p PROJ --name "Sprint 24"
    ///   jirac issue sprint-create -p PROJ --name "Sprint 24" --board-id 12 --goal "Stabilize release" --start-date 2026-05-20 --end-date 2026-06-03
    #[command(name = "sprint-create")]
    SprintCreate {
        /// Project key (e.g. PROJ). Defaults to configured project when present.
        #[arg(short, long, value_name = "PROJECT")]
        project: Option<String>,
        /// Sprint name
        #[arg(long, value_name = "NAME")]
        name: String,
        /// Scrum board ID. Required only when the project maps to multiple boards.
        #[arg(long, value_name = "BOARD_ID")]
        board_id: Option<u64>,
        /// Optional sprint goal
        #[arg(long, value_name = "TEXT")]
        goal: Option<String>,
        /// Optional planned sprint start date (YYYY-MM-DD)
        #[arg(long, value_name = "YYYY-MM-DD")]
        start_date: Option<String>,
        /// Optional planned sprint end date (YYYY-MM-DD)
        #[arg(long, value_name = "YYYY-MM-DD")]
        end_date: Option<String>,
        /// Output the created sprint as JSON
        #[arg(long)]
        json: bool,
    },

    /// Start a future sprint
    ///
    /// Examples:
    ///   jirac issue sprint-start -p PROJ --sprint "Sprint 24" --end-date 2026-06-03
    ///   jirac issue sprint-start -p PROJ --sprint 42 --start-date 2026-05-20 --end-date 2026-06-03
    #[command(name = "sprint-start")]
    SprintStart {
        /// Project key (e.g. PROJ). Defaults to configured project when present.
        #[arg(short, long, value_name = "PROJECT")]
        project: Option<String>,
        /// Sprint name or numeric sprint ID
        #[arg(long, value_name = "SPRINT")]
        sprint: String,
        /// Sprint start date (YYYY-MM-DD). Defaults to today (UTC).
        #[arg(long, value_name = "YYYY-MM-DD")]
        start_date: Option<String>,
        /// Sprint end date (YYYY-MM-DD)
        #[arg(long, value_name = "YYYY-MM-DD")]
        end_date: String,
        /// Optional sprint goal override
        #[arg(long, value_name = "TEXT")]
        goal: Option<String>,
        /// Output the updated sprint as JSON
        #[arg(long)]
        json: bool,
    },

    /// Complete/close an active sprint
    ///
    /// Examples:
    ///   jirac issue sprint-complete -p PROJ --sprint "Sprint 24"
    ///   jirac issue sprint-complete -p PROJ --sprint 42 --complete-date 2026-06-03
    #[command(name = "sprint-complete")]
    SprintComplete {
        /// Project key (e.g. PROJ). Defaults to configured project when present.
        #[arg(short, long, value_name = "PROJECT")]
        project: Option<String>,
        /// Sprint name or numeric sprint ID
        #[arg(long, value_name = "SPRINT")]
        sprint: String,
        /// Completion date (YYYY-MM-DD). Defaults to today (UTC).
        #[arg(long, value_name = "YYYY-MM-DD")]
        complete_date: Option<String>,
        /// Output the updated sprint as JSON
        #[arg(long)]
        json: bool,
    },

    /// Update sprint metadata like name, goal, or planned dates
    ///
    /// Examples:
    ///   jirac issue sprint-update -p PROJ --sprint "Sprint 24" --name "Sprint 24A"
    ///   jirac issue sprint-update -p PROJ --sprint 42 --goal "Ship polish" --start-date 2026-05-20 --end-date 2026-06-03
    #[command(name = "sprint-update")]
    SprintUpdate {
        /// Project key (e.g. PROJ). Defaults to configured project when present.
        #[arg(short, long, value_name = "PROJECT")]
        project: Option<String>,
        /// Sprint name or numeric sprint ID
        #[arg(long, value_name = "SPRINT")]
        sprint: String,
        /// Rename the sprint
        #[arg(long, value_name = "NAME")]
        name: Option<String>,
        /// Set or replace the sprint goal
        #[arg(long, value_name = "TEXT")]
        goal: Option<String>,
        /// Clear the sprint goal
        #[arg(long, conflicts_with = "goal")]
        clear_goal: bool,
        /// Set or replace the sprint start date (YYYY-MM-DD)
        #[arg(long, value_name = "YYYY-MM-DD")]
        start_date: Option<String>,
        /// Clear the sprint start date
        #[arg(long, conflicts_with = "start_date")]
        clear_start_date: bool,
        /// Set or replace the sprint end date (YYYY-MM-DD)
        #[arg(long, value_name = "YYYY-MM-DD")]
        end_date: Option<String>,
        /// Clear the sprint end date
        #[arg(long, conflicts_with = "end_date")]
        clear_end_date: bool,
        /// Output the updated sprint as JSON
        #[arg(long)]
        json: bool,
    },

    /// Delete a sprint permanently
    ///
    /// Examples:
    ///   jirac issue sprint-delete -p PROJ --sprint "Sprint 24" --force
    #[command(name = "sprint-delete")]
    SprintDelete {
        /// Project key (e.g. PROJ). Defaults to configured project when present.
        #[arg(short, long, value_name = "PROJECT")]
        project: Option<String>,
        /// Sprint name or numeric sprint ID
        #[arg(long, value_name = "SPRINT")]
        sprint: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },

    /// Move an issue from its sprint back to the board backlog
    #[command(name = "sprint-remove-issue")]
    SprintRemoveIssue {
        /// Issue key (e.g. PROJ-123)
        key: String,
        /// Output the result as JSON
        #[arg(long)]
        json: bool,
    },

    /// Rank an issue immediately before or after another issue on its board
    Rank {
        /// Issue key to move
        key: String,
        /// Place the issue immediately before this issue
        #[arg(long, conflicts_with = "after", required_unless_present = "after")]
        before: Option<String>,
        /// Place the issue immediately after this issue
        #[arg(long, conflicts_with = "before", required_unless_present = "before")]
        after: Option<String>,
        /// Output the result as JSON
        #[arg(long)]
        json: bool,
    },

    /// Scan recent Jira @mentions from issue descriptions and comments
    ///
    /// This is a notification-style inbox for direct mentions. Because Jira's
    /// bell drawer is not exposed through the normal REST API used by jirac,
    /// this command derives your inbox by scanning recently updated issues and
    /// extracting ADF mention nodes that target your account.
    ///
    /// Examples:
    ///   jirac issue notifications
    ///   jirac issue notifications -p PROJ --since 3d
    ///   jirac issue notifications --limit 100 --json
    Notifications {
        /// Project key (e.g. PROJ). Defaults to configured project when present.
        #[arg(short, long, value_name = "PROJECT")]
        project: Option<String>,
        /// Lookback window in Jira relative date syntax (e.g. 7d, 48h)
        #[arg(long, default_value = "7d", value_name = "WINDOW")]
        since: String,
        /// Maximum number of recently updated issues to inspect (default: 50, max: 100)
        #[arg(short, long, default_value = "50", value_name = "N")]
        limit: u32,
        /// Output notifications as JSON array
        #[arg(long)]
        json: bool,
    },

    /// View full issue details — description, attachments, and metadata
    ///
    /// Displays: type, status, project, priority, assignee, reporter,
    /// created/updated timestamps, attachment list, and rendered description.
    ///
    /// Use --versions when you also want fix-version backlog preview for the
    /// issue's current project/version assignment.
    ///
    /// Examples:
    ///   jirac issue view PROJ-123
    ///   jirac issue view PROJ-123 --versions
    ///   jirac issue view PROJ-123 --versions --version-limit 10
    ///   jirac issue view PROJ-123 --json
    View {
        /// Issue key (e.g. PROJ-123)
        key: String,
        /// Include fix-version backlog preview for this issue
        #[arg(long)]
        versions: bool,
        /// Maximum number of backlog issues to preview per fix version
        #[arg(long, default_value = "5", value_name = "N")]
        version_limit: u32,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Browse project fix versions, preview backlog items, or update version metadata
    ///
    /// Without --version, lists fix versions for the project.
    /// With --version, shows open backlog items assigned to that fix version.
    /// Add one or more update flags to modify version metadata instead.
    ///
    /// Examples:
    ///   jirac issue versions -p PROJ
    ///   jirac issue versions -p PROJ --version "v1.2.0"
    ///   jirac issue versions -p PROJ --version "v1.2.0" --limit 15
    ///   jirac issue versions -p PROJ --version "v1.2.0" --set-release-date 2026-05-30 --released
    ///   jirac issue versions -p PROJ --create --version "v1.3.0" --description "June release"
    #[command(name = "versions")]
    Versions {
        /// Project key (e.g. PROJ). Defaults to configured project when present.
        #[arg(short, long, value_name = "PROJECT")]
        project: Option<String>,
        /// Specific fix version name to inspect or update
        #[arg(long, value_name = "VERSION")]
        version: Option<String>,
        /// Maximum number of backlog issues to preview (default: 10)
        #[arg(short, long, default_value = "10", value_name = "N")]
        limit: u32,
        /// Create a new fix version instead of listing or previewing
        #[arg(long)]
        create: bool,
        /// Rename the selected version
        #[arg(long, value_name = "NAME")]
        set_name: Option<String>,
        /// Set or replace the version description
        #[arg(long, value_name = "TEXT")]
        description: Option<String>,
        /// Clear the version description
        #[arg(long, conflicts_with = "description")]
        clear_description: bool,
        /// Set or replace the version release date (YYYY-MM-DD)
        #[arg(long, value_name = "YYYY-MM-DD")]
        set_release_date: Option<String>,
        /// Clear the version release date
        #[arg(long, conflicts_with = "set_release_date")]
        clear_release_date: bool,
        /// Set or replace the version start date (YYYY-MM-DD)
        #[arg(long, value_name = "YYYY-MM-DD")]
        set_start_date: Option<String>,
        /// Clear the version start date
        #[arg(long, conflicts_with = "set_start_date")]
        clear_start_date: bool,
        /// Mark the version as released
        #[arg(long, conflicts_with = "unreleased")]
        released: bool,
        /// Mark the version as unreleased
        #[arg(long, conflicts_with = "released")]
        unreleased: bool,
        /// Mark the version as archived
        #[arg(long, conflicts_with = "unarchived")]
        archived: bool,
        /// Mark the version as unarchived
        #[arg(long, conflicts_with = "archived")]
        unarchived: bool,
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
    ///   jirac issue fields -p PROJ --issue-type Bug
    ///
    /// Examples:
    ///   jirac issue create                                         # fully interactive
    ///   jirac issue create -p PROJ -s "Fix login bug" -t Bug
    ///   jirac issue create -p PROJ -s "API story" -t Story --assignee me --labels "backend,api"
    ///   jirac issue create -p PROJ -s "Sub-task" -t Sub-task --parent PROJ-100
    ///   jirac issue create -p PROJ -s "Feat" --description-file description.md
    ///   jirac issue create -p PROJ -s "Fix" --field story_points=5 --field customfield_10020=sprint1
    ///   jirac issue create -p PROJ -s "Plan sprint work" --issue-type Task --sprint "Sprint 24"
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
        /// Sprint to assign on create — accepts a sprint ID or exact sprint name
        #[arg(long, value_name = "SPRINT_ID|NAME")]
        sprint: Option<String>,
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
        /// Run `jirac issue fields -p PROJ --issue-type Bug` to list all field IDs.
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
    /// Note: use `jirac issue change-type` for native issue type changes.
    ///
    /// Examples:
    ///   jirac issue update PROJ-123 --summary "Updated title"
    ///   jirac issue update PROJ-123 --assignee me --priority High
    ///   jirac issue update PROJ-123 --description-file updated.md
    ///   jirac issue update PROJ-123 --labels "bug,backend" --components "auth"
    ///   jirac issue update PROJ-123 --field story_points=8
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
        /// Run `jirac issue fields -p PROJ --issue-type Bug` to list all field IDs.
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
    ///   jirac issue delete PROJ-123
    ///   jirac issue delete PROJ-123 --force      # skip confirmation prompt
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
    ///   jirac api get /rest/api/3/issue/PROJ-123/transitions
    ///
    /// Examples:
    ///   jirac issue transition PROJ-123                 # interactive picker
    ///   jirac issue transition PROJ-123 "In Progress"
    ///   jirac issue transition PROJ-123 Done
    ///   jirac issue transition PROJ-123 31              # by transition ID
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
    ///   jirac issue attach PROJ-123 screenshot.png
    ///   jirac issue attach PROJ-123 report.pdf logs.txt dump.zip
    ///   jirac issue attach PROJ-123 ~/Downloads/output.json
    Attach {
        /// Issue key (e.g. PROJ-123)
        key: String,
        /// One or more file paths to upload as attachments
        #[arg(required = true, value_name = "FILE")]
        files: Vec<std::path::PathBuf>,
    },

    /// Manage attachments on an issue (list, download, delete)
    ///
    /// Examples:
    ///   jirac issue attachment list PROJ-123
    ///   jirac issue attachment download 10100 --out ./tmp
    ///   jirac issue attachment delete 10100 --force
    Attachment {
        #[command(subcommand)]
        command: AttachmentCommand,
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
    ///   jirac issue fields -p PROJ               # interactive issue type picker
    ///   jirac issue fields -p PROJ --issue-type Bug
    ///   jirac issue fields -p PROJ --issue-type Story --required-only
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

    /// Render and validate description content before sending it to Jira
    ///
    /// Useful for previewing how Markdown or plain text will be converted into
    /// Atlassian Document Format (ADF), or for validating raw ADF JSON input.
    ///
    /// Examples:
    ///   jirac issue render --input desc.md
    ///   jirac issue render --input desc.md --format markdown --output text
    ///   jirac issue render --input desc.adf.json --format adf
    Render {
        /// Input file to read. If omitted, reads from stdin.
        #[arg(long, value_name = "FILE")]
        input: Option<std::path::PathBuf>,
        /// Input format: markdown (default), text, or adf
        #[arg(long, value_name = "FORMAT", default_value = "markdown")]
        format: String,
        /// Output format: adf (default) or text
        #[arg(long, value_name = "FORMAT", default_value = "adf")]
        output: String,
    },

    /// Manage comments on an issue
    ///
    /// List, add, edit, or delete a comment in Markdown.
    /// Markdown is converted to ADF before sending to Jira.
    ///
    /// Examples:
    ///   jirac issue comment list PROJ-123
    ///   jirac issue comment add PROJ-123 --body "Need follow-up from backend"
    ///   jirac issue comment add PROJ-123 --file note.md
    Comment {
        /// Issue key (e.g. PROJ-123)
        key: String,
        #[command(subcommand)]
        command: CommentCommand,
    },

    /// Add the same Markdown comment to many issues
    ///
    /// Targets can come from a JQL query or an explicit key list.
    /// Prompts for confirmation unless --force is used.
    ///
    /// Examples:
    ///   jirac issue bulk-comment --jql 'project = PROJ AND status = "In Progress"' --body "QA started verification"
    ///   jirac issue bulk-comment --keys PROJ-123 PROJ-456 --file note.md --force
    #[command(name = "bulk-comment")]
    BulkComment {
        /// JQL query to select issues
        #[arg(long, value_name = "JQL", conflicts_with = "keys")]
        jql: Option<String>,
        /// Explicit issue keys (space- or comma-separated)
        #[arg(long, value_name = "KEY", num_args = 1.., value_delimiter = ',', conflicts_with = "jql")]
        keys: Vec<String>,
        /// Comment body in Markdown
        #[arg(short, long, value_name = "TEXT", conflicts_with = "file")]
        body: Option<String>,
        /// Read comment body from a Markdown file
        #[arg(long, value_name = "FILE", conflicts_with = "body")]
        file: Option<std::path::PathBuf>,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
        /// Output result summary as JSON
        #[arg(long)]
        json: bool,
    },

    /// Manage time tracking (worklogs) on an issue
    ///
    /// List, add, edit, or delete a worklog entry.
    ///
    /// Use `jirac issue worklog update KEY ID` to change only the supplied fields.
    ///
    /// Time format: Jira duration syntax — "2h", "30m", "1d", "1h 30m"
    /// Note: 1d = 8 working hours (default Jira configuration).
    ///
    /// Examples:
    ///   jirac issue worklog list PROJ-123
    ///   jirac issue worklog add PROJ-123 --time "2h 30m"
    ///   jirac issue worklog add PROJ-123 --time 1d --comment "Implemented auth"
    ///   jirac issue worklog delete PROJ-123 <worklog-id>
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
    ///   jirac issue bulk-transition --jql 'project = PROJ AND status = "To Do"' --to "In Progress"
    ///   jirac issue bulk-transition --jql 'assignee = me AND sprint = openSprints()' --to Done --force
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
    ///   jirac issue bulk-update --jql 'project = PROJ AND assignee = EMPTY' --assignee me
    ///   jirac issue bulk-update --jql 'project = PROJ AND priority = Low' --priority High --force
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
    ///   jirac issue archive --jql 'project = PROJ AND status = Done AND updated < -1y'
    ///   jirac issue archive --jql 'project = PROJ AND status = Done' --force
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
    /// Use --move to delete the original after cloning.
    /// For Jira-native move semantics that preserve issue identity/history,
    /// use `jirac issue move` instead.
    ///
    /// Examples:
    ///   jirac issue clone PROJ-123                      # clone in same project
    ///   jirac issue clone PROJ-123 --project NEWPROJ    # clone to another project
    ///   jirac issue clone PROJ-123 --summary "Copy: original title"
    ///   jirac issue clone PROJ-123 --move               # clone then delete original
    ///   jirac issue clone PROJ-123 --project OTHER --json
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

    /// Change an issue to another issue type using Jira's native move semantics
    ///
    /// Keeps the existing issue identity and history. This uses Jira's native
    /// move API under the hood, even when staying within the same project.
    ///
    /// By default the issue stays in its current project. If the issue type is
    /// not available in that project, Jira will reject the move.
    ///
    /// Examples:
    ///   jirac issue change-type PROJ-123 Bug
    ///   jirac issue change-type PROJ-123 Story --json
    #[command(name = "change-type")]
    ChangeType {
        /// Issue key (e.g. PROJ-123)
        key: String,
        /// Target issue type name in the current project (e.g. Bug, Story, Task)
        issue_type: String,
        /// Output the moved issue as JSON
        #[arg(long)]
        json: bool,
    },

    /// Move an issue to another project using Jira's native move semantics
    ///
    /// Keeps the existing issue identity and history. By default this keeps the
    /// current issue type name, resolved in the target project. Use --issue-type
    /// to override when the target project uses a different issue type.
    ///
    /// This command uses Jira's native bulk move API for a single issue, with
    /// default field/status/classification inference enabled. If Jira requires
    /// explicit mappings for your workflow, the API may reject the move.
    ///
    /// Examples:
    ///   jirac issue move PROJ-123 OTHER
    ///   jirac issue move PROJ-123 OTHER --issue-type Task
    ///   jirac issue move PROJ-123 OTHER --json
    Move {
        /// Issue key (e.g. PROJ-123)
        key: String,
        /// Target project key (e.g. OTHER)
        project: String,
        /// Target issue type name in the destination project. Defaults to the current issue type name.
        #[arg(long, value_name = "TYPE")]
        issue_type: Option<String>,
        /// Output the moved issue as JSON
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
    ///   jirac issue jql              # build query, print it
    ///   jirac issue jql --run        # build and run immediately
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
        /// JQL builder params as JSON. Accepts a literal JSON object or @path/to/file.json.
        /// Skips the interactive prompts.
        ///
        /// Schema: see `jira_core::jql::JqlParams`. Example:
        ///   {"project":"PROJ","status":["In Progress"],
        ///    "assignee":[{"type":"current_user"}],
        ///    "order_by":[["updated","desc"]]}
        #[arg(long, value_name = "JSON")]
        params: Option<String>,
    },

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
    ///   jirac issue batch --manifest ops.json
    ///   jirac issue batch --manifest ops.json --json
    Batch {
        /// Path to the JSON manifest file (array of op objects)
        #[arg(long, value_name = "FILE")]
        manifest: std::path::PathBuf,
        /// Output results as JSON array
        #[arg(long)]
        json: bool,
    },

    /// Create multiple issues from a JSON manifest file
    ///
    /// The manifest is a JSON array of issue objects. Each object supports
    /// the same fields as `jirac issue create` flags.
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
    ///   jirac issue bulk-create --manifest issues.json
    #[command(name = "bulk-create")]
    BulkCreate {
        /// Path to the JSON manifest file (array of issue objects)
        #[arg(long, value_name = "FILE")]
        manifest: std::path::PathBuf,
        /// Output created issues as JSON array
        #[arg(long)]
        json: bool,
    },

    /// Manage issue links (blocks, relates, etc.)
    ///
    /// List link types, create a new link, or delete a link by ID.
    ///
    /// Examples:
    ///   jirac issue link list-types
    ///   jirac issue link add PROJ-123 PROJ-456 --type Blocks
    ///   jirac issue link delete 10000
    Link {
        #[command(subcommand)]
        command: LinkCommand,
    },

    /// Manage issue watchers (add/list/remove)
    ///
    /// Examples:
    ///   jirac issue watch PROJ-123 add
    ///   jirac issue watch PROJ-123 add --account-id 5b10ac8d82e05b22cc7d4ef5
    ///   jirac issue watch PROJ-123 list
    ///   jirac issue watch PROJ-123 rm 5b10ac8d82e05b22cc7d4ef5
    Watch {
        /// Issue key (e.g. PROJ-123)
        key: String,
        #[command(subcommand)]
        command: WatchCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum WatchCommand {
    /// Add a watcher to the issue (defaults to the current user)
    Add {
        /// AccountId of the user to add (defaults to the current authenticated user)
        #[arg(long, value_name = "ACCOUNT_ID")]
        account_id: Option<String>,
    },

    /// List all watchers on the issue
    List,

    /// Remove a watcher from the issue
    Rm {
        /// AccountId of the user to remove
        account_id: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum AttachmentCommand {
    /// List all attachments on an issue
    List {
        /// Issue key (e.g. PROJ-123)
        key: String,
        /// Output as JSON array
        #[arg(long)]
        json: bool,
    },

    /// Download an attachment by ID
    Download {
        /// Attachment ID (visible via `jirac issue attachment list`)
        id: String,
        /// Output directory (defaults to current directory)
        #[arg(long, value_name = "DIR")]
        out: Option<std::path::PathBuf>,
        /// Override the filename (defaults to the server-provided name)
        #[arg(long, value_name = "NAME")]
        filename: Option<String>,
        /// Overwrite if the destination file already exists
        #[arg(long)]
        force: bool,
    },

    /// Delete an attachment by ID
    Delete {
        /// Attachment ID to delete
        id: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum CommentCommand {
    /// List all comments on the issue
    List,

    /// Add a comment to an issue
    ///
    /// Examples:
    ///   jirac issue comment add PROJ-123 --body "Please verify in staging"
    ///   jirac issue comment add PROJ-123 --file note.md
    Add {
        /// Comment body in Markdown
        #[arg(short, long, value_name = "TEXT", conflicts_with = "file")]
        body: Option<String>,
        /// Read comment body from a Markdown file
        #[arg(long, value_name = "FILE", conflicts_with = "body")]
        file: Option<std::path::PathBuf>,
    },

    /// Replace a comment body
    Update {
        /// Comment ID shown by `jirac issue comment list`
        id: String,
        /// Comment body in Markdown
        #[arg(short, long, value_name = "TEXT", conflicts_with = "file")]
        body: Option<String>,
        /// Read comment body from a Markdown file
        #[arg(long, value_name = "FILE", conflicts_with = "body")]
        file: Option<std::path::PathBuf>,
    },

    /// Delete a comment
    Delete {
        /// Comment ID shown by `jirac issue comment list`
        id: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
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
    ///   jirac issue worklog add PROJ-123 --time "2h 30m"
    ///   jirac issue worklog add PROJ-123 --time 1d --comment "Implemented login"
    ///   jirac issue worklog add PROJ-123 --time 2h --date 2026-04-21 --start 09:30
    ///   jirac issue worklog add PROJ-123 --time 2h --from 2026-04-21 --to 2026-04-25 --exclude-weekends
    /// Range mode creates one worklog per included date.
    Add {
        /// Time spent in Jira duration format (e.g. "2h", "30m", "1d", "1h 30m")
        #[arg(short, long, value_name = "DURATION")]
        time: String,
        /// Optional comment describing the work done
        #[arg(short, long, value_name = "TEXT")]
        comment: Option<String>,
        /// Optional single work date in local time (YYYY-MM-DD)
        #[arg(long, value_name = "DATE", conflicts_with_all = ["from", "to"])]
        date: Option<String>,
        /// Optional start time in local time (HH:MM or HH:MM:SS)
        #[arg(long, value_name = "TIME")]
        start: Option<String>,
        /// Start date for inclusive range logging (YYYY-MM-DD)
        #[arg(long, value_name = "DATE", requires = "to", conflicts_with = "date")]
        from: Option<String>,
        /// End date for inclusive range logging (YYYY-MM-DD)
        #[arg(long, value_name = "DATE", requires = "from", conflicts_with = "date")]
        to: Option<String>,
        /// Skip Saturday/Sunday entries when using --from/--to
        #[arg(long)]
        exclude_weekends: bool,
    },

    /// Update an existing worklog entry
    Update {
        /// Worklog ID shown by `jirac issue worklog list`
        id: String,
        /// Replace the time spent (e.g. 2h, 30m)
        #[arg(short, long, value_name = "DURATION")]
        time: Option<String>,
        /// Replace the worklog comment
        #[arg(short, long, value_name = "TEXT")]
        comment: Option<String>,
        /// Replace the work date (YYYY-MM-DD)
        #[arg(long, value_name = "DATE")]
        date: Option<String>,
        /// Replace the start time (HH:MM or HH:MM:SS)
        #[arg(long, value_name = "TIME")]
        start: Option<String>,
    },

    /// Delete a worklog entry
    ///
    /// Use `jirac issue worklog list KEY` to find the worklog ID.
    /// Prompts for confirmation unless --force is used.
    ///
    /// Examples:
    ///   jirac issue worklog delete PROJ-123 12345
    ///   jirac issue worklog delete PROJ-123 12345 --force
    Delete {
        /// Worklog ID (see: jirac issue worklog list PROJ-123)
        id: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum LinkCommand {
    /// List available issue link types
    #[command(name = "list-types")]
    ListTypes {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Link two issues together
    Add {
        /// Outward issue key (the "source" of the link, e.g. the blocker)
        outward: String,
        /// Inward issue key (the "target" of the link, e.g. the blocked issue)
        inward: String,
        /// Link type name (e.g. "Blocks", "Relates", "Duplicate")
        #[arg(short, long, value_name = "TYPE")]
        link_type: String,
        /// Optional comment to add to the link
        #[arg(short, long, value_name = "TEXT")]
        comment: Option<String>,
    },

    /// Delete an issue link by ID
    Delete {
        /// Issue link ID
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
    default_issue_limit: Option<u32>,
) -> Result<()> {
    match cmd {
        command @ (IssueCommand::List { .. }
        | IssueCommand::Export { .. }
        | IssueCommand::Standup { .. }
        | IssueCommand::SprintSummary { .. }
        | IssueCommand::Sprints { .. }
        | IssueCommand::SprintCreate { .. }
        | IssueCommand::SprintStart { .. }
        | IssueCommand::SprintComplete { .. }
        | IssueCommand::SprintUpdate { .. }
        | IssueCommand::SprintDelete { .. }
        | IssueCommand::SprintRemoveIssue { .. }
        | IssueCommand::Rank { .. }
        | IssueCommand::Notifications { .. }
        | IssueCommand::View { .. }
        | IssueCommand::Versions { .. }) => {
            browse::handle_command(client, command, default_project, default_issue_limit).await
        }
        command @ (IssueCommand::Create { .. }
        | IssueCommand::Update { .. }
        | IssueCommand::Delete { .. }
        | IssueCommand::Transition { .. }
        | IssueCommand::Attach { .. }
        | IssueCommand::Attachment { .. }
        | IssueCommand::Fields { .. }
        | IssueCommand::Render { .. }) => {
            manage::handle_command(client, command, default_project).await
        }
        command @ (IssueCommand::Comment { .. }
        | IssueCommand::Worklog { .. }
        | IssueCommand::Watch { .. }
        | IssueCommand::BulkComment { .. }) => {
            collaboration::handle_issue_actions(client, command).await
        }
        command @ (IssueCommand::Link { .. }
        | IssueCommand::BulkTransition { .. }
        | IssueCommand::BulkUpdate { .. }
        | IssueCommand::Archive { .. }
        | IssueCommand::Jql { .. }
        | IssueCommand::BulkCreate { .. }
        | IssueCommand::Clone { .. }
        | IssueCommand::ChangeType { .. }
        | IssueCommand::Move { .. }
        | IssueCommand::Batch { .. }) => bulk::handle_command(client, command).await,
    }
}

async fn handle_agile_issue_order(client: JiraClient, command: IssueCommand) -> Result<()> {
    match command {
        IssueCommand::SprintRemoveIssue { key, json } => {
            client
                .move_issue_to_backlog(&key)
                .await
                .context("Failed to move issue to backlog")?;
            if json {
                println!("{}", serde_json::json!({"key": key, "moved_to": "backlog"}));
            } else {
                println!("✓ Moved {key} back to the board backlog");
            }
            Ok(())
        }
        IssueCommand::Rank {
            key,
            before,
            after,
            json,
        } => {
            let (relative, is_before) = before
                .as_deref()
                .map(|target| (target, true))
                .or_else(|| after.as_deref().map(|target| (target, false)))
                .context("Provide either --before or --after")?;
            client
                .rank_issue(&key, relative, is_before)
                .await
                .context("Failed to rank issue")?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "key": key,
                        "relative_issue_key": relative,
                        "position": if is_before { "before" } else { "after" }
                    })
                );
            } else {
                println!(
                    "✓ Ranked {key} {} {relative}",
                    if is_before { "before" } else { "after" }
                );
            }
            Ok(())
        }
        _ => anyhow::bail!("Unsupported Agile issue-order command"),
    }
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
