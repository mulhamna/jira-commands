use serde_json::{json, Value};

use crate::{
    error::{AppError, AppResult},
    models::{CommentAddArgs, CommentDeleteArgs, CommentUpdateArgs, IssueKeyArgs},
};

use super::{shared::to_value, JiraApp};

impl JiraApp {
    pub async fn comment_list(&self, args: IssueKeyArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let comments = client.get_comments(&args.key).await?;
        Ok(json!({
            "key": args.key,
            "comments": comments
        }))
    }

    pub async fn comment_add(&self, args: CommentAddArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let comment = client.add_comment(&args.key, &args.body).await?;
        to_value(comment)
    }

    pub async fn comment_update(&self, args: CommentUpdateArgs) -> AppResult<Value> {
        if args.body.trim().is_empty() {
            return Err(AppError::validation("Comment body cannot be empty"));
        }
        let client = self.build_client()?;
        let comment = client
            .update_comment(&args.key, &args.comment_id, &args.body)
            .await?;
        to_value(comment)
    }

    pub async fn comment_delete(&self, args: CommentDeleteArgs) -> AppResult<Value> {
        super::shared::require_confirm(args.confirm)?;
        let client = self.build_client()?;
        client.delete_comment(&args.key, &args.comment_id).await?;
        Ok(json!({
            "key": args.key,
            "comment_id": args.comment_id,
            "deleted": true
        }))
    }
}
