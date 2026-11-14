use serde_json::{json, Value};

use crate::{
    error::{AppError, AppResult},
    models::{IssueKeyArgs, WorklogAddArgs, WorklogDeleteArgs, WorklogUpdateArgs},
};

use super::{shared::to_value, JiraApp};

impl JiraApp {
    pub async fn worklog_list(&self, args: IssueKeyArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let worklogs = client.get_worklogs(&args.key).await?;
        Ok(json!({
            "key": args.key,
            "worklogs": worklogs
        }))
    }

    pub async fn worklog_add(&self, args: WorklogAddArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let worklog = client
            .add_worklog(
                &args.key,
                &args.time_spent,
                args.comment.as_deref(),
                args.started.as_deref(),
            )
            .await?;
        to_value(worklog)
    }

    pub async fn worklog_update(&self, args: WorklogUpdateArgs) -> AppResult<Value> {
        if args.time_spent.is_none() && args.comment.is_none() && args.started.is_none() {
            return Err(AppError::validation(
                "At least one worklog field must be provided",
            ));
        }
        let client = self.build_client()?;
        let worklog = client
            .update_worklog(
                &args.key,
                &args.id,
                args.time_spent.as_deref(),
                args.comment.as_deref(),
                args.started.as_deref(),
            )
            .await?;
        to_value(worklog)
    }

    pub async fn worklog_delete(&self, args: WorklogDeleteArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        client.delete_worklog(&args.key, &args.id).await?;
        Ok(json!({
            "key": args.key,
            "id": args.id,
            "deleted": true
        }))
    }
}
