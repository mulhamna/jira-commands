use std::collections::HashMap;

use crate::cli::progress::{progress_bar, spinner_new};
use crate::{
    datetime::{build_worklog_range_dates, build_worklog_started, build_worklog_started_for_date},
    notifications::scan_mention_notifications,
    version_insights::{extract_fix_versions, load_issue_version_insight},
};
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use clap::Subcommand;
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

#[derive(Debug, Subcommand)]
pub enum IssueCommand {
    /// List issues — by project, JQL, or your assigned issues
    ///
    /// Without flags, shows issues assigned to you (assignee = currentUser()).
    /// Use --project for a project overview, or --jql for full control.
    ///
    /// Examples:
    ///   jirac issue list                              # your assigned issues
    ///   jirac issue list -p PROJ                      # all issues in project
    ///   jirac issue list -p PROJ -l 50                # up to 50 results
    ///   jirac issue list --jql 'status = "In Progress" AND project = PROJ'
    ///   jirac issue list --jql 'sprint = openSprints() AND assignee = me'
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
    /// List existing comments or add a new comment in Markdown.
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
    /// Log time, list existing entries, or delete a worklog.
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
) -> Result<()> {
    match cmd {
        IssueCommand::List {
            project,
            jql,
            limit,
            json,
        } => list_issues(client, project.or(default_project), jql, limit, json).await,
        IssueCommand::Standup {
            project,
            jql,
            since,
            limit,
            json,
        } => standup_summary(client, project.or(default_project), jql, since, limit, json).await,
        IssueCommand::SprintSummary {
            project,
            sprint,
            limit,
            json,
        } => sprint_summary(client, project.or(default_project), sprint, limit, json).await,
        IssueCommand::Sprints {
            project,
            state,
            json,
        } => list_sprints(client, project.or(default_project), state, json).await,
        IssueCommand::SprintCreate {
            project,
            name,
            board_id,
            goal,
            start_date,
            end_date,
            json,
        } => {
            create_sprint(
                client,
                project.or(default_project),
                name,
                board_id,
                goal,
                start_date,
                end_date,
                json,
            )
            .await
        }
        IssueCommand::SprintStart {
            project,
            sprint,
            start_date,
            end_date,
            goal,
            json,
        } => {
            start_sprint(
                client,
                project.or(default_project),
                sprint,
                start_date,
                end_date,
                goal,
                json,
            )
            .await
        }
        IssueCommand::SprintComplete {
            project,
            sprint,
            complete_date,
            json,
        } => {
            sprint_complete(
                client,
                project.or(default_project),
                sprint,
                complete_date,
                json,
            )
            .await
        }
        IssueCommand::SprintUpdate {
            project,
            sprint,
            name,
            goal,
            clear_goal,
            start_date,
            clear_start_date,
            end_date,
            clear_end_date,
            json,
        } => {
            let update = SprintUpdateArgs {
                name,
                goal,
                clear_goal,
                start_date,
                clear_start_date,
                end_date,
                clear_end_date,
            };
            update_sprint_command(client, project.or(default_project), sprint, update, json).await
        }
        IssueCommand::SprintDelete {
            project,
            sprint,
            force,
        } => sprint_delete(client, project.or(default_project), sprint, force).await,
        IssueCommand::Notifications {
            project,
            since,
            limit,
            json,
        } => notifications(client, project.or(default_project), since, limit, json).await,
        IssueCommand::View {
            key,
            versions,
            version_limit,
            json,
        } => view_issue(client, key, versions, version_limit, json).await,
        IssueCommand::Versions {
            project,
            version,
            limit,
            create,
            set_name,
            description,
            clear_description,
            set_release_date,
            clear_release_date,
            set_start_date,
            clear_start_date,
            released,
            unreleased,
            archived,
            unarchived,
            json,
        } => {
            let update = ProjectVersionUpdateArgs {
                create,
                set_name,
                description,
                clear_description,
                set_release_date,
                clear_release_date,
                set_start_date,
                clear_start_date,
                released,
                unreleased,
                archived,
                unarchived,
            };
            view_project_versions(
                client,
                project.or(default_project),
                version,
                limit,
                update,
                json,
            )
            .await
        }
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
            sprint,
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
                sprint,
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
        IssueCommand::Link { command } => handle_link_command(client, command).await,
        IssueCommand::Attach { key, files } => attach_files(client, key, files).await,
        IssueCommand::Attachment { command } => attachment(client, command).await,
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
        IssueCommand::Render {
            input,
            format,
            output,
        } => render_issue_content(input, format, output),
        IssueCommand::Comment { key, command } => comment(client, key, command).await,
        IssueCommand::Watch { key, command } => watch(client, key, command).await,
        IssueCommand::BulkComment {
            jql,
            keys,
            body,
            file,
            force,
            json,
        } => bulk_comment(client, jql, keys, body, file, force, json).await,
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
        IssueCommand::Jql { run, params } => jql_builder(client, run, params).await,
        IssueCommand::BulkCreate { manifest, json } => bulk_create(client, manifest, json).await,
        IssueCommand::Clone {
            key,
            project,
            summary,
            assignee,
            r#move,
            json,
        } => clone_issue(client, key, project, summary, assignee, r#move, json).await,
        IssueCommand::ChangeType {
            key,
            issue_type,
            json,
        } => change_issue_type(client, key, issue_type, json).await,
        IssueCommand::Move {
            key,
            project,
            issue_type,
            json,
        } => move_issue_native(client, key, project, issue_type, json).await,
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

async fn notifications(
    client: JiraClient,
    project: Option<String>,
    since: String,
    limit: u32,
    json: bool,
) -> Result<()> {
    let spinner = spinner_new("Scanning recent Jira mentions...");
    let scan = scan_mention_notifications(&client, project.as_deref(), &since, limit).await?;
    spinner.finish_and_clear();

    if json {
        println!("{}", serde_json::to_string_pretty(&scan.entries)?);
        return Ok(());
    }

    if scan.entries.is_empty() {
        println!("No Jira mentions found for the last {}.", since);
        if scan.comment_errors > 0 {
            eprintln!(
                "warning: failed to inspect comments on {} issue(s) during the scan",
                scan.comment_errors
            );
        }
        return Ok(());
    }

    println!(
        "{:<8} {:<12} {:<18} {:<20} {:<22} SUMMARY / EXCERPT",
        "STATUS", "ISSUE", "SOURCE", "WHEN", "AUTHOR"
    );
    println!("{}", "─".repeat(110));
    for entry in &scan.entries {
        println!(
            "{:<8} {:<12} {:<18} {:<20} {:<22} {} — {}",
            if entry.read { "read" } else { "unread" },
            entry.issue.key,
            truncate(&entry.source, 17),
            truncate(&entry.created, 19),
            truncate(entry.author.as_deref().unwrap_or("—"), 21),
            truncate(&entry.issue.summary, 32),
            truncate(&entry.excerpt, 48),
        );
    }

    println!(
        "\nScanned {} recent issue(s) with JQL: {}",
        scan.scanned_issues, scan.jql
    );
    if scan.comment_errors > 0 {
        eprintln!(
            "warning: failed to inspect comments on {} issue(s) during the scan",
            scan.comment_errors
        );
    }

    Ok(())
}

// ─── view ────────────────────────────────────────────────────────────────────

async fn view_issue(
    client: JiraClient,
    key: String,
    versions: bool,
    version_limit: u32,
    json: bool,
) -> Result<()> {
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

    let fix_versions = extract_fix_versions(&issue.fields);
    if !fix_versions.is_empty() {
        println!();
        println!("  Fix Versions: {}", fix_versions.join(", "));
    }

    if versions {
        let version_insight = load_issue_version_insight(&client, &key, version_limit)
            .await
            .ok();
        if let Some(insight) = &version_insight {
            if !insight.issue_fix_versions.is_empty() {
                println!();
                println!("  Fix Version Backlog Preview:");
                for version_name in &insight.issue_fix_versions {
                    print_version_summary(version_name, insight);
                }
            }
        }
    }

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

fn print_version_summary(
    version_name: &str,
    insight: &crate::version_insights::IssueVersionInsight,
) {
    if let Some(version) = insight
        .project_versions
        .iter()
        .find(|version| version.name == *version_name)
    {
        let mut badges = Vec::new();
        if version.archived {
            badges.push("archived".to_string());
        } else if version.released {
            badges.push("released".to_string());
        } else {
            badges.push("unreleased".to_string());
        }
        if let Some(date) = version.release_date.as_deref() {
            badges.push(format!("release {}", &date[..10.min(date.len())]));
        }
        if badges.is_empty() {
            println!("    • {}", version.name);
        } else {
            println!("    • {} ({})", version.name, badges.join(", "));
        }
    } else {
        println!("    • {version_name}");
    }

    if let Some(preview) = insight
        .previews
        .iter()
        .find(|preview| preview.version.name == *version_name)
    {
        println!("      Open backlog: {}", preview.total_open);
        if preview.issues.is_empty() {
            println!("        ✓ No open backlog items");
        } else {
            for backlog_issue in &preview.issues {
                println!(
                    "        - {} [{}] {}",
                    backlog_issue.key, backlog_issue.status, backlog_issue.summary
                );
            }
            if preview.total_open > preview.issues.len() as u64 {
                println!(
                    "        … {} more",
                    preview
                        .total_open
                        .saturating_sub(preview.issues.len() as u64)
                );
            }
        }
    }
}

#[derive(Debug, Clone)]
struct ProjectVersionUpdateArgs {
    create: bool,
    set_name: Option<String>,
    description: Option<String>,
    clear_description: bool,
    set_release_date: Option<String>,
    clear_release_date: bool,
    set_start_date: Option<String>,
    clear_start_date: bool,
    released: bool,
    unreleased: bool,
    archived: bool,
    unarchived: bool,
}

impl ProjectVersionUpdateArgs {
    fn has_changes(&self) -> bool {
        self.create
            || self.set_name.is_some()
            || self.description.is_some()
            || self.clear_description
            || self.set_release_date.is_some()
            || self.clear_release_date
            || self.set_start_date.is_some()
            || self.clear_start_date
            || self.released
            || self.unreleased
            || self.archived
            || self.unarchived
    }

    fn to_create_request(
        &self,
        project_key: &str,
        version_name: &str,
    ) -> Result<CreateProjectVersionRequest> {
        Ok(CreateProjectVersionRequest {
            name: version_name.trim().to_string(),
            project: project_key.to_string(),
            description: normalize_optional_text(self.description.as_deref()),
            archived: self.archived,
            released: self.released,
            release_date: if self.clear_release_date {
                None
            } else {
                self.set_release_date
                    .as_deref()
                    .map(|value| validate_ymd_date(value, "release date"))
                    .transpose()?
            },
            start_date: if self.clear_start_date {
                None
            } else {
                self.set_start_date
                    .as_deref()
                    .map(|value| validate_ymd_date(value, "start date"))
                    .transpose()?
            },
        })
    }

    fn to_request(&self) -> Result<UpdateProjectVersionRequest> {
        let release_date = if self.clear_release_date {
            Some(String::new())
        } else if let Some(value) = self.set_release_date.as_deref() {
            Some(validate_ymd_date(value, "release date")?)
        } else {
            None
        };

        let start_date = if self.clear_start_date {
            Some(String::new())
        } else if let Some(value) = self.set_start_date.as_deref() {
            Some(validate_ymd_date(value, "start date")?)
        } else {
            None
        };

        Ok(UpdateProjectVersionRequest {
            name: self
                .set_name
                .as_deref()
                .map(|value| value.trim().to_string()),
            description: if self.clear_description {
                Some(String::new())
            } else {
                normalize_optional_text(self.description.as_deref())
            },
            archived: if self.archived {
                Some(true)
            } else if self.unarchived {
                Some(false)
            } else {
                None
            },
            released: if self.released {
                Some(true)
            } else if self.unreleased {
                Some(false)
            } else {
                None
            },
            release_date,
            start_date,
        })
    }
}

struct SprintUpdateArgs {
    name: Option<String>,
    goal: Option<String>,
    clear_goal: bool,
    start_date: Option<String>,
    clear_start_date: bool,
    end_date: Option<String>,
    clear_end_date: bool,
}

impl SprintUpdateArgs {
    fn has_changes(&self) -> bool {
        self.name.is_some()
            || self.goal.is_some()
            || self.clear_goal
            || self.start_date.is_some()
            || self.clear_start_date
            || self.end_date.is_some()
            || self.clear_end_date
    }

    fn to_request(&self) -> Result<Value> {
        let mut body = serde_json::Map::new();

        if let Some(name) = self
            .name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            body.insert("name".into(), Value::String(name.to_string()));
        }

        if self.clear_goal {
            body.insert("goal".into(), Value::String(String::new()));
        } else if let Some(goal) = normalize_optional_text(self.goal.as_deref()) {
            body.insert("goal".into(), Value::String(goal));
        }

        if self.clear_start_date {
            body.insert("startDate".into(), Value::String(String::new()));
        } else if let Some(value) = self.start_date.as_deref() {
            body.insert(
                "startDate".into(),
                Value::String(ymd_to_jira_datetime(&validate_ymd_date(
                    value,
                    "sprint start date",
                )?)?),
            );
        }

        if self.clear_end_date {
            body.insert("endDate".into(), Value::String(String::new()));
        } else if let Some(value) = self.end_date.as_deref() {
            body.insert(
                "endDate".into(),
                Value::String(ymd_to_jira_datetime(&validate_ymd_date(
                    value,
                    "sprint end date",
                )?)?),
            );
        }

        Ok(Value::Object(body))
    }
}

async fn view_project_versions(
    client: JiraClient,
    project: Option<String>,
    version: Option<String>,
    limit: u32,
    update: ProjectVersionUpdateArgs,
    json: bool,
) -> Result<()> {
    let project_key = project
        .context("Project key is required. Pass --project or configure a default project.")?;
    let mut versions = client.get_project_versions(&project_key).await?;
    versions.sort_by_key(|version| {
        (
            if version.archived {
                2
            } else if version.released {
                1
            } else {
                0
            },
            version.release_date.clone().unwrap_or_default(),
            version.name.to_lowercase(),
        )
    });

    if update.has_changes() {
        let version_name = version.clone().ok_or_else(|| {
            anyhow::anyhow!(
                "Version create/update actions require --version \"<name>\" so jirac knows which fix version to create or modify"
            )
        })?;

        if update.create {
            let request = update.to_create_request(&project_key, &version_name)?;
            let created = client.create_project_version(&request).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&created)?);
                return Ok(());
            }
            print_project_version_metadata(&project_key, &created, Some("✓ Created fix version"));
            return Ok(());
        }

        let target = versions
            .iter()
            .find(|item| item.name == version_name)
            .or_else(|| {
                versions
                    .iter()
                    .find(|item| item.name.eq_ignore_ascii_case(&version_name))
            })
            .cloned()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Fix version '{}' not found in project {}",
                    version_name,
                    project_key
                )
            })?;
        let request = update.to_request()?;
        let updated = client.update_project_version(&target.id, &request).await?;

        if json {
            println!("{}", serde_json::to_string_pretty(&updated)?);
            return Ok(());
        }

        print_project_version_metadata(&project_key, &updated, Some("✓ Updated fix version"));
        return Ok(());
    }

    if let Some(version_name) = version {
        let jql = format!(
            "project = \"{}\" AND fixVersion = \"{}\" AND statusCategory != Done ORDER BY updated DESC",
            project_key.replace('\\', "\\\\").replace('"', "\\\""),
            version_name.replace('\\', "\\\\").replace('"', "\\\""),
        );
        let backlog = client.search_issues(&jql, None, Some(limit)).await?;
        let version_meta = versions
            .iter()
            .find(|item| item.name == version_name)
            .cloned();

        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "project": project_key,
                    "version": version_name,
                    "meta": version_meta,
                    "total_open": backlog.total.unwrap_or(backlog.issues.len() as u64),
                    "issues": backlog.issues,
                }))?
            );
            return Ok(());
        }

        println!("Fix version backlog — {} / {}", project_key, version_name);
        if let Some(meta) = version_meta {
            print_project_version_metadata(&project_key, &meta, None);
        }
        println!(
            "  Open backlog: {}",
            backlog.total.unwrap_or(backlog.issues.len() as u64)
        );
        println!();
        if backlog.issues.is_empty() {
            println!("  ✓ No open backlog items");
        } else {
            for issue in backlog.issues {
                println!("  - {} [{}] {}", issue.key, issue.status, issue.summary);
            }
        }
        return Ok(());
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&versions)?);
        return Ok(());
    }

    println!("Project fix versions — {}", project_key);
    println!();
    for version in &versions {
        let status = if version.archived {
            "archived"
        } else if version.released {
            "released"
        } else {
            "unreleased"
        };
        let mut details = vec![status.to_string()];
        if let Some(date) = version.start_date.as_deref() {
            details.push(format!("start {}", &date[..10.min(date.len())]));
        }
        if let Some(date) = version.release_date.as_deref() {
            details.push(format!("release {}", &date[..10.min(date.len())]));
        }
        println!("  • {} [{}]", version.name, details.join(" | "));
    }
    println!();
    println!(
        "Tip: run `jirac issue versions -p {} --version \"<name>\"` to preview backlog for one fix version.",
        project_key
    );
    println!(
        "Tip: add `--set-name`, `--description`, `--set-start-date YYYY-MM-DD`, `--set-release-date YYYY-MM-DD`, `--released`, or `--archived` with --version to update metadata."
    );
    println!(
        "Tip: add `--create --version \"<name>\"` to create a new fix version in the project."
    );
    Ok(())
}

