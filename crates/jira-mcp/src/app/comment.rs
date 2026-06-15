use serde_json::{json, Value};

use crate::{
    error::AppResult,
    models::{CommentAddArgs, IssueKeyArgs},
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
}
