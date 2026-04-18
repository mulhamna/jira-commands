use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use crate::error::{JiraError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraConfig {
    pub base_url: String,
    pub email: String,
    pub token: Option<String>,
    pub project: Option<String>,
    pub timeout_secs: u64,
}

impl Default for JiraConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            email: String::new(),
            token: None,
            project: None,
            timeout_secs: 30,
        }
    }
}

impl JiraConfig {
    /// Load config from ~/.config/jira/config.toml and env vars.
    /// Returns default config if file doesn't exist.
    pub fn load() -> Result<Self> {
        let config_path = config_file_path();

        let mut figment = Figment::from(Serialized::defaults(JiraConfig::default()));

        if config_path.exists() {
            figment = figment.merge(Toml::file(&config_path));
        }

        figment = figment.merge(Env::prefixed("JIRA_").map(|key| {
            let normalized = key.as_str().to_ascii_lowercase();
            match normalized.as_str() {
                "url" => "base_url".into(),
                "email" => "email".into(),
                "token" => "token".into(),
                other => other.to_string().into(),
            }
        }));

        figment
            .extract()
            .map_err(|e| JiraError::Config(e.to_string()))
    }

    /// Save config to ~/.config/jira/config.toml
    pub fn save(&self) -> Result<()> {
        let config_path = config_file_path();
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| JiraError::Config(format!("Failed to create config dir: {e}")))?;
        }

        let toml_str = toml::to_string_pretty(self)
            .map_err(|e| JiraError::Config(format!("Failed to serialize config: {e}")))?;

        std::fs::write(&config_path, toml_str)
            .map_err(|e| JiraError::Config(format!("Failed to write config: {e}")))?;

        // Restrict permissions to owner-only (rw-------) on Unix
        #[cfg(unix)]
        std::fs::set_permissions(&config_path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| JiraError::Config(format!("Failed to set config permissions: {e}")))?;

        Ok(())
    }
}

pub fn config_file_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("jira")
        .join("config.toml")
}
