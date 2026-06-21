use std::{collections::HashSet, fs, path::PathBuf, sync::Arc};

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{json, Value};

use jira_core::{
    adf::{adf_to_text, mentioned_account_ids},
    config::config_file_path,
    model::Issue,
    JiraClient,
};

use crate::{
    error::{AppError, AppResult},
    models::NotificationsMarkReadArgs,
};

use super::{jql::build_notifications_jql, JiraApp};

impl JiraApp {
    pub async fn notifications_mark_read(
        &self,
        args: NotificationsMarkReadArgs,
    ) -> AppResult<Value> {
        if args.ids.is_empty() {
            return Err(AppError::validation(
                "Provide at least one notification id to mark as read",
            ));
        }
        let marked = mark_notifications_read(&args.ids)?;
        Ok(json!({
            "marked": marked,
            "ids": args.ids,
        }))
    }
}

#[derive(Debug, Clone)]
pub(super) struct ResolvedTransition {
    pub(super) id: String,
    pub(super) name: String,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct IssueNotificationEntry {
    pub(super) id: String,
    pub(super) issue: Issue,
    pub(super) source: String,
    pub(super) author: Option<String>,
    pub(super) created: String,
    pub(super) excerpt: String,
    pub(super) url: String,
    pub(super) read: bool,
}

#[derive(Debug, Clone)]
pub(super) struct IssueNotificationScan {
    pub(super) entries: Vec<IssueNotificationEntry>,
    pub(super) scanned_issues: usize,
    pub(super) comment_errors: usize,
    pub(super) jql: String,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub(super) struct NotificationReadState {
    pub(super) read_ids: HashSet<String>,
}

pub(super) fn notifications_state_path() -> PathBuf {
    config_file_path()
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("notifications-read.json")
}

pub(super) fn load_notification_read_state() -> NotificationReadState {
    let path = notifications_state_path();
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub(super) fn save_notification_read_state(state: &NotificationReadState) -> AppResult<()> {
    let path = notifications_state_path();
    let raw = serde_json::to_string(state)?;
    fs::write(&path, raw)?;
    Ok(())
}

/// Mark the given notification ids as read, persisting the read state.
/// Returns the number of ids that were not already marked.
pub(super) fn mark_notifications_read(ids: &[String]) -> AppResult<usize> {
    let mut state = load_notification_read_state();
    let newly = ids
        .iter()
        .filter(|id| state.read_ids.insert((*id).clone()))
        .count();
    save_notification_read_state(&state)?;
    Ok(newly)
}

pub(super) fn notification_entry_id(issue_key: &str, source: &str, created: &str) -> String {
    format!("{issue_key}|{source}|{created}")
}

pub(super) async fn scan_issue_notifications(
    client: &JiraClient,
    project: Option<&str>,
    since: &str,
    limit: u32,
) -> AppResult<IssueNotificationScan> {
    let account_id = client.get_myself().await?;
    let limit = limit.clamp(1, 100);
    let jql = build_notifications_jql(project, since);
    let result = client.search_issues(&jql, None, Some(limit)).await?;

    let read_state = load_notification_read_state();
    let mut entries = Vec::new();
    let mut comment_errors = 0usize;
    let scanned_issues = result.issues.len();
    let base_url = client.base_url().to_string();
    let mut comment_tasks = tokio::task::JoinSet::new();
    let comment_scan_limit = Arc::new(tokio::sync::Semaphore::new(8));

    for issue in result.issues {
        if let Some(description) = issue.description.as_ref() {
            if mentioned_account_ids(description)
                .iter()
                .any(|mentioned| mentioned == &account_id)
            {
                let id = notification_entry_id(&issue.key, "description-mention", &issue.updated);
                entries.push(IssueNotificationEntry {
                    id: id.clone(),
                    issue: issue.clone(),
                    source: "description-mention".to_string(),
                    author: issue.reporter.clone(),
                    created: issue.updated.clone(),
                    excerpt: notification_excerpt(&adf_to_text(description)),
                    url: format!("{base_url}/browse/{}", issue.key),
                    read: read_state.read_ids.contains(&id),
                });
            }
        }

        let issue_for_comments = issue.clone();
        let client_for_comments = client.clone();
        let comment_scan_limit = Arc::clone(&comment_scan_limit);
        comment_tasks.spawn(async move {
            let _permit = comment_scan_limit.acquire_owned().await.ok();
            let comments = client_for_comments
                .get_comments(&issue_for_comments.key)
                .await;
            (issue_for_comments, comments)
        });
    }

    while let Some(joined) = comment_tasks.join_next().await {
        let Ok((issue, comments)) = joined else {
            comment_errors += 1;
            continue;
        };

        match comments {
            Ok(comments) => {
                for comment in comments {
                    if comment.author_account_id.as_deref() == Some(account_id.as_str()) {
                        continue;
                    }
                    if comment
                        .mentions
                        .iter()
                        .any(|mentioned| mentioned == &account_id)
                    {
                        let id =
                            notification_entry_id(&issue.key, "comment-mention", &comment.created);
                        entries.push(IssueNotificationEntry {
                            id: id.clone(),
                            issue: issue.clone(),
                            source: "comment-mention".to_string(),
                            author: comment.author.clone(),
                            created: comment.created.clone(),
                            excerpt: notification_excerpt(comment.body.as_deref().unwrap_or("")),
                            url: format!("{base_url}/browse/{}", issue.key),
                            read: read_state.read_ids.contains(&id),
                        });
                    }
                }
            }
            Err(_) => comment_errors += 1,
        }
    }

    entries.sort_by_key(|entry| std::cmp::Reverse(parse_jira_datetime(&entry.created)));

    Ok(IssueNotificationScan {
        entries,
        scanned_issues,
        comment_errors,
        jql,
    })
}

pub(super) fn parse_jira_datetime(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

pub(super) fn notification_excerpt(raw: &str) -> String {
    let condensed = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if condensed.chars().count() <= 140 {
        condensed
    } else {
        let truncated: String = condensed.chars().take(137).collect();
        format!("{truncated}...")
    }
}

pub(super) async fn resolve_transition(
    client: &JiraClient,
    key: &str,
    name_or_id: &str,
) -> AppResult<ResolvedTransition> {
    let transitions = client.get_transitions(key).await?;
    let found = transitions
        .into_iter()
        .find(|t| t.id == name_or_id || t.name.eq_ignore_ascii_case(name_or_id))
        .ok_or_else(|| {
            AppError::not_found(
                format!("Transition '{name_or_id}' not found for {key}"),
                Some(json!({ "key": key })),
            )
        })?;

    Ok(ResolvedTransition {
        id: found.id,
        name: found.name,
    })
}