async fn list_sprints(
    client: JiraClient,
    project: Option<String>,
    state: String,
    json: bool,
) -> Result<()> {
    let project_key = project
        .context("Project key is required. Pass --project or configure a default project.")?;
    let states = parse_sprint_states(&state)?;
    let sprints = client
        .list_sprints_for_project_with_states(&project_key, &states)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&sprints)?);
        return Ok(());
    }

    if sprints.is_empty() {
        println!("No sprints found for {} [{}].", project_key, state);
        return Ok(());
    }

    println!("Project sprints — {}", project_key);
    println!();
    for sprint in &sprints {
        print_sprint_metadata(sprint, true);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn create_sprint(
    client: JiraClient,
    project: Option<String>,
    name: String,
    board_id: Option<u64>,
    goal: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
    json: bool,
) -> Result<()> {
    let project_key = project
        .context("Project key is required. Pass --project or configure a default project.")?;
    let board_id = resolve_board_id_for_project(&client, &project_key, board_id).await?;
    let start_date = start_date
        .as_deref()
        .map(|value| validate_ymd_date(value, "sprint start date"))
        .transpose()?;
    let end_date = end_date
        .as_deref()
        .map(|value| validate_ymd_date(value, "sprint end date"))
        .transpose()?;
    let start_ts = start_date
        .as_deref()
        .map(ymd_to_jira_datetime)
        .transpose()?;
    let end_ts = end_date.as_deref().map(ymd_to_jira_datetime).transpose()?;
    let normalized_goal = normalize_optional_text(goal.as_deref());
    let created = client
        .create_sprint(
            board_id,
            name.trim(),
            start_ts.as_deref(),
            end_ts.as_deref(),
            normalized_goal.as_deref(),
        )
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&created)?);
        return Ok(());
    }

    println!("✓ Created sprint — {} / board {}", project_key, board_id);
    print_sprint_metadata(&created, false);
    Ok(())
}

