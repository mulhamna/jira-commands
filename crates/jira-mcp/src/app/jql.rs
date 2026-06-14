use chrono::Duration;
use jira_core::jql::{compose_jql, JqlParams};
use serde_json::{json, Value};

use crate::{
    error::{AppError, AppResult},
    models::JqlBuildArgs,
};

use super::JiraApp;

impl JiraApp {
    pub async fn jql_build(&self, args: JqlBuildArgs) -> AppResult<Value> {
        let params: JqlParams = serde_json::from_value(args.params)
            .map_err(|e| AppError::validation(format!("Invalid JqlParams: {e}")))?;
        let jql =
            compose_jql(&params).map_err(|e| AppError::validation(format!("Bad JQL: {e}")))?;

        let mut out = json!({ "jql": jql });

        if args.dry_run.unwrap_or(false) {
            let client = self.build_client()?;
            let max = args.max_preview.unwrap_or(10).min(100);
            let result = client.search_issues(&jql, None, Some(max)).await?;
            out["preview_keys"] = json!(result
                .issues
                .iter()
                .map(|i| i.key.clone())
                .collect::<Vec<_>>());
            if let Some(total) = result.total {
                out["total"] = json!(total);
            }
        }

        Ok(out)
    }
}

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
