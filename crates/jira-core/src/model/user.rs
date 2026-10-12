/// A Jira user, returned by `GET /rest/api/3/user/search` and similar endpoints.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JiraUser {
    /// Cloud identifies users by accountId; Data Center / Server do not return it at all.
    #[serde(default)]
    pub account_id: Option<String>,
    /// Data Center / Server login name (also used as the assignee/watcher identifier there).
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub email_address: Option<String>,
    #[serde(default)]
    pub active: Option<bool>,
    #[serde(default)]
    pub account_type: Option<String>,
}

impl JiraUser {
    /// Identifier accepted by the API: accountId where present (Cloud), otherwise the
    /// login name / key (Data Center, Server).
    pub fn identifier(&self) -> Option<String> {
        self.account_id
            .clone()
            .or_else(|| self.name.clone())
            .or_else(|| self.key.clone())
    }
}
