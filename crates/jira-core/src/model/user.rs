/// A Jira user, returned by `GET /rest/api/3/user/search` and similar endpoints.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JiraUser {
    pub account_id: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub email_address: Option<String>,
    #[serde(default)]
    pub active: Option<bool>,
    #[serde(default)]
    pub account_type: Option<String>,
}
