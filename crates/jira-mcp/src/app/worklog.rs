use serde_json::{json, Value};

use crate::{
    error::AppResult,
    models::{IssueKeyArgs, WorklogAddArgs, WorklogDeleteArgs},
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
