use serde_json::{json, Value};

use crate::error::AppResult;

use super::JiraApp;

impl JiraApp {
    pub async fn whoami(&self) -> AppResult<Value> {
        let client = self.build_client()?;
        let account_id = client.get_myself().await?;
        let timezone = client.get_myself_timezone().await?;
        Ok(json!({
            "account_id": account_id,
            "timezone": timezone,
            "base_url": client.base_url(),
        }))
    }

    pub async fn server_info(&self) -> AppResult<Value> {
        let client = self.build_client()?;
        let info = client.get_server_info().await?;
        let premium = client.is_premium().await;
        Ok(json!({
            "premium": premium,
            "server_info": info,
        }))
    }
}
