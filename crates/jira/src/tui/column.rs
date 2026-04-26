use jira_core::model::Issue;
use ratatui::{
    layout::Constraint,
    style::{Color, Style},
    widgets::Cell,
};

pub(super) const AVAILABLE_COLUMNS: [ColumnKind; 13] = [
    ColumnKind::Key,
    ColumnKind::Type,
    ColumnKind::Priority,
    ColumnKind::Status,
    ColumnKind::Assignee,
    ColumnKind::Reporter,
    ColumnKind::Project,
    ColumnKind::Created,
    ColumnKind::Updated,
    ColumnKind::Labels,
    ColumnKind::Components,
    ColumnKind::FixVersions,
    ColumnKind::Summary,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(super) enum ColumnKind {
    Key,
    Type,
    Priority,
    Status,
    Assignee,
    Reporter,
    Project,
    Created,
    Updated,
    Labels,
    Components,
    FixVersions,
    Summary,
}

impl ColumnKind {
    pub(super) fn label(self) -> &'static str {
        match self {
            ColumnKind::Key => "Key",
            ColumnKind::Type => "Type",
            ColumnKind::Priority => "Priority",
            ColumnKind::Status => "Status",
            ColumnKind::Assignee => "Assignee",
            ColumnKind::Reporter => "Reporter",
            ColumnKind::Project => "Project",
            ColumnKind::Created => "Created",
            ColumnKind::Updated => "Updated",
            ColumnKind::Labels => "Labels",
            ColumnKind::Components => "Components",
            ColumnKind::FixVersions => "Fix Versions",
            ColumnKind::Summary => "Summary",
        }
    }

    pub(super) fn width(self) -> Constraint {
        match self {
            ColumnKind::Key => Constraint::Length(12),
            ColumnKind::Type => Constraint::Length(8),
            ColumnKind::Priority => Constraint::Length(8),
            ColumnKind::Status => Constraint::Length(14),
            ColumnKind::Assignee => Constraint::Length(16),
            ColumnKind::Reporter => Constraint::Length(16),
            ColumnKind::Project => Constraint::Length(10),
            ColumnKind::Created => Constraint::Length(11),
            ColumnKind::Updated => Constraint::Length(11),
            ColumnKind::Labels => Constraint::Min(14),
            ColumnKind::Components => Constraint::Min(14),
            ColumnKind::FixVersions => Constraint::Min(14),
            ColumnKind::Summary => Constraint::Min(24),
        }
    }

    pub(super) fn cell(self, issue: &Issue) -> Cell<'static> {
        match self {
            ColumnKind::Key => {
                Cell::from(issue.key.clone()).style(Style::default().fg(Color::Cyan))
            }
            ColumnKind::Type => Cell::from(issue.issue_type.clone()),
            ColumnKind::Priority => {
                Cell::from(issue.priority.clone().unwrap_or_else(|| "-".into()))
            }
            ColumnKind::Status => Cell::from(issue.status.clone())
                .style(Style::default().fg(status_color(&issue.status))),
            ColumnKind::Assignee => {
                Cell::from(issue.assignee.clone().unwrap_or_else(|| "-".into()))
            }
            ColumnKind::Reporter => {
                Cell::from(issue.reporter.clone().unwrap_or_else(|| "-".into()))
            }
            ColumnKind::Project => Cell::from(issue.project_key.clone()),
            ColumnKind::Created => Cell::from(
                issue
                    .created
                    .get(..10)
                    .unwrap_or(&issue.created)
                    .to_string(),
            )
            .style(Style::default().fg(Color::DarkGray)),
            ColumnKind::Updated => Cell::from(
                issue
                    .updated
                    .get(..10)
                    .unwrap_or(&issue.updated)
                    .to_string(),
            )
            .style(Style::default().fg(Color::DarkGray)),
            ColumnKind::Labels => Cell::from(join_string_array(issue, "labels")),
            ColumnKind::Components => Cell::from(join_named_array(issue, "components")),
            ColumnKind::FixVersions => Cell::from(join_named_array(issue, "fixVersions")),
            ColumnKind::Summary => Cell::from(issue.summary.clone()),
        }
    }
}

pub(super) fn status_color(status: &str) -> Color {
    let s = status.to_lowercase();
    if s.contains("done") || s.contains("closed") || s.contains("resolved") {
        Color::Green
    } else if s.contains("progress") || s.contains("review") {
        Color::Yellow
    } else if s.contains("blocked") || s.contains("impediment") {
        Color::Red
    } else {
        Color::White
    }
}

fn join_string_array(issue: &Issue, field: &str) -> String {
    issue
        .fields
        .get(field)
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "-".into())
}

fn join_named_array(issue: &Issue, field: &str) -> String {
    issue
        .fields
        .get(field)
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("name").and_then(|value| value.as_str()))
                .map(|name| name.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "-".into())
}

pub(super) fn format_column_summary(columns: &[ColumnKind]) -> String {
    columns
        .iter()
        .map(|column| column.label())
        .collect::<Vec<_>>()
        .join(", ")
}
