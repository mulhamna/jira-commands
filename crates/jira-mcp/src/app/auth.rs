use serde_json::{json, Value};

use jira_core::config::{
    config_file_path, default_profile_name, parse_auth_type, parse_deployment, JiraConfig,
    JiraProfilesFile,
};

use crate::{
    error::{AppError, AppResult},
    models::AuthSetCredentialsArgs,
};

use super::{shared::value_or_null, JiraApp};

impl JiraApp {
    pub fn auth_status(&self) -> AppResult<Value> {
        let config = self.load_config()?;
        let store = JiraProfilesFile::load()?;
        Ok(json!({
            "configured": !config.base_url.is_empty() && (!config.requires_user_identity() || !config.email.is_empty()),
            "profile": config.profile_name.clone(),
            "url": value_or_null(config.base_url.clone()),
            "email": value_or_null(config.email.clone()),
            "token_present": config.token_present(),
            "project": config.project,
            "timeout_secs": config.timeout_secs,
            "deployment": format!("{:?}", config.deployment).to_lowercase(),
            "auth_type": format!("{:?}", config.auth_type).to_lowercase(),
            "api_version": config.api_version,
            "profiles": store.profiles.keys().cloned().collect::<Vec<_>>(),
            "config_path": config_file_path().display().to_string()
        }))
    }

    pub fn auth_set_credentials(&self, args: AuthSetCredentialsArgs) -> AppResult<Value> {
        if args.url.is_none()
            && args.email.is_none()
            && args.token.is_none()
            && args.project.is_none()
            && args.timeout_secs.is_none()
            && args.deployment.is_none()
            && args.auth_type.is_none()
        {
            return Err(AppError::validation(
                "Provide at least one of url, email, token, project, timeout_secs, deployment, or auth_type",
            ));
        }

        let store = JiraProfilesFile::load().unwrap_or_default();
        let profile_name = args
            .profile
            .clone()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| store.current_profile_name())
            .unwrap_or_else(default_profile_name);

        let mut config: JiraConfig = store
            .profiles
            .get(&profile_name)
            .cloned()
            .map(Into::into)
            .unwrap_or_else(JiraConfig::default);
        config.profile_name = Some(profile_name.clone());

        if let Some(url) = args.url {
            config.base_url = url.trim().to_string();
        }
        if let Some(email) = args.email {
            config.email = email.trim().to_string();
        }
        if let Some(token) = args.token {
            config.token = Some(token);
        }
        if let Some(project) = args.project {
            config.project = if project.trim().is_empty() {
                None
            } else {
                Some(project.trim().to_string())
            };
        }
        if let Some(timeout_secs) = args.timeout_secs {
            config.timeout_secs = timeout_secs;
        }
        if let Some(deployment) = args.deployment {
            config.deployment = parse_deployment(&deployment)
                .ok_or_else(|| AppError::validation("deployment must be cloud or datacenter"))?;
            config.api_version = 0;
        }
        if let Some(auth_type) = args.auth_type {
            config.auth_type = parse_auth_type(&auth_type).ok_or_else(|| {
                AppError::validation(
                    "auth_type must be cloud_api_token, datacenter_pat, or datacenter_basic",
                )
            })?;
        }
        if !config.requires_user_identity() {
            config.email.clear();
        }

        config.save()?;
        self.auth_status()
    }

    pub fn auth_logout(&self) -> AppResult<Value> {
        let mut config = self.load_config().unwrap_or_default();
        config.token = None;
        config.save()?;
        self.auth_status()
    }
}
