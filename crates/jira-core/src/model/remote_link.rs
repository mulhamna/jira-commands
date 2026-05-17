/// A remote link attached to a Jira issue.
///
/// Returned by `GET /rest/api/3/issue/{key}/remotelink`. Only the fields
/// consumed by this crate's callers are typed; the full Jira payload
/// includes additional optional metadata (icon, status, application).
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteLink {
    pub id: i64,
    #[serde(rename = "self", default)]
    pub self_url: Option<String>,
    #[serde(default)]
    pub global_id: Option<String>,
    #[serde(default)]
    pub relationship: Option<String>,
    pub object: RemoteLinkObject,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct RemoteLinkObject {
    pub title: String,
    pub url: String,
    #[serde(default)]
    pub summary: Option<String>,
}
