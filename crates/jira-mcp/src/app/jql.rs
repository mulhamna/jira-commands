use chrono::Duration;

use crate::error::{AppError, AppResult};

pub(super) fn escape_jql_literal(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

pub(super) fn parse_relative_window(raw: &str) -> AppResult<Duration> {
    let value = raw.trim().to_lowercase();
    if value.len() < 2 {
        return Err(AppError::validation(format!(
            "Invalid relative window '{}'. Use values like 2d, 36h, or 1w.",
            raw
        )));
    }

    let (num, unit) = value.split_at(value.len() - 1);
    let amount: i64 = num.parse().map_err(|_| {
        AppError::validation(format!(
            "Invalid relative window '{}'. Use values like 2d, 36h, or 1w.",
            raw
        ))
    })?;

    match unit {
        "h" => Ok(Duration::hours(amount)),
        "d" => Ok(Duration::days(amount)),
        "w" => Ok(Duration::weeks(amount)),
        _ => Err(AppError::validation(format!(
            "Invalid relative window '{}'. Use values like 2d, 36h, or 1w.",
            raw
        ))),
    }
}

pub(super) fn build_notifications_jql(project: Option<&str>, since: &str) -> String {
    let since = since.trim();
    if let Some(project) = project {
        format!("project = {project} AND updated >= -{since} ORDER BY updated DESC")
    } else {
        format!("updated >= -{since} ORDER BY updated DESC")
    }
}
