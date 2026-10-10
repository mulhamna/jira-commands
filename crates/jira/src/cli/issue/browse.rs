use super::manage::truncate;
use super::*;

pub(super) async fn handle_command(
    client: JiraClient,
    command: IssueCommand,
    default_project: Option<String>,
    default_issue_limit: Option<u32>,
) -> Result<()> {
    match command {
        command @ (IssueCommand::List { .. }
        | IssueCommand::Export { .. }
        | IssueCommand::Standup { .. }
        | IssueCommand::SprintSummary { .. }
        | IssueCommand::Notifications { .. }
        | IssueCommand::View { .. }
        | IssueCommand::Versions { .. }) => {
            handle_query_command(client, command, default_project, default_issue_limit).await
        }
        command @ (IssueCommand::Sprints { .. }
        | IssueCommand::SprintCreate { .. }
        | IssueCommand::SprintStart { .. }
        | IssueCommand::SprintComplete { .. }
        | IssueCommand::SprintUpdate { .. }
        | IssueCommand::SprintDelete { .. }
        | IssueCommand::SprintRemoveIssue { .. }
        | IssueCommand::Rank { .. }) => {
            handle_sprint_command(client, command, default_project).await
        }
        _ => anyhow::bail!("Unsupported browse or Agile issue command"),
    }
}

async fn handle_query_command(
    client: JiraClient,
    command: IssueCommand,
    default_project: Option<String>,
    default_issue_limit: Option<u32>,
) -> Result<()> {
    match command {
        IssueCommand::List {
            project,
            jql,
            limit,
            all,
            json,
        } => {
            list_issues(
                client,
                project.or(default_project),
                jql,
                LimitOptions {
                    limit,
                    all,
                    configured: default_issue_limit,
                },
                json,
            )
            .await
        }
        IssueCommand::Export {
            project,
            jql,
            all,
            limit,
            format,
            output,
        } => {
            export_issues(
                client,
                project.or(default_project),
                jql,
                LimitOptions {
                    limit,
                    all,
                    configured: default_issue_limit,
                },
                format,
                output,
            )
            .await
        }
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
        _ => anyhow::bail!("Unsupported issue query command"),
    }
}

async fn handle_sprint_command(
    client: JiraClient,
    command: IssueCommand,
    default_project: Option<String>,
) -> Result<()> {
    match command {
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
        command @ (IssueCommand::SprintRemoveIssue { .. } | IssueCommand::Rank { .. }) => {
            super::handle_agile_issue_order(client, command).await
        }
        _ => anyhow::bail!("Unsupported Agile command"),
    }
}
// ─── list ────────────────────────────────────────────────────────────────────

