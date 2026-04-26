use std::{collections::HashSet, path::PathBuf};

use anyhow::{Context, Result};
use jira_core::config::config_file_path;

use super::column::{ColumnKind, AVAILABLE_COLUMNS};
use super::theme::ThemeName;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct SavedJql {
    pub(super) name: String,
    pub(super) jql: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct TuiPreferences {
    pub(super) visible_columns: Vec<ColumnKind>,
    #[serde(default)]
    pub(super) saved_jqls: Vec<SavedJql>,
    #[serde(default)]
    pub(super) theme: ThemeName,
}

impl Default for TuiPreferences {
    fn default() -> Self {
        Self {
            visible_columns: AVAILABLE_COLUMNS.to_vec(),
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
        self.visible_columns.retain(|c| seen.insert(*c));
        self.visible_columns
            .retain(|c| AVAILABLE_COLUMNS.contains(c));
        if self.visible_columns.is_empty() {
            self.visible_columns = AVAILABLE_COLUMNS.to_vec();
        }
        if !self.visible_columns.contains(&ColumnKind::Summary) {
            self.visible_columns.push(ColumnKind::Summary);
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
