use serde_json::Value;

use crate::{
    error::AppResult,
    models::{BoardGetArgs, BoardIssuesArgs, BoardListArgs},
};

use super::{shared::to_value, JiraApp};

impl JiraApp {
    pub async fn board_list(&self, args: BoardListArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let boards = client
            .list_boards(args.project_key.as_deref(), args.board_type.as_deref())
            .await?;
        to_value(boards)
    }

    pub async fn board_get(&self, args: BoardGetArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let board = client.get_board(args.board_id).await?;
        to_value(board)
    }

    pub async fn board_issues(&self, args: BoardIssuesArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let issues = client
            .board_issues(args.board_id, args.jql.as_deref(), args.max_results)
            .await?;
        to_value(issues)
    }

    pub async fn board_backlog(&self, args: BoardIssuesArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let issues = client
            .board_backlog(args.board_id, args.jql.as_deref(), args.max_results)
            .await?;
        to_value(issues)
    }
}
