use serde_json::{json, Value};

use crate::{
    error::AppResult,
    models::{
        IssueKeyArgs, IssueLinkCreateArgs, IssueLinkDeleteArgs, RemoteLinkAddArgs,
        RemoteLinkDeleteArgs,
    },
};

use super::{shared::require_confirm, JiraApp};

impl JiraApp {
    pub async fn issue_link_types_list(&self) -> AppResult<Value> {
        let client = self.build_client()?;
        let link_types = client.list_issue_link_types().await?;
        Ok(json!({
            "link_types": link_types
        }))
    }

    pub async fn issue_link_create(&self, args: IssueLinkCreateArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        client
            .link_issues(
                &args.outward_key,
                &args.inward_key,
                &args.link_type,
                args.comment.as_deref(),
            )
            .await?;
        Ok(json!({
            "outward_key": args.outward_key,
            "inward_key": args.inward_key,
            "link_type": args.link_type,
            "created": true
        }))
    }

    pub async fn issue_link_delete(&self, args: IssueLinkDeleteArgs) -> AppResult<Value> {
        require_confirm(args.confirm)?;
        let client = self.build_client()?;
        client.delete_issue_link(&args.link_id).await?;
        Ok(json!({
            "link_id": args.link_id,
            "deleted": true
        }))
    }

    pub async fn remote_link_list(&self, args: IssueKeyArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let links = client.get_remote_links(&args.key).await?;
        Ok(json!({
            "key": args.key,
            "remote_links": links
        }))
    }

    pub async fn remote_link_add(&self, args: RemoteLinkAddArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let link = client
            .add_remote_link(&args.key, &args.url, &args.title)
            .await?;
        Ok(json!({
            "key": args.key,
            "remote_link": link
        }))
    }

    pub async fn remote_link_delete(&self, args: RemoteLinkDeleteArgs) -> AppResult<Value> {
        require_confirm(args.confirm)?;
        let client = self.build_client()?;
        client.delete_remote_link(&args.key, &args.link_id).await?;
        Ok(json!({
            "key": args.key,
            "link_id": args.link_id,
            "deleted": true
        }))
    }
}
