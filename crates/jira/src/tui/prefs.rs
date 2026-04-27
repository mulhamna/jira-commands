use std::{collections::HashSet, path::PathBuf};

use anyhow::{Context, Result};
use jira_core::config::config_file_path;
use serde_json::Value;

use super::column::{default_column_ids, BUILTIN_COLUMNS};
use super::theme::ThemeName;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct SavedJql {
    pub(super) name: String,
    pub(super) jql: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct TuiPreferences {
    #[serde(deserialize_with = "deserialize_column_ids")]
    pub(super) visible_columns: Vec<String>,
    #[serde(default)]
    pub(super) saved_jqls: Vec<SavedJql>,
    #[serde(default)]
    pub(super) theme: ThemeName,
}

impl Default for TuiPreferences {
    fn default() -> Self {
        Self {
            visible_columns: default_column_ids(),
            saved_jqls: vec![
                SavedJql {
                    name: "My open issues".into(),
                    jql:
                        "assignee = currentUser() AND resolution = Unresolved ORDER BY updated DESC"
                            .into(),
                },
                SavedJql {
                    name: "Updated this week".into(),
                    jql: "updated >= -7d ORDER BY updated DESC".into(),
                },
                SavedJql {
                    name: "Recently created".into(),
                    jql: "created >= -7d ORDER BY created DESC".into(),
                },
            ],
            theme: ThemeName::Default,
        }
    }
}

impl TuiPreferences {
    pub(super) fn load() -> Self {
        let path = tui_preferences_path();
        std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str::<Self>(&raw).ok())
            .map(|mut prefs| {
                prefs.normalize();
                prefs
            })
            .unwrap_or_default()
    }

    pub(super) fn save(&self) -> Result<()> {
        let path = tui_preferences_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .context("failed to create TUI preferences directory")?;
        }
        let payload =
            serde_json::to_string_pretty(self).context("failed to serialize TUI preferences")?;
        std::fs::write(&path, payload)
            .with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }

    pub(super) fn normalize(&mut self) {
        let mut seen = HashSet::new();
        self.visible_columns.retain(|c| seen.insert(c.clone()));
        self.visible_columns.retain(|c| !c.trim().is_empty());
        if self.visible_columns.is_empty() {
            self.visible_columns = default_column_ids();
        }
        // Always keep summary visible — it's the main display column.
        if !self.visible_columns.iter().any(|c| c == "summary")
            && BUILTIN_COLUMNS.iter().any(|b| b.id == "summary")
        {
            self.visible_columns.push("summary".to_string());
        }

        self.saved_jqls
            .retain(|saved| !saved.name.trim().is_empty() && !saved.jql.trim().is_empty());
    }
}

fn tui_preferences_path() -> PathBuf {
    let mut path = config_file_path();
    path.set_file_name("tui-preferences.json");
    path
}

/// Deserialize column IDs, accepting both new string IDs and legacy enum variants
/// (e.g. `"Key"` → `"key"`, `"FixVersions"` → `"fixVersions"`, `"Type"` → `"issuetype"`).
fn deserialize_column_ids<'de, D>(deserializer: D) -> std::result::Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let raw = Vec::<Value>::deserialize(deserializer)?;
    Ok(raw.into_iter().filter_map(legacy_to_id).collect())
}

fn legacy_to_id(v: Value) -> Option<String> {
    let s = v.as_str()?.to_string();
    let mapped = match s.as_str() {
        "Key" => "key",
        "Type" => "issuetype",
        "Priority" => "priority",
        "Status" => "status",
        "Assignee" => "assignee",
        "Reporter" => "reporter",
        "Project" => "project",
        "Created" => "created",
        "Updated" => "updated",
        "Labels" => "labels",
        "Components" => "components",
        "FixVersions" => "fixVersions",
        "Summary" => "summary",
        other => other,
    };
    Some(mapped.to_string())
}
