use jira_core::model::Issue;
use ratatui::{
    layout::Constraint,
    style::{Color, Style},
    widgets::Cell,
};

pub(super) const AVAILABLE_COLUMNS: [ColumnKind; 8] = [
    ColumnKind::Key,
    ColumnKind::Type,
    ColumnKind::Priority,
    ColumnKind::Status,
    ColumnKind::Assignee,
    ColumnKind::Created,
    ColumnKind::Updated,
    ColumnKind::Summary,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(super) enum ColumnKind {
    Key,
    Type,
    Priority,
    Status,
    Assignee,
    Created,
    Updated,
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
            ColumnKind::Created => "Created",
            ColumnKind::Updated => "Updated",
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
            ColumnKind::Created => Constraint::Length(11),
            ColumnKind::Updated => Constraint::Length(11),
            ColumnKind::Summary => Constraint::Min(15),
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
            ColumnKind::Summary => {
                let summary = if issue.summary.len() > 40 {
                    format!("{}…", &issue.summary[..39])
                } else {
                    issue.summary.clone()
                };
                Cell::from(summary)
            }
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

pub(super) fn format_column_summary(columns: &[ColumnKind]) -> String {
    columns
        .iter()
        .map(|column| column.label())
        .collect::<Vec<_>>()
        .join(", ")
}
