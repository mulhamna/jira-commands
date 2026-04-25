use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{collections::BTreeMap, env, path::PathBuf};

use crate::error::{JiraError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum JiraDeployment {
    #[default]
    Cloud,
    DataCenter,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum JiraAuthType {
    #[default]
    CloudApiToken,
    DataCenterPat,
    DataCenterBasic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraConfig {
    #[serde(default)]
    pub profile_name: Option<String>,
    pub base_url: String,
    pub email: String,
    pub token: Option<String>,
    pub project: Option<String>,
    pub timeout_secs: u64,
    #[serde(default)]
    pub deployment: JiraDeployment,
    #[serde(default)]
    pub auth_type: JiraAuthType,
    #[serde(default = "default_api_version")]
    pub api_version: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraProfileConfig {
    pub base_url: String,
    #[serde(default)]
    pub email: String,
    pub token: Option<String>,
    pub project: Option<String>,
    pub timeout_secs: u64,
    #[serde(default)]
    pub deployment: JiraDeployment,
    #[serde(default)]
    pub auth_type: JiraAuthType,
    #[serde(default = "default_api_version")]
    pub api_version: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JiraProfilesFile {
    pub current_profile: Option<String>,
    #[serde(default)]
    pub profiles: BTreeMap<String, JiraProfileConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyJiraConfig {
    pub base_url: String,
    pub email: String,
    pub token: Option<String>,
    pub project: Option<String>,
    pub timeout_secs: u64,
}

fn default_api_version() -> u8 {
    3
}

impl Default for JiraConfig {
    fn default() -> Self {
        Self {
            profile_name: Some(default_profile_name()),
            base_url: String::new(),
            email: String::new(),
            token: None,
            project: None,
            timeout_secs: 30,
            deployment: JiraDeployment::Cloud,
            auth_type: JiraAuthType::CloudApiToken,
            api_version: default_api_version(),
        }
    }
}

impl Default for JiraProfileConfig {
    fn default() -> Self {
        JiraConfig::default().into_profile()
    }
}

impl From<JiraProfileConfig> for JiraConfig {
    fn from(value: JiraProfileConfig) -> Self {
        let api_version = normalize_api_version(value.api_version, &value.deployment);
        Self {
            profile_name: None,
            base_url: value.base_url,
            email: value.email,
            token: value.token,
            project: value.project,
            timeout_secs: value.timeout_secs,
            deployment: value.deployment,
            auth_type: value.auth_type,
            api_version,
        }
    }
}

impl JiraConfig {
    /// Load the active profile from ~/.config/jira/config.toml and env vars.
    pub fn load() -> Result<Self> {
        let profile_override = env::var("JIRA_PROFILE").ok().filter(|s| !s.trim().is_empty());
        let store = JiraProfilesFile::load()?;

        let mut config = if let Some(profile_name) = profile_override.clone() {
            store
                .profiles
                .get(&profile_name)
                .cloned()
                .map(Into::into)
                .unwrap_or_else(JiraConfig::default)
        } else {
            store.active_profile().map(Into::into).unwrap_or_default()
        };

        config.profile_name = Some(
            profile_override
                .or_else(|| store.current_profile.clone())
                .unwrap_or_else(default_profile_name),
        );
        config.apply_env_overrides();
        config.api_version = normalize_api_version(config.api_version, &config.deployment);

        Ok(config)
    }

    pub fn into_profile(self) -> JiraProfileConfig {
        let api_version = normalize_api_version(self.api_version, &self.deployment);
        JiraProfileConfig {
            base_url: self.base_url,
            email: self.email,
            token: self.token,
            project: self.project,
            timeout_secs: self.timeout_secs,
            deployment: self.deployment,
            auth_type: self.auth_type,
            api_version,
        }
    }

    pub fn save(&self) -> Result<()> {
        let profile_name = self
            .profile_name
            .clone()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(default_profile_name);

        let mut store = JiraProfilesFile::load()?;
        store.current_profile = Some(profile_name.clone());
        store
            .profiles
            .insert(profile_name, self.clone().into_profile().normalized());
        store.save()
    }

    pub fn token_present(&self) -> bool {
        self.token
            .as_deref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
    }

    pub fn requires_user_identity(&self) -> bool {
        matches!(self.auth_type, JiraAuthType::CloudApiToken | JiraAuthType::DataCenterBasic)
    }

    pub fn credential_label(&self) -> &'static str {
        match self.auth_type {
            JiraAuthType::CloudApiToken => "API token",
            JiraAuthType::DataCenterPat => "Personal access token",
            JiraAuthType::DataCenterBasic => "Password or personal access token",
        }
    }

    pub fn user_label(&self) -> &'static str {
        match self.auth_type {
            JiraAuthType::DataCenterBasic => "Username",
            _ => "Email address",
        }
    }

    pub fn auth_header_kind(&self) -> &'static str {
        match self.auth_type {
            JiraAuthType::DataCenterPat => "Bearer",
            JiraAuthType::CloudApiToken | JiraAuthType::DataCenterBasic => "Basic",
        }
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(url) = env::var("JIRA_URL") {
            self.base_url = url;
        }
        if let Ok(email) = env::var("JIRA_EMAIL") {
            self.email = email;
        }
        if let Ok(token) = env::var("JIRA_TOKEN") {
            self.token = Some(token);
        }
        if let Ok(project) = env::var("JIRA_PROJECT") {
            self.project = if project.trim().is_empty() {
                None
            } else {
                Some(project)
            };
        }
        if let Ok(timeout_secs) = env::var("JIRA_TIMEOUT_SECS") {
            if let Ok(value) = timeout_secs.parse::<u64>() {
                self.timeout_secs = value;
            }
        }
        if let Ok(deployment) = env::var("JIRA_DEPLOYMENT") {
            if let Some(value) = parse_deployment(&deployment) {
                self.deployment = value;
            }
        }
        if let Ok(auth_type) = env::var("JIRA_AUTH_TYPE") {
            if let Some(value) = parse_auth_type(&auth_type) {
                self.auth_type = value;
            }
        }
        if let Ok(api_version) = env::var("JIRA_API_VERSION") {
            if let Ok(value) = api_version.parse::<u8>() {
                self.api_version = value;
            }
        }
    }
}

impl JiraProfileConfig {
    fn normalized(mut self) -> Self {
        self.api_version = normalize_api_version(self.api_version, &self.deployment);
        self
    }
}

impl JiraProfilesFile {
    pub fn load() -> Result<Self> {
        let config_path = config_file_path();
        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| JiraError::Config(format!("Failed to read config: {e}")))?;

        let parsed: toml::Value = toml::from_str(&content)
            .map_err(|e| JiraError::Config(format!("Failed to parse config: {e}")))?;

        if parsed.get("profiles").is_some() || parsed.get("current_profile").is_some() {
            let mut store: JiraProfilesFile = toml::from_str(&content)
                .map_err(|e| JiraError::Config(format!("Failed to parse config: {e}")))?;
            for profile in store.profiles.values_mut() {
                profile.api_version = normalize_api_version(profile.api_version, &profile.deployment);
            }
            return Ok(store);
        }

        let legacy: LegacyJiraConfig = toml::from_str(&content)
            .map_err(|e| JiraError::Config(format!("Failed to parse legacy config: {e}")))?;

        let mut profiles = BTreeMap::new();
        profiles.insert(
            default_profile_name(),
            JiraProfileConfig {
                base_url: legacy.base_url,
                email: legacy.email,
                token: legacy.token,
                project: legacy.project,
                timeout_secs: legacy.timeout_secs,
                deployment: JiraDeployment::Cloud,
                auth_type: JiraAuthType::CloudApiToken,
                api_version: default_api_version(),
            },
        );

        Ok(Self {
            current_profile: Some(default_profile_name()),
            profiles,
        })
    }

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

        #[cfg(unix)]
        std::fs::set_permissions(&config_path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| JiraError::Config(format!("Failed to set config permissions: {e}")))?;

        Ok(())
    }

    pub fn active_profile(&self) -> Option<JiraProfileConfig> {
        let name = self
            .current_profile
            .clone()
            .or_else(|| self.profiles.keys().next().cloned())?;
        self.profiles.get(&name).cloned()
    }

    pub fn current_profile_name(&self) -> Option<String> {
        self.current_profile
            .clone()
            .or_else(|| self.profiles.keys().next().cloned())
    }

    pub fn set_current_profile(&mut self, profile_name: &str) -> Result<()> {
        if !self.profiles.contains_key(profile_name) {
            return Err(JiraError::Config(format!("Profile not found: {profile_name}")));
        }
        self.current_profile = Some(profile_name.to_string());
        Ok(())
    }

    pub fn remove_profile(&mut self, profile_name: &str) -> Result<()> {
        if self.profiles.remove(profile_name).is_none() {
            return Err(JiraError::Config(format!("Profile not found: {profile_name}")));
        }
        if self.current_profile.as_deref() == Some(profile_name) {
            self.current_profile = self.profiles.keys().next().cloned();
        }
        Ok(())
    }
}

pub fn config_file_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("jira")
        .join("config.toml")
}

