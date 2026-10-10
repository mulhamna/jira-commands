use super::manage::truncate;
use super::*;
pub(super) async fn handle_issue_actions(client: JiraClient, command: IssueCommand) -> Result<()> {
    match command {
        IssueCommand::Comment { key, command } => comment(client, key, command).await,
        IssueCommand::Worklog { key, command } => worklog(client, key, command).await,
        IssueCommand::Watch { key, command } => watch(client, key, command).await,
        IssueCommand::BulkComment {
            jql,
            keys,
            body,
            file,
            force,
            json,
        } => bulk_comment(client, jql, keys, body, file, force, json).await,
        _ => anyhow::bail!("Unsupported issue action"),
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
        require_interactive("confirmation", "--force")?;
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
        CommentCommand::Update { id, body, file } => {
            let body = read_comment_body(body, file)?;
            let comment = client
                .update_comment(&key, &id, &body)
                .await
                .context("Failed to update comment")?;
            println!("✓ Updated comment {} on {}", comment.id, key);
            Ok(())
        }
        CommentCommand::Delete { id, force } => {
            if !force {
                require_interactive("confirmation", "--force")?;
                let confirm = inquire::Confirm::new(&format!("Delete comment {id} on {key}?"))
                    .with_default(false)
                    .prompt()
                    .context("Failed to read confirmation")?;
                if !confirm {
                    println!("Aborted.");
                    return Ok(());
                }
            }
            client
                .delete_comment(&key, &id)
                .await
                .context("Failed to delete comment")?;
            println!("✓ Deleted comment {id} from {key}");
            Ok(())
        }
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
        require_interactive("confirmation", "--force")?;
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
        WorklogCommand::Update {
            id,
            time,
            comment,
            date,
            start,
        } => {
            if time.is_none() && comment.is_none() && date.is_none() && start.is_none() {
                anyhow::bail!("Provide at least one of --time, --comment, --date, or --start");
            }
            let jira_timezone = if date.is_some() || start.is_some() {
                client
                    .get_myself_timezone()
                    .await
                    .context("Failed to fetch Jira user timezone")?
            } else {
                None
            };
            let started =
                build_worklog_started(date.as_deref(), start.as_deref(), jira_timezone.as_deref())?;
            let log = client
                .update_worklog(
                    &key,
                    &id,
                    time.as_deref(),
                    comment.as_deref(),
                    started.as_deref(),
                )
                .await
                .context("Failed to update worklog")?;
            println!("✓ Updated worklog {} on {}", log.id, key);
            Ok(())
        }
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
        require_interactive("confirmation", "--force")?;
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