async fn start_sprint(
    client: JiraClient,
    project: Option<String>,
    sprint: String,
    start_date: Option<String>,
    end_date: String,
    goal: Option<String>,
    json: bool,
) -> Result<()> {
    let project_key = project
        .context("Project key is required. Pass --project or configure a default project.")?;
    let sprint_meta = resolve_sprint_for_project(&client, &project_key, &sprint).await?;
    let start_date = match start_date {
        Some(value) => validate_ymd_date(&value, "sprint start date")?,
        None => Utc::now().date_naive().format("%Y-%m-%d").to_string(),
    };
    let end_date = validate_ymd_date(&end_date, "sprint end date")?;
    let start_ts = ymd_to_jira_datetime(&start_date)?;
    let end_ts = ymd_to_jira_datetime(&end_date)?;
    let mut body = serde_json::json!({
        "state": "active",
        "startDate": start_ts,
        "endDate": end_ts,
    });
    if let Some(goal) =
        normalize_optional_text(goal.as_deref()).or_else(|| sprint_meta.goal.clone())
    {
        body["goal"] = Value::String(goal);
    }
    let updated = client.update_sprint(sprint_meta.id, body).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&updated)?);
        return Ok(());
    }

    println!("✓ Started sprint — {}", project_key);
    print_sprint_metadata(&updated, false);
    Ok(())
}

async fn sprint_complete(
    client: JiraClient,
    project: Option<String>,
    sprint: String,
    complete_date: Option<String>,
    json: bool,
) -> Result<()> {
    let project_key = project
        .context("Project key is required. Pass --project or configure a default project.")?;
    let sprint_meta = resolve_sprint_for_project(&client, &project_key, &sprint).await?;
    let complete_date = match complete_date {
        Some(value) => validate_ymd_date(&value, "sprint complete date")?,
        None => Utc::now().date_naive().format("%Y-%m-%d").to_string(),
    };
    let complete_ts = ymd_to_jira_datetime(&complete_date)?;
    let mut body = serde_json::json!({
        "state": "closed",
        "completeDate": complete_ts.clone(),
    });
    if let Some(end_date) = sprint_meta
        .end_date
        .clone()
        .or_else(|| Some(complete_ts.clone()))
    {
        body["endDate"] = Value::String(end_date);
    }
    let updated = client.update_sprint(sprint_meta.id, body).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&updated)?);
        return Ok(());
    }

    println!("✓ Completed sprint — {}", project_key);
    print_sprint_metadata(&updated, false);
    Ok(())
}

async fn update_sprint_command(
    client: JiraClient,
    project: Option<String>,
    sprint: String,
    update: SprintUpdateArgs,
    json: bool,
) -> Result<()> {
    let project_key = project
        .context("Project key is required. Pass --project or configure a default project.")?;
    if !update.has_changes() {
        anyhow::bail!(
            "Sprint update requires at least one change flag like --name, --goal, --start-date, or --end-date"
        );
    }
    let sprint_meta = resolve_sprint_for_project(&client, &project_key, &sprint).await?;
    let updated = client
        .update_sprint(sprint_meta.id, update.to_request()?)
        .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&updated)?);
        return Ok(());
    }

    println!("✓ Updated sprint — {}", project_key);
    print_sprint_metadata(&updated, false);
    Ok(())
}

async fn sprint_delete(
    client: JiraClient,
    project: Option<String>,
    sprint: String,
    force: bool,
) -> Result<()> {
    let project_key = project
        .context("Project key is required. Pass --project or configure a default project.")?;
    let sprint_meta = resolve_sprint_for_project(&client, &project_key, &sprint).await?;
    if !force {
        let confirmed = Confirm::new(&format!(
            "Delete sprint '{}' (id:{}) in project {}? This cannot be undone.",
            sprint_meta.name, sprint_meta.id, project_key
        ))
        .with_default(false)
        .prompt()
        .context("Sprint delete confirmation aborted")?;
        if !confirmed {
            println!("Canceled sprint deletion.");
            return Ok(());
        }
    }
    client.delete_sprint(sprint_meta.id).await?;
    println!(
        "✓ Deleted sprint — {} / {} (id:{})",
        project_key, sprint_meta.name, sprint_meta.id
    );
    Ok(())
}

