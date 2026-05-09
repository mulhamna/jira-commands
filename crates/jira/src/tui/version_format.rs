use jira_core::model::{Issue, ProjectVersion};

use crate::version_insights::{IssueVersionInsight, VersionBacklogPreview};

pub(super) fn version_status_badges(version: &ProjectVersion) -> String {
    let mut parts: Vec<String> = Vec::new();
    if version.archived {
        parts.push("archived".into());
    } else if version.released {
        parts.push("released".into());
    } else {
        parts.push("unreleased".into());
    }
    if let Some(date) = version.release_date.as_deref() {
        let short = &date[..10.min(date.len())];
        parts.push(format!("release {short}"));
    }
    parts.join(" • ")
}

pub(super) fn preview_issue_line(issue: &Issue) -> String {
    format!("{} [{}] {}", issue.key, issue.status, issue.summary)
}

pub(super) fn project_versions_preview(insight: &IssueVersionInsight, limit: usize) -> Vec<String> {
    insight
        .project_versions
        .iter()
        .take(limit)
        .map(|version| {
            let marker = if insight
                .issue_fix_versions
                .iter()
                .any(|name| name == &version.name)
            {
                "*"
            } else {
                "•"
            };
            let badges = version_status_badges(version);
            if badges.is_empty() {
                format!("{marker} {}", version.name)
            } else {
                format!("{marker} {} ({badges})", version.name)
            }
        })
        .collect()
}

pub(super) fn backlog_preview_lines(preview: &VersionBacklogPreview, limit: usize) -> Vec<String> {
    let mut lines = vec![format!(
        "{} — {} open issue(s)",
        preview.version.name, preview.total_open
    )];
    if preview.issues.is_empty() {
        lines.push("  ✓ No open backlog items".into());
        return lines;
    }
    for issue in preview.issues.iter().take(limit) {
        lines.push(format!("  - {}", preview_issue_line(issue)));
    }
    if preview.total_open > preview.issues.len() as u64 {
        lines.push(format!(
            "  … {} more",
            preview
                .total_open
                .saturating_sub(preview.issues.len() as u64)
        ));
    }
    lines
}