pub fn parse_deployment(value: &str) -> Option<JiraDeployment> {
    match value.trim().to_ascii_lowercase().as_str() {
        "cloud" => Some(JiraDeployment::Cloud),
        "datacenter" | "data_center" | "data-center" | "dc" | "self-managed" | "self_managed" => {
            Some(JiraDeployment::DataCenter)
        }
        _ => None,
    }
}

pub fn parse_auth_type(value: &str) -> Option<JiraAuthType> {
    match value.trim().to_ascii_lowercase().as_str() {
        "cloud_api_token" | "cloud-api-token" | "cloud" | "api-token" | "api_token" => {
            Some(JiraAuthType::CloudApiToken)
        }
        "datacenter_pat" | "datacenter-pat" | "data_center_pat" | "dc-pat" | "pat" => {
            Some(JiraAuthType::DataCenterPat)
        }
        "datacenter_basic" | "datacenter-basic" | "data_center_basic" | "dc-basic" | "basic" => {
            Some(JiraAuthType::DataCenterBasic)
        }
        _ => None,
    }
}

pub fn normalize_api_version(api_version: u8, deployment: &JiraDeployment) -> u8 {
    if api_version == 0 {
        match deployment {
            JiraDeployment::Cloud => 3,
            JiraDeployment::DataCenter => 2,
        }
    } else {
        api_version
    }
}

