use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use jira_core::{
    adf::{adf_to_text, mentioned_account_ids},
    config::config_file_path,
    model::Issue,
    JiraClient,
};

#[derive(Debug, Clone, serde::Serialize)]
pub struct NotificationEntry {
    pub id: String,
    pub issue: Issue,
    pub source: String,
    pub author: Option<String>,
    pub created: String,
    pub excerpt: String,
    pub url: String,
    pub read: bool,
}

#[derive(Debug, Clone)]
pub struct NotificationScan {
    pub entries: Vec<NotificationEntry>,
    pub scanned_issues: usize,
    pub comment_errors: usize,
    pub jql: String,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct NotificationReadState {
    read_ids: HashSet<String>,
}

type NotificationIssueGroup = (Issue, usize, usize, Option<DateTime<Utc>>);

pub fn build_notifications_jql(project: Option<&str>, since: &str) -> String {
    let since = since.trim();
    if let Some(project) = project {
        format!("project = {project} AND updated >= -{since} ORDER BY updated DESC")
    } else {
        format!("updated >= -{since} ORDER BY updated DESC")
    }
}

fn notifications_state_path() -> PathBuf {
    config_file_path()
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("notifications-read.json")
}

fn load_notification_read_state() -> NotificationReadState {
    let path = notifications_state_path();
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_notification_read_state(state: &NotificationReadState) -> Result<()> {
    let path = notifications_state_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    let raw = serde_json::to_string_pretty(state)?;
    fs::write(&path, raw).with_context(|| format!("Failed to write {}", path.display()))
}

fn notification_entry_id(issue_key: &str, source: &str, created: &str) -> String {
    format!("{issue_key}|{source}|{created}")
}

pub fn mark_notifications_read(
    entries: &mut [NotificationEntry],
    issue_key: &str,
) -> Result<usize> {
    let mut state = load_notification_read_state();
    let mut changed = 0usize;

    for entry in entries
        .iter_mut()
        .filter(|entry| entry.issue.key == issue_key)
    {
        if !entry.read && state.read_ids.insert(entry.id.clone()) {
            entry.read = true;
            changed += 1;
        }
    }

    if changed > 0 {
        save_notification_read_state(&state)?;
    }

    Ok(changed)
}

pub async fn scan_mention_notifications(
    client: &JiraClient,
    project: Option<&str>,
    since: &str,
    limit: u32,
) -> Result<NotificationScan> {
    let account_id = client
        .get_myself()
        .await
        .context("Failed to resolve current Jira account")?;
    let limit = limit.clamp(1, 100);
    let jql = build_notifications_jql(project, since);
    let result = client
        .search_issues(&jql, None, Some(limit))
        .await
        .context("Failed to fetch recent issues for notification scan")?;

    let read_state = load_notification_read_state();
    let mut entries = Vec::new();
    let mut comment_errors = 0usize;
    let scanned_issues = result.issues.len();

    for issue in result.issues {
        if let Some(description) = issue.description.as_ref() {
            if mentioned_account_ids(description)
                .iter()
                .any(|mentioned| mentioned == &account_id)
            {
                let id = notification_entry_id(&issue.key, "description-mention", &issue.updated);
                entries.push(NotificationEntry {
                    id: id.clone(),
                    issue: issue.clone(),
                    source: "description-mention".to_string(),
                    author: issue.reporter.clone(),
                    created: issue.updated.clone(),
                    excerpt: notification_excerpt(&adf_to_text(description)),
                    url: format!("{}/browse/{}", client.base_url(), issue.key),
                    read: read_state.read_ids.contains(&id),
                });
            }
        }

        match client.get_comments(&issue.key).await {
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
                        entries.push(NotificationEntry {
                            id: id.clone(),
                            issue: issue.clone(),
                            source: "comment-mention".to_string(),
                            author: comment.author.clone(),
                            created: comment.created.clone(),
                            excerpt: notification_excerpt(comment.body.as_deref().unwrap_or("")),
                            url: format!("{}/browse/{}", client.base_url(), issue.key),
                            read: read_state.read_ids.contains(&id),
                        });
                    }
                }
            }
            Err(_) => {
                comment_errors += 1;
            }
        }
    }

    entries.sort_by_key(|entry| std::cmp::Reverse(parse_jira_datetime(&entry.created)));

    Ok(NotificationScan {
        entries,
        scanned_issues,
        comment_errors,
        jql,
    })
}

