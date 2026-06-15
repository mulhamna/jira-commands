use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    pub id: u64,
    pub name: String,
    /// e.g. `"scrum"`, `"kanban"`, `"simple"`.
    #[serde(rename = "type")]
    pub board_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<u64>,
    #[serde(rename = "self", default, skip_serializing_if = "String::is_empty")]
    pub self_url: String,
}

impl Board {
    /// Parse from the raw Agile API board JSON object.
    pub fn from_value(v: &Value) -> Option<Self> {
        let location = v.get("location");
        Some(Board {
            id: v.get("id").and_then(|x| x.as_u64())?,
            name: v.get("name").and_then(|x| x.as_str())?.to_string(),
            board_type: v
                .get("type")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            project_key: location
                .and_then(|l| l.get("projectKey"))
                .and_then(|x| x.as_str())
                .map(|s| s.to_string()),
            project_id: location
                .and_then(|l| l.get("projectId"))
                .and_then(|x| x.as_u64()),
            self_url: v
                .get("self")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_minimal_board() {
        let v = json!({
            "id": 7,
            "name": "ABC scrum",
            "type": "scrum",
            "self": "https://x/rest/agile/1.0/board/7"
        });
        let b = Board::from_value(&v).unwrap();
        assert_eq!(b.id, 7);
        assert_eq!(b.name, "ABC scrum");
        assert_eq!(b.board_type, "scrum");
        assert_eq!(b.project_key, None);
        assert_eq!(b.self_url, "https://x/rest/agile/1.0/board/7");
    }

    #[test]
    fn parses_board_with_location() {
        let v = json!({
            "id": 12,
            "name": "Team board",
            "type": "kanban",
            "location": { "projectKey": "ABC", "projectId": 10000 }
        });
        let b = Board::from_value(&v).unwrap();
        assert_eq!(b.project_key.as_deref(), Some("ABC"));
        assert_eq!(b.project_id, Some(10000));
    }

    #[test]
    fn missing_required_fields_returns_none() {
        assert!(Board::from_value(&json!({})).is_none());
        assert!(Board::from_value(&json!({ "id": 1 })).is_none());
    }
}
