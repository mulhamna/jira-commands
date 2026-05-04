use std::collections::HashMap;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use jira_core::{
    adf::{adf_to_text, mentioned_account_ids},
    model::Issue,
    JiraClient,
};

#[derive(Debug, Clone, serde::Serialize)]
pub struct NotificationEntry {
    pub issue: Issue,
    pub source: String,
    pub author: Option<String>,
    pub created: String,
    pub excerpt: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct NotificationScan {
    pub entries: Vec<NotificationEntry>,
    pub scanned_issues: usize,
    pub comment_errors: usize,
    pub jql: String,
}

pub fn build_notifications_jql(project: Option<&str>, since: &str) -> String {
    let since = since.trim();
    if let Some(project) = project {
        format!("project = {project} AND updated >= -{since} ORDER BY updated DESC")
    } else {
        format!("updated >= -{since} ORDER BY updated DESC")
    }
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

    let mut entries = Vec::new();
    let mut comment_errors = 0usize;
    let scanned_issues = result.issues.len();

    for issue in result.issues {
        if let Some(description) = issue.description.as_ref() {
            if mentioned_account_ids(description)
                .iter()
                .any(|mentioned| mentioned == &account_id)
            {
                entries.push(NotificationEntry {
                    issue: issue.clone(),
                    source: "description-mention".to_string(),
                    author: issue.reporter.clone(),
                    created: issue.updated.clone(),
                    excerpt: notification_excerpt(&adf_to_text(description)),
                    url: format!("{}/browse/{}", client.base_url(), issue.key),
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
                        entries.push(NotificationEntry {
                            issue: issue.clone(),
                            source: "comment-mention".to_string(),
                            author: comment.author.clone(),
                            created: comment.created.clone(),
                            excerpt: notification_excerpt(comment.body.as_deref().unwrap_or("")),
                            url: format!("{}/browse/{}", client.base_url(), issue.key),
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
    let mut grouped: HashMap<String, (Issue, usize, Option<DateTime<Utc>>)> = HashMap::new();

    for entry in entries {
        let ts = parse_jira_datetime(&entry.created);
        grouped
            .entry(entry.issue.key.clone())
            .and_modify(|(_, count, latest)| {
                *count += 1;
                if ts > *latest {
                    *latest = ts;
                }
            })
            .or_insert_with(|| (entry.issue.clone(), 1, ts));
    }

    let mut items: Vec<(Issue, usize, Option<DateTime<Utc>>)> = grouped.into_values().collect();
    items.sort_by_key(|(_, _, latest)| std::cmp::Reverse(*latest));

    items
        .into_iter()
        .map(|(mut issue, count, _)| {
            issue.summary = if count > 1 {
                format!("[{count} mentions] {}", issue.summary)
            } else {
                format!("[mention] {}", issue.summary)
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
