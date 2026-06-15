use serde_json::{json, Value};

use crate::{error::AppResult, models::ApiRequestArgs};

use super::{
    shared::{build_api_path, normalize_method},
    JiraApp,
};

impl JiraApp {
    pub async fn api_request(&self, args: ApiRequestArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let method = normalize_method(&args.method)?;
        let path = build_api_path(args.path, args.query)?;
        let body = client.raw_request(&method, &path, args.body).await?;
        Ok(json!({
            "method": method,
            "path": path,
            "body": body
        }))
    }
}
