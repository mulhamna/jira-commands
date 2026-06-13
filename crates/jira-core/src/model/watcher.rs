use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Watcher {
    pub self_url: String,
    pub account_id: String,
    pub display_name: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Watchers {
    pub issue_key: String,
    pub is_watching: bool,
    pub watch_count: u64,
    pub watchers: Vec<Watcher>,
}

impl Watcher {
    pub fn from_value(v: &Value) -> Option<Self> {
        Some(Watcher {
            self_url: v
                .get("self")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string(),
            account_id: v.get("accountId")?.as_str()?.to_string(),
            display_name: v
                .get("displayName")
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string(),
            active: v.get("active").and_then(|a| a.as_bool()).unwrap_or(true),
        })
    }
}

impl Watchers {
    pub fn from_value(v: &Value, issue_key: &str) -> Option<Self> {
        Some(Watchers {
            issue_key: issue_key.to_string(),
            is_watching: v
                .get("isWatching")
                .and_then(|b| b.as_bool())
                .unwrap_or(false),
            watch_count: v.get("watchCount").and_then(|c| c.as_u64()).unwrap_or(0),
            watchers: v
                .get("watchers")
                .and_then(|w| w.as_array())
                .map(|arr| arr.iter().filter_map(Watcher::from_value).collect())
                .unwrap_or_default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_watcher() {
        let v = json!({
            "self": "https://x.atlassian.net/rest/api/3/user?accountId=abc",
            "accountId": "abc",
            "displayName": "Alice",
            "active": true
        });
        let w = Watcher::from_value(&v).unwrap();
        assert_eq!(w.account_id, "abc");
        assert_eq!(w.display_name, "Alice");
        assert!(w.active);
    }

    #[test]
    fn parses_watchers_envelope() {
        let v = json!({
            "isWatching": true,
            "watchCount": 2,
            "watchers": [
                {"self": "", "accountId": "a", "displayName": "Alice", "active": true},
                {"self": "", "accountId": "b", "displayName": "Bob", "active": false}
            ]
        });
        let ws = Watchers::from_value(&v, "ABC-1").unwrap();
        assert_eq!(ws.issue_key, "ABC-1");
        assert!(ws.is_watching);
        assert_eq!(ws.watch_count, 2);
        assert_eq!(ws.watchers.len(), 2);
        assert_eq!(ws.watchers[1].account_id, "b");
        assert!(!ws.watchers[1].active);
    }

    #[test]
    fn parses_empty_watchers() {
        let v = json!({"isWatching": false, "watchCount": 0, "watchers": []});
        let ws = Watchers::from_value(&v, "ABC-2").unwrap();
        assert_eq!(ws.watch_count, 0);
        assert!(ws.watchers.is_empty());
    }
}