fn print_project_version_metadata(
    project_key: &str,
    version: &jira_core::model::ProjectVersion,
    prefix: Option<&str>,
) {
    if let Some(prefix) = prefix {
        println!("{} — {} / {}", prefix, project_key, version.name);
    }

    let status = if version.archived {
        "archived"
    } else if version.released {
        "released"
    } else {
        "unreleased"
    };
    println!("  Status: {status}");
    if let Some(date) = version.start_date.as_deref() {
        println!("  Start: {}", &date[..10.min(date.len())]);
    }
    if let Some(date) = version.release_date.as_deref() {
        println!("  Release: {}", &date[..10.min(date.len())]);
    }
    if let Some(description) = version.description.as_deref() {
        let description = description.trim();
        if !description.is_empty() {
            println!("  Description: {description}");
        }
    }
}

fn validate_ymd_date(value: &str, label: &str) -> Result<String> {
    let value = value.trim();
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| {
        anyhow::anyhow!("Invalid {} '{}'. Expected format: YYYY-MM-DD", label, value)
    })?;
    Ok(value.to_string())
}

fn ymd_to_jira_datetime(value: &str) -> Result<String> {
    let date = NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").map_err(|_| {
        anyhow::anyhow!(
            "Invalid sprint date '{}'. Expected format: YYYY-MM-DD",
            value.trim()
        )
    })?;
    Ok(format!("{}T00:00:00.000Z", date.format("%Y-%m-%d")))
}

fn parse_sprint_states(value: &str) -> Result<Vec<&str>> {
    let mut states = Vec::new();
    for state in value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        match state {
            "active" | "future" | "closed" => states.push(state),
            other => anyhow::bail!(
                "Unsupported sprint state '{}'. Use a comma-separated subset of: active,future,closed",
                other
            ),
        }
    }
    if states.is_empty() {
        anyhow::bail!("At least one sprint state is required")
    }
    Ok(states)
}

fn print_sprint_metadata(sprint: &Sprint, bullet: bool) {
    let prefix = if bullet { "  •" } else { "  " };
    let board = sprint
        .board_id
        .map(|id| format!(" | board {id}"))
        .unwrap_or_default();
    println!(
        "{prefix} {} [id:{} | {}{}]",
        sprint.name, sprint.id, sprint.state, board
    );
    if let Some(goal) = sprint
        .goal
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        println!("    goal: {goal}");
    }
    if let Some(start) = sprint.start_date.as_deref() {
        println!("    start: {}", &start[..10.min(start.len())]);
    }
    if let Some(end) = sprint.end_date.as_deref() {
        println!("    end:   {}", &end[..10.min(end.len())]);
    }
    if let Some(complete) = sprint.complete_date.as_deref() {
        println!("    done:  {}", &complete[..10.min(complete.len())]);
    }
}

async fn resolve_board_id_for_project(
    client: &JiraClient,
    project_key: &str,
    requested_board_id: Option<u64>,
) -> Result<u64> {
    let boards = client
        .raw_request(
            "GET",
            &format!("/rest/agile/1.0/board?projectKeyOrId={project_key}&maxResults=100"),
            None,
        )
        .await
        .context("Failed to list boards for sprint creation")?
        .unwrap_or(Value::Null);

    let board_values = boards
        .get("values")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            anyhow::anyhow!("Unexpected board response while resolving project board")
        })?;

    let boards = board_values
        .iter()
        .filter_map(|board| {
            Some((
                board.get("id").and_then(Value::as_u64)?,
                board
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("Unnamed board")
                    .to_string(),
            ))
        })
        .collect::<Vec<_>>();

    if boards.is_empty() {
        anyhow::bail!("No sprint-enabled boards found for project {}", project_key);
    }

    if let Some(board_id) = requested_board_id {
        if boards.iter().any(|(id, _)| *id == board_id) {
            return Ok(board_id);
        }
        let options = boards
            .iter()
            .map(|(id, name)| format!("{name} ({id})"))
            .collect::<Vec<_>>()
            .join(", ");
        anyhow::bail!(
            "Board {} is not available for project {}. Available boards: {}",
            board_id,
            project_key,
            options
        )
    }

    if boards.len() == 1 {
        return Ok(boards[0].0);
    }

    let options = boards
        .iter()
        .map(|(id, name)| format!("{name} ({id})"))
        .collect::<Vec<_>>()
        .join(", ");
    anyhow::bail!(
        "Project {} maps to multiple boards. Re-run with --board-id. Available boards: {}",
        project_key,
        options
    )
}

