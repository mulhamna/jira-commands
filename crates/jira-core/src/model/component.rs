/// A project component, returned by `GET /rest/api/3/project/{key}/components`.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Component {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, rename = "self")]
    pub self_url: Option<String>,
}