pub fn notification_issues(entries: &[NotificationEntry]) -> Vec<Issue> {
    let mut grouped: HashMap<String, NotificationIssueGroup> = HashMap::new();

    for entry in entries {
        let ts = parse_jira_datetime(&entry.created);
        grouped
            .entry(entry.issue.key.clone())
            .and_modify(|(_, total, unread, latest)| {
                *total += 1;
                if !entry.read {
                    *unread += 1;
                }
                if ts > *latest {
                    *latest = ts;
                }
            })
            .or_insert_with(|| (entry.issue.clone(), 1, usize::from(!entry.read), ts));
    }

    let mut items: Vec<NotificationIssueGroup> = grouped.into_values().collect();
    items.sort_by_key(|(_, _, _, latest)| std::cmp::Reverse(*latest));

    items
        .into_iter()
        .map(|(mut issue, total, unread, _)| {
            issue.summary = if unread == 0 {
                format!("[read] {}", issue.summary)
            } else if unread == total && total == 1 {
                format!("[mention] {}", issue.summary)
            } else if unread == total {
                format!("[{total} mentions] {}", issue.summary)
            } else {
                format!("[{unread} unread/{total} mentions] {}", issue.summary)
            };
            issue
        })
        .collect()
}

pub fn notification_issue_jql(entries: &[NotificationEntry], fallback_jql: &str) -> String {
    let mut keys: Vec<String> = entries
        .iter()
        .map(|entry| entry.issue.key.clone())
        .collect();
    keys.sort();
    keys.dedup();

    if keys.is_empty() {
        return fallback_jql.to_string();
    }

    format!("key in ({}) ORDER BY updated DESC", keys.join(", "))
}

pub fn notification_excerpt(raw: &str) -> String {
    let normalized = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        "(no preview)".to_string()
    } else {
        normalized
    }
}

pub fn parse_jira_datetime(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use jira_core::model::Issue;

    fn issue(key: &str, summary: &str) -> Issue {
        Issue {
            id: format!("id-{key}"),
            key: key.to_string(),
            summary: summary.to_string(),
            description: None,
            status: "To Do".to_string(),
            assignee: None,
            reporter: None,
            priority: None,
            issue_type: "Task".to_string(),
            project_key: "TEST".to_string(),
            created: "2023-01-01T00:00:00Z".to_string(),
            updated: "2023-01-01T00:00:00Z".to_string(),
            attachments: vec![],
            links: vec![],
            fields: serde_json::json!({}),
        }
    }

    #[test]
    fn notification_issues_groups_and_marks_read_state() {
        let issues = notification_issues(&[
            NotificationEntry {
                id: "1".into(),
                issue: issue("TEST-1", "Alpha"),
                source: "comment-mention".into(),
                author: Some("A".into()),
                created: "2024-01-02T00:00:00Z".into(),
                excerpt: "x".into(),
                url: "https://example/browse/TEST-1".into(),
                read: false,
            },
            NotificationEntry {
                id: "2".into(),
                issue: issue("TEST-1", "Alpha"),
                source: "description-mention".into(),
                author: Some("B".into()),
                created: "2024-01-01T00:00:00Z".into(),
                excerpt: "y".into(),
                url: "https://example/browse/TEST-1".into(),
                read: true,
            },
            NotificationEntry {
                id: "3".into(),
                issue: issue("TEST-2", "Beta"),
                source: "comment-mention".into(),
                author: Some("C".into()),
                created: "2024-01-03T00:00:00Z".into(),
                excerpt: "z".into(),
                url: "https://example/browse/TEST-2".into(),
                read: true,
            },
        ]);

        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].key, "TEST-2");
        assert_eq!(issues[0].summary, "[read] Beta");
        assert_eq!(issues[1].summary, "[1 unread/2 mentions] Alpha");
    }

    #[test]
    fn notification_entry_id_is_stable_for_same_source() {
        assert_eq!(
            notification_entry_id("TEST-1", "comment-mention", "2024-01-01T00:00:00Z"),
            "TEST-1|comment-mention|2024-01-01T00:00:00Z"
        );
    }
}