async fn resolve_sprint_for_project(
    client: &JiraClient,
    project_key: &str,
    sprint: &str,
) -> Result<Sprint> {
    let sprints = client
        .list_sprints_for_project_with_states(project_key, &["active", "future", "closed"])
        .await
        .context("Failed to list project sprints")?;

    if let Ok(id) = sprint.trim().parse::<u64>() {
        return sprints
            .into_iter()
            .find(|item| item.id == id)
            .ok_or_else(|| {
                anyhow::anyhow!("Sprint id {} was not found in project {}", id, project_key)
            });
    }

    let matches = sprints
        .into_iter()
        .filter(|item| item.name.eq_ignore_ascii_case(sprint.trim()))
        .collect::<Vec<_>>();

    match matches.len() {
        0 => anyhow::bail!(
            "Sprint '{}' was not found on any sprint-enabled board for project {}",
            sprint,
            project_key
        ),
        1 => Ok(matches.into_iter().next().expect("single sprint match")),
        _ => {
            let options = matches
                .iter()
                .map(|item| {
                    format!(
                        "{} (id:{}, board:{})",
                        item.name,
                        item.id,
                        item.board_id.unwrap_or_default()
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            anyhow::bail!(
                "Sprint '{}' matched multiple sprints. Use a numeric sprint ID instead: {}",
                sprint,
                options
            )
        }
    }
}

fn normalize_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
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
    sprint: Option<String>,
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

    if let Some(sprint) = sprint {
        let (field_id, field_value) =
            resolve_sprint_assignment(&client, &project_key, &issue_type_id, &sprint).await?;
        custom_fields.insert(field_id, field_value);
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
            match client.get_issue(&issue.key).await {
                Ok(refreshed) => refreshed,
                Err(e) => {
                    eprintln!(
                        "warning: re-fetch after attach failed ({e}); attachment metadata may be missing"
                    );
                    issue
                }
            }
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

    let cache = FieldCache::new();
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
            .find(|t| t.id == name_or_id || t.name == name_or_id)
            .map(|t| t.id.clone())
            .ok_or_else(|| anyhow::anyhow!("Transition '{}' not found", name_or_id))?
    } else {
        let options: Vec<String> = transitions
            .iter()
            .map(|t| format!("{} [{}]", t.name, t.id))
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

// ─── attachment ──────────────────────────────────────────────────────────────

async fn attachment(client: JiraClient, cmd: AttachmentCommand) -> Result<()> {
    match cmd {
        AttachmentCommand::List { key, json } => attachment_list(client, key, json).await,
        AttachmentCommand::Download {
            id,
            out,
            filename,
            force,
        } => attachment_download(client, id, out, filename, force).await,
        AttachmentCommand::Delete { id, force } => attachment_delete(client, id, force).await,
    }
}

async fn attachment_list(client: JiraClient, key: String, json: bool) -> Result<()> {
    let spinner = spinner_new(format!("Fetching attachments for {key}..."));
    let attachments = client
        .list_attachments(&key)
        .await
        .with_context(|| format!("Failed to list attachments for {key}"))?;
    spinner.finish_and_clear();

    if json {
        println!("{}", serde_json::to_string_pretty(&attachments)?);
        return Ok(());
    }

    if attachments.is_empty() {
        println!("No attachments on {key}.");
        return Ok(());
    }

    for a in &attachments {
        println!(
            "{:<10} {:<10} {:>10}  {}",
            a.id, a.mime_type, a.size, a.filename
        );
    }
    Ok(())
}

async fn attachment_download(
    client: JiraClient,
    id: String,
    out: Option<std::path::PathBuf>,
    filename: Option<String>,
    force: bool,
) -> Result<()> {
    let spinner = spinner_new(format!("Downloading attachment {id}..."));
    let (server_name, bytes, mime) = client
        .download_attachment(&id)
        .await
        .with_context(|| format!("Failed to download attachment {id}"))?;
    spinner.finish_and_clear();

    let out_dir = out.unwrap_or_else(|| std::path::PathBuf::from("."));
    if !out_dir.exists() {
        std::fs::create_dir_all(&out_dir)
            .with_context(|| format!("Failed to create {}", out_dir.display()))?;
    }
    let name = filename.unwrap_or(server_name);
    let dest = out_dir.join(&name);
    if dest.exists() && !force {
        anyhow::bail!(
            "{} already exists. Use --force to overwrite.",
            dest.display()
        );
    }
    std::fs::write(&dest, &bytes).with_context(|| format!("Failed to write {}", dest.display()))?;
    println!(
        "✓ Saved {} ({} bytes, {})",
        dest.display(),
        bytes.len(),
        mime
    );
    Ok(())
}

async fn attachment_delete(client: JiraClient, id: String, force: bool) -> Result<()> {
    if !force {
        let ok = Confirm::new(&format!("Delete attachment {id}?"))
            .with_default(false)
            .prompt()
            .context("Failed to read confirmation")?;
        if !ok {
            println!("Aborted.");
            return Ok(());
        }
    }
    let spinner = spinner_new(format!("Deleting attachment {id}..."));
    client
        .delete_attachment(&id)
        .await
        .with_context(|| format!("Failed to delete attachment {id}"))?;
    spinner.finish_and_clear();
    println!("✓ Deleted attachment {id}");
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

fn render_issue_content(
    input: Option<std::path::PathBuf>,
    format: String,
    output: String,
) -> Result<()> {
    let content = read_render_input(input.as_deref())?;
    let format = normalize_render_format(&format)?;
    let output = normalize_render_output(&output)?;

    let adf = match format {
        "markdown" => jira_core::adf::markdown_to_adf(&content),
        "text" => jira_core::adf::plain_text_to_adf(&content),
        "adf" => serde_json::from_str::<Value>(&content)
            .context("--format adf requires valid JSON ADF content")?,
        _ => unreachable!(),
    };

    match output {
        "adf" => println!("{}", serde_json::to_string_pretty(&adf)?),
        "text" => println!("{}", jira_core::adf::adf_to_text(&adf)),
        _ => unreachable!(),
    }

    Ok(())
}

// ─── helpers ─────────────────────────────────────────────────────────────────

fn read_render_input(path: Option<&std::path::Path>) -> Result<String> {
    match path {
        Some(path) => std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read input file: {}", path.display())),
        None => {
            use std::io::Read;

            let mut input = String::new();
            std::io::stdin()
                .read_to_string(&mut input)
                .context("Failed to read stdin")?;
            Ok(input)
        }
    }
}

fn normalize_render_format(value: &str) -> Result<&str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "markdown" | "md" => Ok("markdown"),
        "text" | "txt" => Ok("text"),
        "adf" | "json" => Ok("adf"),
        other => {
            anyhow::bail!("Unsupported input format '{other}'. Use one of: markdown, text, adf")
        }
    }
}

fn normalize_render_output(value: &str) -> Result<&str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "adf" | "json" => Ok("adf"),
        "text" | "txt" => Ok("text"),
        other => anyhow::bail!("Unsupported output format '{other}'. Use one of: adf, text"),
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len.saturating_sub(1)])
    }
}

async fn resolve_sprint_assignment(
    client: &JiraClient,
    project_key: &str,
    issue_type_id: &str,
    sprint: &str,
) -> Result<(String, FieldValue)> {
    if issue_type_id.is_empty() {
        anyhow::bail!(
            "Sprint assignment requires a resolved issue type so Jira fields can be inspected"
        );
    }

    let fields = client
        .get_fields_for_issue_type(project_key, issue_type_id)
        .await
        .context("Failed to inspect fields for sprint assignment")?;

    let sprint_field = fields
        .into_iter()
        .find(|field| {
            field.name.eq_ignore_ascii_case("Sprint")
                || field
                    .schema
                    .as_ref()
                    .and_then(|schema| schema.get("custom"))
                    .and_then(|value| value.as_str())
                    .map(|custom| custom.contains("gh-sprint"))
                    .unwrap_or(false)
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Sprint is not available for project {} / this issue type on create",
                project_key
            )
        })?;

    let sprint_id = if let Ok(id) = sprint.trim().parse::<u64>() {
        id
    } else {
        resolve_sprint_id_by_name(client, project_key, sprint).await?
    };

    Ok((
        sprint_field.id,
        FieldValue::Raw(serde_json::json!([{ "id": sprint_id }])),
    ))
}

async fn resolve_sprint_id_by_name(
    client: &JiraClient,
    project_key: &str,
    sprint_name: &str,
) -> Result<u64> {
    let boards = client
        .raw_request(
            "GET",
            &format!("/rest/agile/1.0/board?projectKeyOrId={project_key}&maxResults=100"),
            None,
        )
        .await
        .context("Failed to list boards for sprint resolution")?
        .unwrap_or(Value::Null);

    let board_values = boards
        .get("values")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("Unexpected board response while resolving sprint"))?;

    let mut matches = Vec::new();

    for board in board_values {
        let board_id = match board.get("id").and_then(Value::as_u64) {
            Some(id) => id,
            None => continue,
        };

        let response = client
            .raw_request(
                "GET",
                &format!(
                    "/rest/agile/1.0/board/{board_id}/sprint?state=active,future,closed&maxResults=100"
                ),
                None,
            )
            .await;

        let Ok(Some(payload)) = response else {
            continue;
        };

        if let Some(values) = payload.get("values").and_then(Value::as_array) {
            for sprint in values {
                let Some(name) = sprint.get("name").and_then(Value::as_str) else {
                    continue;
                };
                if name.eq_ignore_ascii_case(sprint_name) {
                    if let Some(id) = sprint.get("id").and_then(Value::as_u64) {
                        matches.push((id, board_id, name.to_string()));
                    }
                }
            }
        }
    }

    match matches.len() {
        0 => anyhow::bail!(
            "Sprint '{}' was not found on any sprint-enabled board for project {}",
            sprint_name,
            project_key
        ),
        1 => Ok(matches[0].0),
        _ => {
            let options = matches
                .into_iter()
                .map(|(id, board_id, name)| format!("{name} (id:{id}, board:{board_id})"))
                .collect::<Vec<_>>()
                .join(", ");
            anyhow::bail!(
                "Sprint '{}' matched multiple sprints. Use a numeric sprint ID instead: {}",
                sprint_name,
                options
            )
        }
    }
}

// ─── watch ───────────────────────────────────────────────────────────────────

async fn watch(client: JiraClient, key: String, cmd: WatchCommand) -> Result<()> {
    match cmd {
        WatchCommand::Add { account_id } => watch_add(client, key, account_id).await,
        WatchCommand::List => watch_list(client, key).await,
        WatchCommand::Rm { account_id, force } => watch_rm(client, key, account_id, force).await,
    }
}

async fn watch_add(client: JiraClient, key: String, account_id: Option<String>) -> Result<()> {
    let target = match account_id {
        Some(id) => id,
        None => {
            let spinner = spinner_new("Resolving current user...".to_string());
            let me = client
                .get_myself()
                .await
                .context("Failed to resolve current user")?;
            spinner.finish_and_clear();
            me
        }
    };

    let spinner = spinner_new(format!("Adding watcher to {key}..."));
    client
        .add_watcher(&key, &target)
        .await
        .context("Failed to add watcher")?;
    spinner.finish_and_clear();

    println!("✓ Added watcher {target} to {key}");
    Ok(())
}

