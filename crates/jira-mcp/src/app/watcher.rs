use serde_json::{json, Value};

use crate::{
    error::AppResult,
    models::{IssueKeyArgs, WatcherAddArgs, WatcherRemoveArgs},
};

use super::{
    shared::{require_confirm, to_value},
    JiraApp,
};

impl JiraApp {
    pub async fn watcher_list(&self, args: IssueKeyArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let watchers = client.list_watchers(&args.key).await?;
        to_value(watchers)
    }

    pub async fn watcher_add(&self, args: WatcherAddArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let account_id = match args.account_id {
            Some(id) if !id.trim().is_empty() => id,
            _ => client.get_myself().await?,
        };
        client.add_watcher(&args.issue_key, &account_id).await?;
        Ok(json!({
            "ok": true,
            "issue_key": args.issue_key,
            "account_id": account_id,
        }))
    }

    pub async fn watcher_remove(&self, args: WatcherRemoveArgs) -> AppResult<Value> {
        require_confirm(args.confirm)?;
        let client = self.build_client()?;
        client
            .remove_watcher(&args.issue_key, &args.account_id)
            .await?;
        Ok(json!({
            "ok": true,
            "issue_key": args.issue_key,
            "account_id": args.account_id,
        }))
    }
}
