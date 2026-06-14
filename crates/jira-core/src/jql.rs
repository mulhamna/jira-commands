//! Structured JQL builder.
//!
//! Compose safe JQL queries from typed parameters. Values are escaped
//! to avoid injection through user-supplied identifiers, labels, or text.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderDir {
    Asc,
    Desc,
}

impl OrderDir {
    fn as_str(self) -> &'static str {
        match self {
            OrderDir::Asc => "ASC",
            OrderDir::Desc => "DESC",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssigneeFilter {
    CurrentUser,
    Empty,
    AccountId { account_id: String },
    Email { email: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DateExpr {
    /// Absolute date `YYYY-MM-DD`.
    Date { date: String },
    /// Relative window like `-7d`, `-2w`, `-1M`.
    Relative { window: String },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct JqlParams {
    pub project: Option<String>,
    pub status: Vec<String>,
    pub assignee: Vec<AssigneeFilter>,
    pub priority: Vec<String>,
    pub labels: Vec<String>,
    pub components: Vec<String>,
    pub fix_versions: Vec<String>,
    pub text: Option<String>,
    pub created_after: Option<DateExpr>,
    pub updated_after: Option<DateExpr>,
    pub extra_clauses: Vec<String>,
    pub order_by: Vec<(String, OrderDir)>,
}

#[derive(Debug, thiserror::Error)]
pub enum JqlBuildError {
    #[error("value contains control characters: {0:?}")]
    ControlChar(String),
    #[error("relative window must match `-?[0-9]+[dwmy]`, got {0:?}")]
    BadRelativeWindow(String),
    #[error("date must be YYYY-MM-DD, got {0:?}")]
    BadDate(String),
    #[error("field name must match `[A-Za-z0-9_.]+`, got {0:?}")]
    BadFieldName(String),
}

/// Escape a value for use inside a double-quoted JQL literal.
///
/// Replaces `\` → `\\` and `"` → `\"`. Rejects control characters
/// (any char with `is_control()` other than tab) to avoid breaking
/// downstream parsers or smuggling newlines.
pub fn escape_jql_literal(value: &str) -> Result<String, JqlBuildError> {
    if value.chars().any(|c| c.is_control() && c != '\t') {
        return Err(JqlBuildError::ControlChar(value.to_string()));
    }
    Ok(value.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Validate a JQL field name (used for ORDER BY and custom fields).
fn validate_field_name(name: &str) -> Result<&str, JqlBuildError> {
    let trimmed = name.trim();
    if trimmed.is_empty()
        || !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
    {
        return Err(JqlBuildError::BadFieldName(name.to_string()));
    }
    Ok(trimmed)
}

fn validate_relative_window(s: &str) -> Result<&str, JqlBuildError> {
    let trimmed = s.trim();
    let mut chars = trimmed.chars().peekable();
    if let Some('-') = chars.peek() {
        chars.next();
    }
    let mut digits = String::new();
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            digits.push(c);
            chars.next();
        } else {
            break;
        }
    }
    let unit: String = chars.collect();
    if digits.is_empty() || !matches!(unit.as_str(), "d" | "w" | "m" | "M" | "y") {
        return Err(JqlBuildError::BadRelativeWindow(s.to_string()));
    }
    Ok(trimmed)
}

fn validate_date(s: &str) -> Result<&str, JqlBuildError> {
    let trimmed = s.trim();
    let parts: Vec<&str> = trimmed.split('-').collect();
    let lens_ok =
        parts.len() == 3 && parts[0].len() == 4 && parts[1].len() == 2 && parts[2].len() == 2;
    let digits_ok = parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit()));
    if !lens_ok || !digits_ok {
        return Err(JqlBuildError::BadDate(s.to_string()));
    }
    Ok(trimmed)
}

fn or_clause(field: &str, values: &[String]) -> Result<Option<String>, JqlBuildError> {
    if values.is_empty() {
        return Ok(None);
    }
    let mut quoted = Vec::with_capacity(values.len());
    for v in values {
        quoted.push(format!("\"{}\"", escape_jql_literal(v)?));
    }
    Ok(Some(format!("{} in ({})", field, quoted.join(", "))))
}

fn assignee_clause(filters: &[AssigneeFilter]) -> Result<Option<String>, JqlBuildError> {
    if filters.is_empty() {
        return Ok(None);
    }
    let mut parts = Vec::with_capacity(filters.len());
    for f in filters {
        match f {
            AssigneeFilter::CurrentUser => parts.push("assignee = currentUser()".to_string()),
            AssigneeFilter::Empty => parts.push("assignee is EMPTY".to_string()),
            AssigneeFilter::AccountId { account_id } => parts.push(format!(
                "assignee = \"{}\"",
                escape_jql_literal(account_id)?
            )),
            AssigneeFilter::Email { email } => {
                parts.push(format!("assignee = \"{}\"", escape_jql_literal(email)?))
            }
        }
    }
    Ok(Some(if parts.len() == 1 {
        parts.remove(0)
    } else {
        format!("({})", parts.join(" OR "))
    }))
}

fn date_clause(field: &str, expr: &DateExpr) -> Result<String, JqlBuildError> {
    let rhs = match expr {
        DateExpr::Date { date } => format!("\"{}\"", validate_date(date)?),
        DateExpr::Relative { window } => format!("\"{}\"", validate_relative_window(window)?),
    };
    Ok(format!("{} >= {}", field, rhs))
}

/// Compose a JQL query from structured parameters.
///
/// Returns `Ok("")` if every input is empty (caller may treat as "no filter").
pub fn compose_jql(params: &JqlParams) -> Result<String, JqlBuildError> {
    let mut clauses: Vec<String> = Vec::new();

    if let Some(project) = &params.project {
        clauses.push(format!(
            "project = \"{}\"",
            escape_jql_literal(project.trim())?
        ));
    }
    if let Some(c) = or_clause("status", &params.status)? {
        clauses.push(c);
    }
    if let Some(c) = assignee_clause(&params.assignee)? {
        clauses.push(c);
    }
    if let Some(c) = or_clause("priority", &params.priority)? {
        clauses.push(c);
    }
    if let Some(c) = or_clause("labels", &params.labels)? {
        clauses.push(c);
    }
    if let Some(c) = or_clause("component", &params.components)? {
        clauses.push(c);
    }
    if let Some(c) = or_clause("fixVersion", &params.fix_versions)? {
        clauses.push(c);
    }
    if let Some(text) = &params.text {
        clauses.push(format!("text ~ \"{}\"", escape_jql_literal(text)?));
    }
    if let Some(expr) = &params.created_after {
        clauses.push(date_clause("created", expr)?);
    }
    if let Some(expr) = &params.updated_after {
        clauses.push(date_clause("updated", expr)?);
    }
    for extra in &params.extra_clauses {
        let trimmed = extra.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.chars().any(|c| c.is_control() && c != '\t') {
            return Err(JqlBuildError::ControlChar(extra.clone()));
        }
        clauses.push(format!("({})", trimmed));
    }

    let mut jql = clauses.join(" AND ");

    if !params.order_by.is_empty() {
        let mut order_parts = Vec::with_capacity(params.order_by.len());
        for (field, dir) in &params.order_by {
            let name = validate_field_name(field)?;
            order_parts.push(format!("{} {}", name, dir.as_str()));
        }
        if !jql.is_empty() {
            jql.push(' ');
        }
        jql.push_str("ORDER BY ");
        jql.push_str(&order_parts.join(", "));
    }

    Ok(jql)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_basic() {
        assert_eq!(escape_jql_literal("plain").unwrap(), "plain");
        assert_eq!(escape_jql_literal("a\"b").unwrap(), "a\\\"b");
        assert_eq!(escape_jql_literal("a\\b").unwrap(), "a\\\\b");
        assert_eq!(escape_jql_literal("O'Reilly").unwrap(), "O'Reilly");
    }

    #[test]
    fn escape_rejects_control_chars() {
        assert!(escape_jql_literal("line1\nline2").is_err());
        assert!(escape_jql_literal("bell\x07").is_err());
        assert!(escape_jql_literal("tab\there").is_ok());
    }

    #[test]
    fn empty_params_returns_empty() {
        let jql = compose_jql(&JqlParams::default()).unwrap();
        assert_eq!(jql, "");
    }

    #[test]
    fn project_and_status() {
        let p = JqlParams {
            project: Some("ABC".into()),
            status: vec!["In Progress".into(), "Done".into()],
            ..Default::default()
        };
        let jql = compose_jql(&p).unwrap();
        assert_eq!(
            jql,
            r#"project = "ABC" AND status in ("In Progress", "Done")"#
        );
    }

    #[test]
    fn assignee_mixed() {
        let p = JqlParams {
            assignee: vec![
                AssigneeFilter::CurrentUser,
                AssigneeFilter::Email {
                    email: "u@x.com".into(),
                },
            ],
            ..Default::default()
        };
        let jql = compose_jql(&p).unwrap();
        assert_eq!(jql, r#"(assignee = currentUser() OR assignee = "u@x.com")"#);
    }

    #[test]
    fn assignee_empty_clause() {
        let p = JqlParams {
            assignee: vec![AssigneeFilter::Empty],
            ..Default::default()
        };
        assert_eq!(compose_jql(&p).unwrap(), "assignee is EMPTY");
    }

    #[test]
    fn text_search_escapes_quotes() {
        let p = JqlParams {
            text: Some(r#"He said "hi""#.into()),
            ..Default::default()
        };
        let jql = compose_jql(&p).unwrap();
        assert_eq!(jql, r#"text ~ "He said \"hi\"""#);
    }

    #[test]
    fn dates_absolute_and_relative() {
        let p = JqlParams {
            created_after: Some(DateExpr::Date {
                date: "2026-01-01".into(),
            }),
            updated_after: Some(DateExpr::Relative {
                window: "-7d".into(),
            }),
            ..Default::default()
        };
        let jql = compose_jql(&p).unwrap();
        assert_eq!(jql, r#"created >= "2026-01-01" AND updated >= "-7d""#);
    }

    #[test]
    fn bad_date_rejected() {
        let p = JqlParams {
            created_after: Some(DateExpr::Date {
                date: "2026/01/01".into(),
            }),
            ..Default::default()
        };
        assert!(matches!(compose_jql(&p), Err(JqlBuildError::BadDate(_))));
    }

    #[test]
    fn bad_relative_window_rejected() {
        for bad in ["7", "-7x", "abc", ""] {
            let p = JqlParams {
                updated_after: Some(DateExpr::Relative { window: bad.into() }),
                ..Default::default()
            };
            assert!(
                matches!(compose_jql(&p), Err(JqlBuildError::BadRelativeWindow(_))),
                "should reject {bad:?}"
            );
        }
    }

    #[test]
    fn order_by_validated() {
        let p = JqlParams {
            project: Some("ABC".into()),
            order_by: vec![
                ("priority".into(), OrderDir::Desc),
                ("created".into(), OrderDir::Asc),
            ],
            ..Default::default()
        };
        let jql = compose_jql(&p).unwrap();
        assert_eq!(
            jql,
            r#"project = "ABC" ORDER BY priority DESC, created ASC"#
        );
    }

    #[test]
    fn order_by_rejects_bad_field() {
        let p = JqlParams {
            order_by: vec![("priority; DROP".into(), OrderDir::Asc)],
            ..Default::default()
        };
        assert!(matches!(
            compose_jql(&p),
            Err(JqlBuildError::BadFieldName(_))
        ));
    }

    #[test]
    fn extra_clauses_wrapped() {
        let p = JqlParams {
            project: Some("ABC".into()),
            extra_clauses: vec!["sprint in openSprints()".into()],
            ..Default::default()
        };
        let jql = compose_jql(&p).unwrap();
        assert_eq!(jql, r#"project = "ABC" AND (sprint in openSprints())"#);
    }

    #[test]
    fn extra_clauses_reject_control() {
        let p = JqlParams {
            extra_clauses: vec!["foo\nbar".into()],
            ..Default::default()
        };
        assert!(matches!(
            compose_jql(&p),
            Err(JqlBuildError::ControlChar(_))
        ));
    }

    #[test]
    fn labels_components_versions() {
        let p = JqlParams {
            labels: vec!["needs-review".into()],
            components: vec!["api".into(), "ui".into()],
            fix_versions: vec!["1.2.0".into()],
            ..Default::default()
        };
        let jql = compose_jql(&p).unwrap();
        assert_eq!(
            jql,
            r#"labels in ("needs-review") AND component in ("api", "ui") AND fixVersion in ("1.2.0")"#
        );
    }
}