async fn watch_list(client: JiraClient, key: String) -> Result<()> {
    let spinner = spinner_new(format!("Fetching watchers for {key}..."));
    let watchers = client
        .list_watchers(&key)
        .await
        .context("Failed to fetch watchers")?;
    spinner.finish_and_clear();

    if watchers.watchers.is_empty() {
        println!("No watchers on {key}.");
        return Ok(());
    }

    println!("Watchers on {} ({} total):", key, watchers.watch_count);
    for w in &watchers.watchers {
        let status = if w.active { "active" } else { "inactive" };
        println!("  - {} ({}) [{}]", w.display_name, w.account_id, status);
    }
    if watchers.is_watching {
        println!("\nYou are currently watching this issue.");
    }
    Ok(())
}

async fn watch_rm(client: JiraClient, key: String, account_id: String, force: bool) -> Result<()> {
    if !force {
        let confirm = inquire::Confirm::new(&format!("Remove watcher {account_id} from {key}?"))
            .with_default(false)
            .prompt()
            .context("Failed to read confirmation")?;
        if !confirm {
            println!("Aborted.");
            return Ok(());
        }
    }

    let spinner = spinner_new(format!("Removing watcher from {key}..."));
    client
        .remove_watcher(&key, &account_id)
        .await
        .context("Failed to remove watcher")?;
    spinner.finish_and_clear();

    println!("✓ Removed watcher {account_id} from {key}");
    Ok(())
}

// ─── comment ─────────────────────────────────────────────────────────────────

async fn comment(client: JiraClient, key: String, cmd: CommentCommand) -> Result<()> {
    match cmd {
        CommentCommand::List => comment_list(client, key).await,
        CommentCommand::Add { body, file } => comment_add(client, key, body, file).await,
    }
}

async fn comment_list(client: JiraClient, key: String) -> Result<()> {
    let spinner = spinner_new(format!("Fetching comments for {key}..."));
    let comments = client
        .get_comments(&key)
        .await
        .context("Failed to fetch comments")?;
    spinner.finish_and_clear();

    if comments.is_empty() {
        println!("No comments found for {key}.");
        return Ok(());
    }

    for c in comments {
        println!("#{}", c.id);
        if let Some(author) = &c.author {
            println!("  Author : {}", author);
        }
        if !c.created.is_empty() {
            println!("  Created: {}", c.created);
        }
        if let Some(body) = &c.body {
            println!("  Body   : {}", body.replace('\n', "\n           "));
        }
        println!();
    }

    Ok(())
}

async fn comment_add(
    client: JiraClient,
    key: String,
    body: Option<String>,
    file: Option<std::path::PathBuf>,
) -> Result<()> {
    let comment_body = read_comment_body(body, file)?;

    let spinner = spinner_new(format!("Adding comment to {key}..."));
    let comment = client
        .add_comment(&key, &comment_body)
        .await
        .context("Failed to add comment")?;
    spinner.finish_and_clear();

    println!("✓ Added comment {} to {}", comment.id, key);
    Ok(())
}

fn read_comment_body(body: Option<String>, file: Option<std::path::PathBuf>) -> Result<String> {
    let comment_body = match (body, file) {
        (Some(body), None) if !body.trim().is_empty() => body,
        (None, Some(path)) => std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read comment file {}", path.display()))?,
        _ => anyhow::bail!("Provide exactly one of --body or --file with non-empty content"),
    };

    if comment_body.trim().is_empty() {
        anyhow::bail!("Comment cannot be empty");
    }

    Ok(comment_body)
}

fn normalize_issue_keys(raw_keys: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for raw in raw_keys {
        for key in raw
            .split(|c: char| c == ',' || c.is_whitespace())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            if !out.iter().any(|existing| existing == key) {
                out.push(key.to_string());
            }
        }
    }
    out
}

async fn bulk_comment(
    client: JiraClient,
    jql: Option<String>,
    keys: Vec<String>,
    body: Option<String>,
    file: Option<std::path::PathBuf>,
    force: bool,
    json: bool,
) -> Result<()> {
    let comment_body = read_comment_body(body, file)?;

    let target_keys = if let Some(jql) = jql.as_deref() {
        let spinner = spinner_new("Fetching issues...");
        let issues = client
            .get_all_issues(jql)
            .await
            .context("Failed to fetch issues")?;
        spinner.finish_and_clear();
        issues
            .into_iter()
            .map(|issue| issue.key)
            .collect::<Vec<_>>()
    } else {
        normalize_issue_keys(keys)
    };

    if target_keys.is_empty() {
        if jql.is_some() {
            println!("No issues found matching JQL.");
            return Ok(());
        }
        anyhow::bail!("Provide --jql or at least one issue key via --keys.");
    }

    println!("Found {} issue(s).", target_keys.len());

    if !force {
        let target_label = if jql.is_some() {
            "matched issues"
        } else {
            "explicit issue(s)"
        };
        let confirm = inquire::Confirm::new(&format!(
            "Add this comment to {} {}?",
            target_keys.len(),
            target_label
        ))
        .with_default(false)
        .prompt()
        .context("Failed to read confirmation")?;
        if !confirm {
            println!("Aborted.");
            return Ok(());
        }
    }

    let pb = progress_bar(target_keys.len() as u64);
    let mut ok = 0u64;
    let mut failed: Vec<String> = Vec::new();

    for key in &target_keys {
        pb.set_message(key.clone());
        match client.add_comment(key, &comment_body).await {
            Ok(_) => ok += 1,
            Err(e) => failed.push(format!("{}: {}", key, e)),
        }
        pb.inc(1);
    }

    pb.finish_and_clear();

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "total": target_keys.len(),
                "succeeded": ok,
                "failed_count": failed.len(),
                "failed": failed,
                "targets": target_keys,
            }))?
        );
    } else {
        println!("✓ Added comment to {ok}/{} issues", target_keys.len());
        if !failed.is_empty() {
            println!("✗ Failed ({}):", failed.len());
            for item in &failed {
                println!("  {item}");
            }
        }
    }

    Ok(())
}

// ─── worklog ─────────────────────────────────────────────────────────────────

struct WorklogAddOptions {
    time: String,
    comment: Option<String>,
    date: Option<String>,
    start: Option<String>,
    range: Option<WorklogRangeOptions>,
}

struct WorklogRangeOptions {
    from: String,
    to: String,
    exclude_weekends: bool,
}

