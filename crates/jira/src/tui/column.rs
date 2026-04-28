use jira_core::model::{field::Field, Issue};
use ratatui::{
    layout::Constraint,
    style::{Color, Style},
    widgets::Cell,
};
use serde_json::Value;

/// A renderable column in the issue table.
#[derive(Debug, Clone)]
pub(super) struct ColumnSpec {
    pub(super) id: String,
    pub(super) label: String,
    pub(super) width: Constraint,
}

impl ColumnSpec {
    pub(super) fn for_id(id: &str) -> ColumnSpec {
        if let Some(b) = BUILTIN_COLUMNS.iter().find(|b| b.id == id) {
            return ColumnSpec {
                id: b.id.to_string(),
                label: b.label.to_string(),
                width: b.width,
            };
        }
        ColumnSpec {
            id: id.to_string(),
            label: id.to_string(),
            width: Constraint::Min(14),
        }
    }

    pub(super) fn from_field(field: &Field) -> ColumnSpec {
        if let Some(b) = BUILTIN_COLUMNS.iter().find(|b| b.id == field.id) {
            return ColumnSpec {
                id: b.id.to_string(),
                label: b.label.to_string(),
                width: b.width,
            };
        }
        ColumnSpec {
            id: field.id.clone(),
            label: field.name.clone(),
            width: Constraint::Min(14),
        }
    }

    pub(super) fn cell(&self, issue: &Issue) -> Cell<'static> {
        if let Some(b) = BUILTIN_COLUMNS.iter().find(|b| b.id == self.id) {
            return (b.render)(issue);
        }
        render_generic_field(issue, &self.id)
    }
}

pub(super) struct BuiltinColumn {
    pub(super) id: &'static str,
    pub(super) label: &'static str,
    pub(super) width: Constraint,
    pub(super) render: fn(&Issue) -> Cell<'static>,
}

pub(super) const BUILTIN_COLUMNS: &[BuiltinColumn] = &[
    BuiltinColumn {
        id: "key",
        label: "Key",
        width: Constraint::Length(12),
        render: render_key,
    },
    BuiltinColumn {
        id: "issuetype",
        label: "Type",
        width: Constraint::Length(8),
        render: render_type,
    },
    BuiltinColumn {
        id: "priority",
        label: "Priority",
        width: Constraint::Length(8),
        render: render_priority,
    },
    BuiltinColumn {
        id: "status",
        label: "Status",
        width: Constraint::Length(14),
        render: render_status,
    },
    BuiltinColumn {
        id: "assignee",
        label: "Assignee",
        width: Constraint::Length(16),
        render: render_assignee,
    },
    BuiltinColumn {
        id: "reporter",
        label: "Reporter",
        width: Constraint::Length(16),
        render: render_reporter,
    },
    BuiltinColumn {
        id: "project",
        label: "Project",
        width: Constraint::Length(10),
        render: render_project,
    },
    BuiltinColumn {
        id: "created",
        label: "Created",
        width: Constraint::Length(11),
        render: render_created,
    },
    BuiltinColumn {
        id: "updated",
        label: "Updated",
        width: Constraint::Length(11),
        render: render_updated,
    },
    BuiltinColumn {
        id: "labels",
        label: "Labels",
        width: Constraint::Min(14),
        render: render_labels,
    },
    BuiltinColumn {
        id: "components",
        label: "Components",
        width: Constraint::Min(14),
        render: render_components,
    },
    BuiltinColumn {
        id: "fixVersions",
        label: "Fix Versions",
        width: Constraint::Min(14),
        render: render_fix_versions,
    },
    BuiltinColumn {
        id: "summary",
        label: "Summary",
        width: Constraint::Min(24),
        render: render_summary,
    },
];

pub(super) fn default_column_ids() -> Vec<String> {
    BUILTIN_COLUMNS.iter().map(|b| b.id.to_string()).collect()
}

pub(super) fn format_column_summary(columns: &[ColumnSpec]) -> String {
    columns
        .iter()
        .map(|c| c.label.as_str())
        .collect::<Vec<_>>()
        .join(", ")
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

fn render_key(issue: &Issue) -> Cell<'static> {
    Cell::from(issue.key.clone()).style(Style::default().fg(Color::Cyan))
}
fn render_type(issue: &Issue) -> Cell<'static> {
    Cell::from(issue.issue_type.clone())
}
fn render_priority(issue: &Issue) -> Cell<'static> {
    Cell::from(issue.priority.clone().unwrap_or_else(|| "-".into()))
}
fn render_status(issue: &Issue) -> Cell<'static> {
    Cell::from(issue.status.clone()).style(Style::default().fg(status_color(&issue.status)))
}
fn render_assignee(issue: &Issue) -> Cell<'static> {
    Cell::from(issue.assignee.clone().unwrap_or_else(|| "-".into()))
}
fn render_reporter(issue: &Issue) -> Cell<'static> {
    Cell::from(issue.reporter.clone().unwrap_or_else(|| "-".into()))
}
fn render_project(issue: &Issue) -> Cell<'static> {
    Cell::from(issue.project_key.clone())
}
fn render_created(issue: &Issue) -> Cell<'static> {
    Cell::from(
        issue
            .created
            .get(..10)
            .unwrap_or(&issue.created)
            .to_string(),
    )
    .style(Style::default().fg(Color::DarkGray))
}
fn render_updated(issue: &Issue) -> Cell<'static> {
    Cell::from(
        issue
            .updated
            .get(..10)
            .unwrap_or(&issue.updated)
            .to_string(),
    )
    .style(Style::default().fg(Color::DarkGray))
}
fn render_labels(issue: &Issue) -> Cell<'static> {
    Cell::from(join_string_array(issue, "labels"))
}
fn render_components(issue: &Issue) -> Cell<'static> {
    Cell::from(join_named_array(issue, "components"))
}
fn render_fix_versions(issue: &Issue) -> Cell<'static> {
    Cell::from(join_named_array(issue, "fixVersions"))
}
fn render_summary(issue: &Issue) -> Cell<'static> {
    Cell::from(issue.summary.clone())
}

fn join_string_array(issue: &Issue, field: &str) -> String {
    issue
        .fields
        .get(field)
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "-".into())
}

fn join_named_array(issue: &Issue, field: &str) -> String {
    issue
        .fields
        .get(field)
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("name").and_then(|v| v.as_str()))
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "-".into())
}

fn render_generic_field(issue: &Issue, field_id: &str) -> Cell<'static> {
    let value = issue.fields.get(field_id);
    Cell::from(json_to_display(value))
}

fn json_to_display(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => "-".into(),
        Some(Value::String(s)) if s.is_empty() => "-".into(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Array(arr)) => {
            let parts: Vec<String> = arr.iter().map(json_scalar).collect();
            let joined = parts.join(", ");
            if joined.is_empty() {
                "-".into()
            } else {
                joined
            }
        }
        Some(Value::Object(obj)) => {
            for key in ["displayName", "name", "value", "key", "emailAddress"] {
                if let Some(v) = obj.get(key).and_then(|v| v.as_str()) {
                    if !v.is_empty() {
                        return v.to_string();
                    }
                }
            }
            "-".into()
        }
    }
}

fn json_scalar(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Object(obj) => {
            for key in ["displayName", "name", "value", "key", "emailAddress"] {
                if let Some(v) = obj.get(key).and_then(|v| v.as_str()) {
                    if !v.is_empty() {
                        return v.to_string();
                    }
                }
            }
            String::new()
        }
        _ => String::new(),
    }
}
