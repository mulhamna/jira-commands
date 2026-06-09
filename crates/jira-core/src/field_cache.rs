use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::{client::JiraClient, error::Result, model::field::Field};

const TTL: Duration = Duration::from_secs(300); // 5 minutes

struct CacheEntry {
    fields: Vec<Field>,
    fetched_at: Instant,
}

impl CacheEntry {
    fn is_fresh(&self) -> bool {
        self.fetched_at.elapsed() < TTL
    }
}

/// In-memory cache for Jira field metadata, keyed by "project:issue_type_id".
///
/// Safe to share across tokio tasks: interior mutability via `Mutex`. The lock
/// is never held across an `.await`.
pub struct FieldCache {
    entries: Mutex<HashMap<String, CacheEntry>>,
}

impl FieldCache {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Return cached fields or fetch from the API if stale / missing.
    pub async fn get_or_fetch(
        &self,
        client: &JiraClient,
        project_key: &str,
        issue_type_id: &str,
    ) -> Result<Vec<Field>> {
        let key = format!("{project_key}:{issue_type_id}");

        {
            let entries = self.entries.lock().expect("FieldCache mutex poisoned");
            if let Some(entry) = entries.get(&key) {
                if entry.is_fresh() {
                    return Ok(entry.fields.clone());
                }
            }
        }

        let fields = client
            .get_fields_for_issue_type(project_key, issue_type_id)
            .await?;

        {
            let mut entries = self.entries.lock().expect("FieldCache mutex poisoned");
            entries.insert(
                key,
                CacheEntry {
                    fields: fields.clone(),
                    fetched_at: Instant::now(),
                },
            );
        }

        Ok(fields)
    }
}

impl Default for FieldCache {
    fn default() -> Self {
        Self::new()
    }
}