async fn worklog(client: JiraClient, key: String, cmd: WorklogCommand) -> Result<()> {
    match cmd {
        WorklogCommand::List => worklog_list(client, key).await,
        WorklogCommand::Add {
            time,
            comment,
            date,
            start,
            from,
            to,
            exclude_weekends,
        } => {
            let options = WorklogAddOptions {
                time,
                comment,
                date,
                start,
                range: match (from, to) {
                    (Some(from), Some(to)) => Some(WorklogRangeOptions {
                        from,
                        to,
                        exclude_weekends,
                    }),
                    _ => None,
                },
            };
            worklog_add(client, key, options).await
        }
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

async fn worklog_add(client: JiraClient, key: String, options: WorklogAddOptions) -> Result<()> {
    let WorklogAddOptions {
        time,
        comment,
        date,
        start,
        range,
    } = options;

    let jira_timezone = if date.is_some() || start.is_some() || range.is_some() {
        client
            .get_myself_timezone()
            .await
            .context("Failed to fetch Jira user timezone")?
    } else {
        None
    };

    if let Some(range) = range {
        return worklog_add_range(client, key, time, comment, start, range, jira_timezone).await;
    }

    let started =
        build_worklog_started(date.as_deref(), start.as_deref(), jira_timezone.as_deref())?;

    let spinner = spinner_new(format!("Logging {time} on {key}..."));
    let log = client
        .add_worklog(&key, &time, comment.as_deref(), started.as_deref())
        .await
        .context("Failed to add worklog")?;
    spinner.finish_and_clear();
    println!(
        "✓ Logged {} on {} (worklog id: {})",
        log.time_spent, key, log.id
    );
    Ok(())
}

async fn worklog_add_range(
    client: JiraClient,
    key: String,
    time: String,
    comment: Option<String>,
    start: Option<String>,
    range: WorklogRangeOptions,
    jira_timezone: Option<String>,
) -> Result<()> {
    let WorklogRangeOptions {
        from,
        to,
        exclude_weekends,
    } = range;

    let dates = build_worklog_range_dates(&from, &to, exclude_weekends)?;

    if dates.is_empty() {
        anyhow::bail!(
            "No worklog dates remain in range {}..{} after applying weekend filtering.",
            from,
            to
        );
    }

    let pb = progress_bar(dates.len() as u64);
    let mut created = Vec::with_capacity(dates.len());

    for date in dates {
        let date_label = date.format("%Y-%m-%d").to_string();
        pb.set_message(format!("{} ({})", key, date_label));

        let started =
            build_worklog_started_for_date(date, start.as_deref(), jira_timezone.as_deref())?;
        match client
            .add_worklog(&key, &time, comment.as_deref(), Some(&started))
            .await
        {
            Ok(log) => {
                created.push((date_label, log.id));
                pb.inc(1);
            }
            Err(err) => {
                pb.finish_and_clear();
                let partial = if created.is_empty() {
                    String::new()
                } else {
                    format!(
                        " Partial success: {}.",
                        created
                            .iter()
                            .map(|(date, id)| format!("{} -> {}", date, id))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };

                anyhow::bail!(
                    "Failed to add worklog for {} on {}: {}.{}",
                    key,
                    date_label,
                    err,
                    partial
                );
            }
        }
    }

    pb.finish_and_clear();

    println!(
        "✓ Logged {} on {} across {} day(s){}",
        time,
        key,
        created.len(),
        if exclude_weekends {
            " (excluding weekends)"
        } else {
            ""
        }
    );
    for (date, id) in created {
        println!("  - {} -> worklog id {}", date, id);
    }

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
        .find(|t| t.id == to || t.name.eq_ignore_ascii_case(&to))
        .map(|t| t.id.clone())
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

fn jql_params_filters_empty(p: &jira_core::jql::JqlParams) -> bool {
    p.project.is_none()
        && p.status.is_empty()
        && p.assignee.is_empty()
        && p.priority.is_empty()
        && p.labels.is_empty()
        && p.components.is_empty()
        && p.fix_versions.is_empty()
        && p.text.is_none()
        && p.created_after.is_none()
        && p.updated_after.is_none()
        && p.extra_clauses.is_empty()
}

fn load_jql_params(spec: &str) -> Result<jira_core::jql::JqlParams> {
    let raw = if let Some(path) = spec.strip_prefix('@') {
        std::fs::read_to_string(path).with_context(|| format!("Failed to read {path}"))?
    } else {
        spec.to_string()
    };
    serde_json::from_str(&raw).context("Failed to parse JqlParams JSON")
}

fn prompt_jql_params() -> Result<jira_core::jql::JqlParams> {
    use jira_core::jql::{AssigneeFilter, JqlParams, OrderDir};

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
        Vec::new()
    } else {
        vec![status_sel.to_string()]
    };

    let assignee_opts = vec!["Me (currentUser)", "Unassigned", "Custom email", "(any)"];
    let assignee_sel = Select::new("Assignee:", assignee_opts)
        .prompt()
        .context("Failed to read assignee")?;
    let assignee = match assignee_sel {
        "Me (currentUser)" => vec![AssigneeFilter::CurrentUser],
        "Unassigned" => vec![AssigneeFilter::Empty],
        "Custom email" => {
            let email = Text::new("Email:")
                .prompt()
                .context("Failed to read email")?;
            vec![AssigneeFilter::Email { email }]
        }
        _ => Vec::new(),
    };

    let priority_opts = vec!["Highest", "High", "Medium", "Low", "Lowest", "(any)"];
    let priority_sel = Select::new("Priority:", priority_opts)
        .prompt()
        .context("Failed to read priority")?;
    let priority = if priority_sel == "(any)" {
        Vec::new()
    } else {
        vec![priority_sel.to_string()]
    };

    let order_opts = vec!["updated DESC", "created DESC", "priority DESC", "key ASC"];
    let order_sel = Select::new("Order by:", order_opts)
        .prompt()
        .context("Failed to read order")?;
    let order_by = vec![match order_sel {
        "updated DESC" => ("updated".to_string(), OrderDir::Desc),
        "created DESC" => ("created".to_string(), OrderDir::Desc),
        "priority DESC" => ("priority".to_string(), OrderDir::Desc),
        _ => ("key".to_string(), OrderDir::Asc),
    }];

    Ok(JqlParams {
        project,
        status,
        assignee,
        priority,
        order_by,
        ..Default::default()
    })
}

async fn jql_builder(client: JiraClient, run: bool, params: Option<String>) -> Result<()> {
    let mut jql_params = if let Some(spec) = params {
        load_jql_params(&spec)?
    } else {
        prompt_jql_params()?
    };

    if jql_params_filters_empty(&jql_params) {
        jql_params
            .assignee
            .push(jira_core::jql::AssigneeFilter::CurrentUser);
    }
    if jql_params.order_by.is_empty() {
        jql_params
            .order_by
            .push(("updated".into(), jira_core::jql::OrderDir::Desc));
    }

    let jql = jira_core::jql::compose_jql(&jql_params).context("Failed to compose JQL")?;
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
                        .find(|t| t.id == to || t.name.eq_ignore_ascii_case(&to))
                        .map(|t| t.id.clone())
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

// ─── native move / type change ───────────────────────────────────────────────

async fn change_issue_type(
    client: JiraClient,
    key: String,
    issue_type: String,
    json: bool,
) -> Result<()> {
    let spinner = spinner_new(format!("Fetching {key}..."));
    let source = client
        .get_issue(&key)
        .await
        .context("Failed to fetch source issue")?;
    spinner.finish_and_clear();

    let target_issue_type = client
        .get_issue_type_by_name(&source.project_key, &issue_type)
        .await
        .with_context(|| {
            format!(
                "Failed to resolve issue type '{}' in project {}",
                issue_type, source.project_key
            )
        })?;

    let spinner = spinner_new(format!("Changing issue type for {key}..."));
    let moved = client
        .move_issue(&key, &source.project_key, &target_issue_type.id, None)
        .await
        .context("Failed to change issue type")?;
    spinner.finish_and_clear();

    if json {
        println!("{}", serde_json::to_string_pretty(&moved)?);
    } else {
        println!(
            "✓ Changed issue type: {} → {} ({})",
            key, moved.key, moved.issue_type
        );
    }

    Ok(())
}

async fn move_issue_native(
    client: JiraClient,
    key: String,
    project: String,
    issue_type: Option<String>,
    json: bool,
) -> Result<()> {
    let spinner = spinner_new(format!("Fetching {key}..."));
    let source = client
        .get_issue(&key)
        .await
        .context("Failed to fetch source issue")?;
    spinner.finish_and_clear();

    let target_issue_type_name = issue_type.unwrap_or_else(|| source.issue_type.clone());
    let target_issue_type = client
        .get_issue_type_by_name(&project, &target_issue_type_name)
        .await
        .with_context(|| {
            format!(
                "Failed to resolve issue type '{}' in project {}",
                target_issue_type_name, project
            )
        })?;

    let spinner = spinner_new(format!("Moving {key} to {project}..."));
    let moved = client
        .move_issue(&key, &project, &target_issue_type.id, None)
        .await
        .context("Failed to move issue")?;
    spinner.finish_and_clear();

    if json {
        println!("{}", serde_json::to_string_pretty(&moved)?);
    } else {
        println!(
            "✓ Moved natively: {} → {} ({})",
            key, moved.key, moved.project_key
        );
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

async fn handle_link_command(client: JiraClient, cmd: LinkCommand) -> Result<()> {
    match cmd {
        LinkCommand::ListTypes { json } => {
            let types = client.list_issue_link_types().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&types)?);
            } else {
                println!(
                    "{:<10} {:<15} {:<20} {:<20}",
                    "ID", "Name", "Inward", "Outward"
                );
                println!("{}", "-".repeat(65));
                for t in types {
                    println!(
                        "{:<10} {:<15} {:<20} {:<20}",
                        t.id, t.name, t.inward, t.outward
                    );
                }
            }
        }
        LinkCommand::Add {
            outward,
            inward,
            link_type,
            comment,
        } => {
            client
                .link_issues(&outward, &inward, &link_type, comment.as_deref())
                .await?;
            println!("✓ Linked {outward} to {inward} as '{link_type}'");
        }
        LinkCommand::Delete { id, force } => {
            if !force {
                let confirmed = inquire::Confirm::new(&format!("Delete issue link {id}?"))
                    .with_default(false)
                    .prompt()?;
                if !confirmed {
                    println!("Aborted.");
                    return Ok(());
                }
            }
            client.delete_issue_link(&id).await?;
            println!("✓ Deleted issue link {id}");
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

fn issue_status_category(issue: &Issue) -> String {
    issue
        .fields
        .get("status")
        .and_then(|status| status.get("statusCategory"))
        .and_then(|category| category.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_lowercase()
}

fn issue_is_blocked(issue: &Issue) -> bool {
    let status = issue.status.to_lowercase();
    status.contains("blocked") || status.contains("on hold") || status.contains("stuck")
}

fn parse_relative_window(raw: &str) -> Result<Duration> {
    let value = raw.trim().to_lowercase();
    if value.len() < 2 {
        anyhow::bail!("Invalid relative window '{raw}'. Use values like 2d, 36h, or 1w.");
    }

    let (num, unit) = value.split_at(value.len() - 1);
    let amount: i64 = num.parse().with_context(|| {
        format!("Invalid relative window '{raw}'. Use values like 2d, 36h, or 1w.")
    })?;

    match unit {
        "h" => Ok(Duration::hours(amount)),
        "d" => Ok(Duration::days(amount)),
        "w" => Ok(Duration::weeks(amount)),
        _ => anyhow::bail!("Invalid relative window '{raw}'. Use values like 2d, 36h, or 1w."),
    }
}

fn issue_updated_at(issue: &Issue) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&issue.updated)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn format_issue_line(issue: &Issue) -> String {
    format!("- {} [{}] {}", issue.key, issue.status, issue.summary)
}

fn escape_jql_literal(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

async fn standup_summary(
    client: JiraClient,
    project: Option<String>,
    jql: Option<String>,
    since: String,
    limit: u32,
    json: bool,
) -> Result<()> {
    let cutoff = Utc::now() - parse_relative_window(&since)?;
    let query = if let Some(jql) = jql {
        jql
    } else if let Some(project) = project {
        format!("project = {project} AND assignee = currentUser() ORDER BY updated DESC")
    } else {
        "assignee = currentUser() ORDER BY updated DESC".to_string()
    };

    let spinner = spinner_new("Generating standup summary...");
    let issues = client
        .search_issues(&query, None, Some(limit.min(100)))
        .await
        .context("Failed to fetch issues for standup summary")?
        .issues;
    spinner.finish_and_clear();

    let mut done = vec![];
    let mut in_progress = vec![];
    let mut next_up = vec![];
    let mut blocked = vec![];
    let mut other = vec![];

    for issue in issues {
        let category = issue_status_category(&issue);
        let is_done_recent = category == "done"
            && issue_updated_at(&issue)
                .map(|updated| updated >= cutoff)
                .unwrap_or(false);

        if issue_is_blocked(&issue) {
            blocked.push(issue);
        } else if is_done_recent {
            done.push(issue);
        } else if category == "indeterminate" {
            in_progress.push(issue);
        } else if category == "new" {
            next_up.push(issue);
        } else {
            other.push(issue);
        }
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "query": query,
                "since": since,
                "recently_done": done,
                "in_progress": in_progress,
                "next_up": next_up,
                "blocked": blocked,
                "other": other,
            }))?
        );
        return Ok(());
    }

    println!("# Daily standup");
    println!();
    println!("Source: `{}`", query);
    println!("Recently done window: {}", since);
    println!();

    for (title, items) in [
        ("Recently done", &done),
        ("In progress", &in_progress),
        ("Next up", &next_up),
        ("Blocked", &blocked),
        ("Other", &other),
    ] {
        if items.is_empty() {
            continue;
        }
        println!("## {} ({})", title, items.len());
        for issue in items {
            println!("{}", format_issue_line(issue));
        }
        println!();
    }

    if done.is_empty()
        && in_progress.is_empty()
        && next_up.is_empty()
        && blocked.is_empty()
        && other.is_empty()
    {
        println!("No issues matched the standup query.");
    }

    Ok(())
}

async fn sprint_summary(
    client: JiraClient,
    project: Option<String>,
    sprint: Option<String>,
    limit: u32,
    json: bool,
) -> Result<()> {
    let project =
        project.context("Project is required. Pass --project or configure a default project.")?;
    let sprint_label = sprint
        .clone()
        .unwrap_or_else(|| "openSprints()".to_string());
    let sprint_clause = match sprint {
        Some(value) if value.trim().parse::<u64>().is_ok() => format!("sprint = {}", value.trim()),
        Some(value) => format!("sprint = \"{}\"", escape_jql_literal(value.trim())),
        None => "sprint in openSprints()".to_string(),
    };
    let query = format!(
        "project = {} AND {} ORDER BY status ASC, updated DESC",
        project, sprint_clause
    );

    let spinner = spinner_new("Generating sprint summary...");
    let issues = client
        .search_issues(&query, None, Some(limit.min(100)))
        .await
        .context("Failed to fetch issues for sprint summary")?
        .issues;
    spinner.finish_and_clear();

    let mut by_status: HashMap<String, Vec<Issue>> = HashMap::new();
    let mut by_assignee: HashMap<String, usize> = HashMap::new();
    let mut done_count = 0usize;
    let mut in_progress_count = 0usize;
    let mut todo_count = 0usize;
    let mut blocked_count = 0usize;

    for issue in issues {
        let category = issue_status_category(&issue);
        if issue_is_blocked(&issue) {
            blocked_count += 1;
        }
        match category.as_str() {
            "done" => done_count += 1,
            "indeterminate" => in_progress_count += 1,
            "new" => todo_count += 1,
            _ => {}
        }
        let assignee = issue
            .assignee
            .clone()
            .unwrap_or_else(|| "Unassigned".to_string());
        *by_assignee.entry(assignee).or_insert(0) += 1;
        by_status
            .entry(issue.status.clone())
            .or_default()
            .push(issue);
    }

    let total: usize = by_status.values().map(Vec::len).sum();

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "project": project,
                "sprint": sprint_label,
                "query": query,
                "total": total,
                "done": done_count,
                "in_progress": in_progress_count,
                "todo": todo_count,
                "blocked": blocked_count,
                "by_assignee": by_assignee,
                "by_status": by_status,
            }))?
        );
        return Ok(());
    }

    println!("# Sprint summary — {}", project);
    println!();
    println!("Sprint: {}", sprint_label);
    println!("Source: `{}`", query);
    println!();
    println!("- total issues: {}", total);
    println!("- done: {}", done_count);
    println!("- in progress: {}", in_progress_count);
    println!("- to do: {}", todo_count);
    println!("- blocked: {}", blocked_count);
    println!();

    let mut assignees = by_assignee.into_iter().collect::<Vec<_>>();
    assignees.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    if !assignees.is_empty() {
        println!("## By assignee");
        for (assignee, count) in assignees {
            println!("- {}: {}", assignee, count);
        }
        println!();
    }

    let mut statuses = by_status.into_iter().collect::<Vec<_>>();
    statuses.sort_by(|a, b| a.0.cmp(&b.0));
    for (status, issues) in statuses {
        println!("## {} ({})", status, issues.len());
        for issue in issues {
            println!("{}", format_issue_line(&issue));
        }
        println!();
    }

    if total == 0 {
        println!("No issues matched the sprint query.");
    }

    Ok(())
}