pub(super) async fn list_issues(
    client: JiraClient,
    project: Option<String>,
    jql: Option<String>,
    bounds: LimitOptions,
    json: bool,
) -> Result<()> {
    let jql_query = if let Some(jql) = jql {
        jql
    } else if let Some(proj) = &project {
        format!("project = {proj} ORDER BY updated DESC")
    } else {
        "assignee = currentUser() ORDER BY updated DESC".to_string()
    };

    // Resolve the effective request limit (priority: --all > --limit >
    // config.default_issue_limit > "all"). `None` means unfetched/unbounded.
    let effective_limit = bounds.effective();

    if effective_limit.is_none() && !bounds.all {
        // No explicit cap anywhere: we are about to fetch every issue. Give a
        // gentle, stderr-only hint (keeps --json output clean) on how to cap.
        eprintln!("ℹ️  No default issue limit set — fetching all issues (this may be slow).");
        eprintln!(
            "   Tip: set a cap with `jirac config set default_issue_limit <N>` or pass --limit."
        );
    }

    let spinner = spinner_new("Fetching issues...");
    let result = fetch_issues(&client, &jql_query, effective_limit).await?;
    spinner.finish_and_clear();

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    if result.is_empty() {
        println!("No issues found.");
        return Ok(());
    }

    println!(
        "{:<12} {:<8} {:<20} {:<40}",
        "KEY", "TYPE", "STATUS", "SUMMARY"
    );
    println!("{}", "─".repeat(82));

    for issue in &result {
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

    println!("\nShowing {} issues", result.len());

    Ok(())
}

/// Jira Cloud's per-request `maxResults` upper bound.
const JIRA_MAX_RESULTS: u32 = 5_000;

/// Fetch issues honoring an optional cap. `None` fetches every matching issue
/// via cursor-based pagination; `Some(n)` issues a single bounded request
/// (clamped to [`JIRA_MAX_RESULTS`] to avoid a raw Jira API 400).
async fn fetch_issues(client: &JiraClient, jql: &str, limit: Option<u32>) -> Result<Vec<Issue>> {
    match limit {
        Some(n) => {
            let raw = n;
            let n = raw.min(JIRA_MAX_RESULTS);
            if raw > JIRA_MAX_RESULTS {
                eprintln!(
                    "ℹ️  Requested limit {raw} exceeds Jira's {JIRA_MAX_RESULTS} per-request cap; using {JIRA_MAX_RESULTS}."
                );
            }

            let result = client
                .search_issues(jql, None, Some(n))
                .await
                .context("Failed to search issues")?;
            Ok(result.issues)
        }
        None => client
            .get_all_issues(jql)
            .await
            .context("Failed to fetch all issues"),
    }
}

pub(super) async fn export_issues(
    client: JiraClient,
    project: Option<String>,
    jql: Option<String>,
    bounds: LimitOptions,
    format: ExportFormat,
    output: Option<std::path::PathBuf>,
) -> Result<()> {
    let jql_query = if let Some(jql) = jql {
        jql
    } else if let Some(proj) = &project {
        format!("project = {proj} ORDER BY updated DESC")
    } else {
        "assignee = currentUser() ORDER BY updated DESC".to_string()
    };

    // Priority: --all > --limit > config.default_issue_limit > (fetch all).
    let effective_limit = bounds.effective();

    let spinner = spinner_new("Exporting issues...");
    let issues = fetch_issues(&client, &jql_query, effective_limit).await?;
    spinner.finish_and_clear();

    let rendered: String = match format {
        ExportFormat::Json => serde_json::to_string_pretty(&issues)?,
        ExportFormat::Csv => render_issues_csv(&issues),
    };

    match output {
        Some(path) => {
            std::fs::write(&path, rendered)
                .with_context(|| format!("Failed to write export to {}", path.display()))?;
            println!(
                "✓ Exported {} issues ({format:?}) to {}",
                issues.len(),
                path.display()
            );
        }
        None => println!("{rendered}"),
    }

    Ok(())
}

/// Render issues as a simple CSV (header + one row per issue). Values are
/// escaped for commas, quotes, and newlines.
fn render_issues_csv(issues: &[Issue]) -> String {
    let mut out = String::new();
    out.push_str("id,key,project,type,status,summary,assignee,reporter,priority,created,updated\n");

    for issue in issues {
        let row = [
            &issue.id,
            &issue.key,
            &issue.project_key,
            &issue.issue_type,
            &issue.status,
            &issue.summary,
            issue.assignee.as_deref().unwrap_or(""),
            issue.reporter.as_deref().unwrap_or(""),
            issue.priority.as_deref().unwrap_or(""),
            &issue.created,
            &issue.updated,
        ];
        let escaped: Vec<String> = row.iter().map(|cell| csv_escape(cell)).collect();
        out.push_str(&escaped.join(","));
        out.push('\n');
    }

    out
}

fn csv_escape(cell: &str) -> String {
    if cell.contains(',') || cell.contains('"') || cell.contains('\n') {
        format!("\"{}\"", cell.replace('"', "\"\""))
    } else {
        cell.to_string()
    }
}

pub(super) async fn notifications(
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

pub(super) async fn view_issue(
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
pub(super) struct ProjectVersionUpdateArgs {
    pub(super) create: bool,
    pub(super) set_name: Option<String>,
    pub(super) description: Option<String>,
    pub(super) clear_description: bool,
    pub(super) set_release_date: Option<String>,
    pub(super) clear_release_date: bool,
    pub(super) set_start_date: Option<String>,
    pub(super) clear_start_date: bool,
    pub(super) released: bool,
    pub(super) unreleased: bool,
    pub(super) archived: bool,
    pub(super) unarchived: bool,
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

pub(super) struct SprintUpdateArgs {
    pub(super) name: Option<String>,
    pub(super) goal: Option<String>,
    pub(super) clear_goal: bool,
    pub(super) start_date: Option<String>,
    pub(super) clear_start_date: bool,
    pub(super) end_date: Option<String>,
    pub(super) clear_end_date: bool,
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

pub(super) async fn view_project_versions(
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

pub(super) async fn list_sprints(
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
pub(super) async fn create_sprint(
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

pub(super) async fn start_sprint(
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

pub(super) async fn sprint_complete(
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

pub(super) async fn update_sprint_command(
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

pub(super) async fn sprint_delete(
    client: JiraClient,
    project: Option<String>,
    sprint: String,
    force: bool,
) -> Result<()> {
    let project_key = project
        .context("Project key is required. Pass --project or configure a default project.")?;
    let sprint_meta = resolve_sprint_for_project(&client, &project_key, &sprint).await?;
    if !force {
        require_interactive("confirmation", "--force")?;
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

pub(super) async fn standup_summary(
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

pub(super) async fn sprint_summary(
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
