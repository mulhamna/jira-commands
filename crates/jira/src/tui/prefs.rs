use std::{collections::HashSet, path::PathBuf};

use anyhow::{Context, Result};
use jira_core::config::config_file_path;

use super::column::{ColumnKind, AVAILABLE_COLUMNS};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct TuiPreferences {
    pub(super) visible_columns: Vec<ColumnKind>,
}

impl Default for TuiPreferences {
    fn default() -> Self {
        Self {
            visible_columns: AVAILABLE_COLUMNS.to_vec(),
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
    }
}

fn tui_preferences_path() -> PathBuf {
    let mut path = config_file_path();
    path.set_file_name("tui-preferences.json");
    path
}
