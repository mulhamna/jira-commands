use serde_json::{json, Value};

use crate::{
    error::{AppError, AppResult},
    models::{
        SprintAddIssueArgs, SprintCreateArgs, SprintDeleteArgs, SprintListArgs, SprintUpdateArgs,
    },
};

use super::{
    shared::{require_confirm, to_value},
    JiraApp,
};

impl JiraApp {
    pub async fn sprint_list(&self, args: SprintListArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let sprints = if let Some(states) = args.states.as_ref().filter(|states| !states.is_empty())
        {
            let normalized: Vec<String> = states
                .iter()
                .map(|state| normalize_sprint_state(state))
                .collect::<AppResult<_>>()?;
            let state_refs: Vec<&str> = normalized.iter().map(String::as_str).collect();
            client
                .list_sprints_for_project_with_states(&args.project_key, &state_refs)
                .await?
        } else {
            client.list_sprints_for_project(&args.project_key).await?
        };

        Ok(json!({
            "project_key": args.project_key,
            "sprints": sprints
        }))
    }

    pub async fn sprint_create(&self, args: SprintCreateArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let sprint = client
            .create_sprint(
                args.board_id,
                &args.name,
                args.start_date.as_deref(),
                args.end_date.as_deref(),
                args.goal.as_deref(),
            )
            .await?;
        to_value(sprint)
    }

    pub async fn sprint_update(&self, args: SprintUpdateArgs) -> AppResult<Value> {
        let has_changes = args.name.is_some()
            || args.state.is_some()
            || args.start_date.is_some()
            || args.end_date.is_some()
            || args.goal.is_some();
        if !has_changes {
            return Err(AppError::validation(
                "Provide at least one sprint field to update",
            ));
        }

        let mut body = serde_json::Map::new();
        if let Some(name) = args.name {
            body.insert("name".into(), Value::String(name));
        }
        if let Some(state) = args.state {
            body.insert(
                "state".into(),
                Value::String(normalize_sprint_state(&state)?),
            );
        }
        if let Some(start_date) = args.start_date {
            body.insert("startDate".into(), Value::String(start_date));
        }
        if let Some(end_date) = args.end_date {
            body.insert("endDate".into(), Value::String(end_date));
        }
        if let Some(goal) = args.goal {
            body.insert("goal".into(), Value::String(goal));
        }

        let client = self.build_client()?;
        let sprint = client
            .update_sprint(args.sprint_id, Value::Object(body))
            .await?;
        to_value(sprint)
    }

    pub async fn sprint_delete(&self, args: SprintDeleteArgs) -> AppResult<Value> {
        require_confirm(args.confirm)?;
        let client = self.build_client()?;
        client.delete_sprint(args.sprint_id).await?;
        Ok(json!({
            "sprint_id": args.sprint_id,
            "deleted": true
        }))
    }

    pub async fn sprint_add_issue(&self, args: SprintAddIssueArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        client
            .add_issue_to_sprint(args.sprint_id, &args.issue_key)
            .await?;
        Ok(json!({
            "sprint_id": args.sprint_id,
            "issue_key": args.issue_key,
            "added": true
        }))
    }
}

pub(super) fn normalize_sprint_state(state: &str) -> AppResult<String> {
    let normalized = state.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "active" | "future" | "closed" => Ok(normalized),
        _ => Err(AppError::validation(
            "sprint state must be one of active, future, or closed",
        )),
    }
}
