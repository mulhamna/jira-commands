/// Issue type metadata (id + name) returned by createmeta.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct IssueType {
    pub id: String,
    pub name: String,
}
