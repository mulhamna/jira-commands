use serde_json::{json, Value};

use crate::error::AppResult;

use super::JiraApp;

impl JiraApp {
    pub async fn plan_list(&self) -> AppResult<Value> {
        let client = self.build_client()?;
        let plans = client.get_plans().await?;
        Ok(json!({ "plans": plans }))
    }
}