pub fn default_profile_name() -> String {
    "default".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};
    use tempfile::TempDir;

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn set_config_home(temp_dir: &TempDir) {
        std::env::set_var("XDG_CONFIG_HOME", temp_dir.path());
        std::env::set_var("HOME", temp_dir.path());
        std::env::set_var("USERPROFILE", temp_dir.path());
        std::env::set_var("APPDATA", temp_dir.path());
        std::env::set_var("LOCALAPPDATA", temp_dir.path());
    }

    fn clear_config_home() {
        std::env::remove_var("XDG_CONFIG_HOME");
        std::env::remove_var("HOME");
        std::env::remove_var("USERPROFILE");
        std::env::remove_var("APPDATA");
        std::env::remove_var("LOCALAPPDATA");
        std::env::remove_var("JIRA_URL");
        std::env::remove_var("JIRA_EMAIL");
        std::env::remove_var("JIRA_TOKEN");
        std::env::remove_var("JIRA_PROFILE");
        std::env::remove_var("JIRA_DEPLOYMENT");
        std::env::remove_var("JIRA_AUTH_TYPE");
        std::env::remove_var("JIRA_API_VERSION");
    }

    #[test]
    fn migrates_legacy_config_into_default_profile() {
        let _guard = env_lock().lock().expect("env lock");
        let temp_dir = TempDir::new().expect("tempdir");
        clear_config_home();
        set_config_home(&temp_dir);

        std::fs::create_dir_all(config_file_path().parent().expect("parent")).expect("mkdir");
        std::fs::write(
            config_file_path(),
            r#"base_url = "https://example.atlassian.net"
email = "dev@example.com"
token = "secret"
project = "PROJ"
timeout_secs = 55
"#,
        )
        .expect("write");

        let config = JiraConfig::load().expect("load legacy");
        assert_eq!(config.profile_name.as_deref(), Some("default"));
        assert_eq!(config.base_url, "https://example.atlassian.net");
        assert_eq!(config.auth_type, JiraAuthType::CloudApiToken);
        assert_eq!(config.api_version, 3);

        clear_config_home();
    }

    #[test]
    fn loads_named_profile_and_applies_env_overrides() {
        let _guard = env_lock().lock().expect("env lock");
        let temp_dir = TempDir::new().expect("tempdir");
        clear_config_home();
        set_config_home(&temp_dir);

        let store = JiraProfilesFile {
            current_profile: Some("cloud-main".into()),
            profiles: BTreeMap::from([
                (
                    "cloud-main".into(),
                    JiraProfileConfig {
                        base_url: "https://example.atlassian.net".into(),
                        email: "cloud@example.com".into(),
                        token: Some("cloud-token".into()),
                        project: Some("CLOUD".into()),
                        timeout_secs: 30,
                        deployment: JiraDeployment::Cloud,
                        auth_type: JiraAuthType::CloudApiToken,
                        api_version: 3,
                    },
                ),
                (
                    "dc-main".into(),
                    JiraProfileConfig {
                        base_url: "https://jira.internal".into(),
                        email: String::new(),
                        token: Some("dc-token".into()),
                        project: Some("DC".into()),
                        timeout_secs: 40,
                        deployment: JiraDeployment::DataCenter,
                        auth_type: JiraAuthType::DataCenterPat,
                        api_version: 2,
                    },
                ),
            ]),
        };
        store.save().expect("save");

        std::env::set_var("JIRA_PROFILE", "dc-main");
        std::env::set_var("JIRA_PROJECT", "OPS");

        let config = JiraConfig::load().expect("load profile");
        assert_eq!(config.profile_name.as_deref(), Some("dc-main"));
        assert_eq!(config.base_url, "https://jira.internal");
        assert_eq!(config.project.as_deref(), Some("OPS"));
        assert_eq!(config.auth_type, JiraAuthType::DataCenterPat);
        assert_eq!(config.api_version, 2);

        clear_config_home();
    }
}
