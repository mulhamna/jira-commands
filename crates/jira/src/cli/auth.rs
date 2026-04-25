use anyhow::{Context, Result};
use clap::{Subcommand, ValueEnum};
use inquire::{Password, PasswordDisplayMode, Select, Text};
use jira_core::config::{
    config_file_path, default_profile_name, JiraAuthType, JiraConfig, JiraDeployment,
    JiraProfilesFile,
};

#[derive(Debug, Clone, ValueEnum)]
pub enum DeploymentArg {
    Cloud,
    Datacenter,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum AuthTypeArg {
    CloudApiToken,
    DatacenterPat,
    DatacenterBasic,
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// Set up Jira credentials — URL, email/username, and token/password
    ///
    /// Credentials are saved to ~/.config/jira/config.toml (chmod 600 on Unix).
    /// Override the active runtime profile with environment variables:
    ///   JIRA_PROFILE, JIRA_URL, JIRA_EMAIL, JIRA_TOKEN
    Login {
        /// Profile name to save as active (defaults to hostname-derived name)
        #[arg(long, value_name = "PROFILE")]
        profile: Option<String>,
        /// Jira base URL (e.g. https://yourorg.atlassian.net)
        #[arg(long, value_name = "URL")]
        url: Option<String>,
        /// Email address or username depending on auth type
        #[arg(long, value_name = "USER")]
        email: Option<String>,
        /// API token / personal access token / password depending on auth type
        #[arg(long, value_name = "SECRET")]
        token: Option<String>,
        /// Default project key
        #[arg(long, value_name = "PROJECT")]
        project: Option<String>,
        /// Request timeout in seconds
        #[arg(long, value_name = "SECONDS")]
        timeout_secs: Option<u64>,
        /// Deployment target: cloud or datacenter
        #[arg(long, value_enum)]
        deployment: Option<DeploymentArg>,
        /// Authentication mode
        #[arg(long = "auth-type", value_enum)]
        auth_type: Option<AuthTypeArg>,
    },

    /// Remove stored token(s) without deleting profile metadata
    Logout {
        /// Target a specific profile
        #[arg(long, value_name = "PROFILE")]
        profile: Option<String>,
        /// Clear tokens for all saved profiles
        #[arg(long)]
        all: bool,
    },

    /// Show current authentication status and active profile
    Status {
        /// Show a specific profile instead of the active one
        #[arg(long, value_name = "PROFILE")]
        profile: Option<String>,
    },

    /// List saved profiles
    Profiles,

    /// Switch the active profile
    Use {
        /// Profile name to activate
        profile: String,
    },

    /// Update one profile without re-running login
    Update {
        /// Target a specific profile (defaults to active profile)
        #[arg(long, value_name = "PROFILE")]
        profile: Option<String>,
        /// New Jira base URL
        #[arg(long, value_name = "URL")]
        url: Option<String>,
        /// New email or username
        #[arg(long, value_name = "USER")]
        email: Option<String>,
        /// New token / PAT / password
        #[arg(long, value_name = "SECRET")]
        token: Option<String>,
        /// New default project (use empty string to clear)
        #[arg(long, value_name = "PROJECT")]
        project: Option<String>,
        /// New timeout in seconds
        #[arg(long, value_name = "SECONDS")]
        timeout_secs: Option<u64>,
        /// New deployment target
        #[arg(long, value_enum)]
        deployment: Option<DeploymentArg>,
        /// New auth mode
        #[arg(long = "auth-type", value_enum)]
        auth_type: Option<AuthTypeArg>,
    },
}

pub async fn handle(cmd: AuthCommand) -> Result<()> {
    match cmd {
        AuthCommand::Login {
            profile,
            url,
            email,
            token,
            project,
            timeout_secs,
            deployment,
            auth_type,
        } => {
            login(LoginArgs {
                profile,
                url,
                email,
                token,
                project,
                timeout_secs,
                deployment,
                auth_type,
            })
            .await
        }
        AuthCommand::Logout { profile, all } => logout(profile, all).await,
        AuthCommand::Status { profile } => status(profile).await,
        AuthCommand::Profiles => profiles().await,
        AuthCommand::Use { profile } => use_profile(profile).await,
        AuthCommand::Update {
            profile,
            url,
            email,
            token,
            project,
            timeout_secs,
            deployment,
            auth_type,
        } => {
            update(UpdateArgs {
                profile,
                url,
                email,
                token,
                project,
                timeout_secs,
                deployment,
                auth_type,
            })
            .await
        }
    }
}

struct LoginArgs {
    profile: Option<String>,
    url: Option<String>,
    email: Option<String>,
    token: Option<String>,
    project: Option<String>,
    timeout_secs: Option<u64>,
    deployment: Option<DeploymentArg>,
    auth_type: Option<AuthTypeArg>,
}

struct UpdateArgs {
    profile: Option<String>,
    url: Option<String>,
    email: Option<String>,
    token: Option<String>,
    project: Option<String>,
    timeout_secs: Option<u64>,
    deployment: Option<DeploymentArg>,
    auth_type: Option<AuthTypeArg>,
}

async fn login(args: LoginArgs) -> Result<()> {
    println!("Jira Authentication Setup");
    println!("─────────────────────────");

    let base_url = match args.url {
        Some(url) => url.trim().to_string(),
        None => Text::new("Jira base URL (e.g. https://yourorg.atlassian.net):")
            .prompt()
            .context("Failed to read URL")?
            .trim()
            .to_string(),
    };

    let deployment = args
        .deployment
        .clone()
        .map(Into::into)
        .unwrap_or_else(|| infer_deployment(&base_url));
    let deployment = if args.deployment.is_some() {
        deployment
    } else {
        prompt_deployment(deployment.clone())?
    };

    let auth_type = args
        .auth_type
        .clone()
        .map(Into::into)
        .unwrap_or_else(|| default_auth_type(&deployment));
    let auth_type = if args.auth_type.is_some() {
        auth_type
    } else {
        prompt_auth_type(&deployment, auth_type.clone())?
    };

    let mut config = JiraConfig {
        profile_name: None,
        base_url,
        email: String::new(),
        token: None,
        project: args.project.and_then(normalize_optional),
        timeout_secs: args.timeout_secs.unwrap_or(30),
        deployment,
        auth_type,
        api_version: 0,
    };

    if config.requires_user_identity() {
        config.email = match args.email {
            Some(email) => email.trim().to_string(),
            None => Text::new(config.user_label())
                .prompt()
                .context("Failed to read user identity")?
                .trim()
                .to_string(),
        };
    }

    let secret_prompt = format!("{}:", config.credential_label());
    let secret = match args.token {
        Some(token) => token,
        None => Password::new(&secret_prompt)
            .with_display_mode(PasswordDisplayMode::Masked)
            .prompt()
            .context("Failed to read secret")?,
    };
    config.token = Some(secret);

    let profile_name = args
        .profile
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| derive_profile_name(&config));
    config.profile_name = Some(profile_name.clone());
    config.save().context("Failed to save config")?;

    println!("\n✓ Profile '{profile_name}' saved to {}", config_file_path().display());
    println!("  Deployment: {}", deployment_label(&config.deployment));
    println!("  Auth:       {}", auth_type_label(&config.auth_type));

    Ok(())
}

async fn logout(profile: Option<String>, all: bool) -> Result<()> {
    let mut store = JiraProfilesFile::load().unwrap_or_default();

    if store.profiles.is_empty() {
        println!("No credentials found.");
        return Ok(());
    }

    if all {
        for config in store.profiles.values_mut() {
            config.token = None;
        }
        store.save().context("Failed to update config")?;
        println!("✓ Logged out from all profiles.");
        return Ok(());
    }

    let target = profile
        .or_else(|| store.current_profile_name())
        .unwrap_or_else(default_profile_name);
    let config = store
        .profiles
        .get_mut(&target)
        .ok_or_else(|| anyhow::anyhow!("Profile not found: {target}"))?;
    config.token = None;
    store.current_profile = Some(target.clone());
    store.save().context("Failed to update config")?;

    println!("✓ Logged out from profile '{target}'.");
    Ok(())
}

async fn update(args: UpdateArgs) -> Result<()> {
    if args.url.is_none()
        && args.email.is_none()
        && args.token.is_none()
        && args.project.is_none()
        && args.timeout_secs.is_none()
        && args.deployment.is_none()
        && args.auth_type.is_none()
    {
        anyhow::bail!(
            "Nothing to update. Use --url, --email, --token, --project, --timeout-secs, --deployment, or --auth-type."
        );
    }

    let store = JiraProfilesFile::load().unwrap_or_default();
    let target = args
        .profile
        .or_else(|| store.current_profile_name())
        .unwrap_or_else(default_profile_name);

    let mut config: JiraConfig = store
        .profiles
        .get(&target)
        .cloned()
        .map(Into::into)
        .unwrap_or_else(JiraConfig::default);
    config.profile_name = Some(target.clone());

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
        config.project = normalize_optional(project);
    }
    if let Some(timeout_secs) = args.timeout_secs {
        config.timeout_secs = timeout_secs;
    }
    if let Some(deployment) = args.deployment {
        config.deployment = deployment.into();
        if matches!(config.auth_type, JiraAuthType::CloudApiToken)
            && matches!(config.deployment, JiraDeployment::DataCenter)
        {
            config.auth_type = JiraAuthType::DataCenterPat;
        }
        if matches!(config.auth_type, JiraAuthType::DataCenterPat | JiraAuthType::DataCenterBasic)
            && matches!(config.deployment, JiraDeployment::Cloud)
        {
            config.auth_type = JiraAuthType::CloudApiToken;
        }
        config.api_version = 0;
    }
    if let Some(auth_type) = args.auth_type {
        config.auth_type = auth_type.into();
    }
    if !config.requires_user_identity() {
        config.email = String::new();
    }

    config.save().context("Failed to save config")?;
    println!("✓ Profile '{target}' updated.");
    Ok(())
}

async fn status(profile: Option<String>) -> Result<()> {
    let store = JiraProfilesFile::load().unwrap_or_default();
    let config = if let Some(profile_name) = profile.as_ref() {
        let mut config: JiraConfig = store
            .profiles
            .get(profile_name)
            .cloned()
            .map(Into::into)
            .ok_or_else(|| anyhow::anyhow!("Profile not found: {profile_name}"))?;
        config.profile_name = Some(profile_name.clone());
        config
    } else {
        JiraConfig::load().unwrap_or_default()
    };

    if config.base_url.is_empty() && !config.token_present() {
        println!("Not configured. Run `jirac auth login` to set up.");
        return Ok(());
    }

    println!("Authentication Status");
    println!("─────────────────────");
    println!(
        "  Active profile: {}",
        config.profile_name.as_deref().unwrap_or("default")
    );
    println!("  URL:            {}", config.base_url);
    println!("  Deployment:     {}", deployment_label(&config.deployment));
    println!("  Auth:           {}", auth_type_label(&config.auth_type));
    if config.requires_user_identity() {
        println!("  User:           {}", config.email);
    }
    if config.token_present() {
        println!("  Secret:         ✓ stored in {}", config_file_path().display());
    } else {
        println!("  Secret:         ✗ not found — run `jirac auth login`");
    }
    println!("  API version:    {}", config.api_version);
    println!("  Profiles saved: {}", store.profiles.len().max(1));

    if let Some(project) = &config.project {
        println!("  Default project: {project}");
    }

    Ok(())
}

async fn profiles() -> Result<()> {
    let store = JiraProfilesFile::load().unwrap_or_default();
    if store.profiles.is_empty() {
        println!("No saved profiles. Run `jirac auth login` to create one.");
        return Ok(());
    }

    println!("Saved profiles");
    println!("──────────────");
    let current = store.current_profile_name();
    for (name, profile) in store.profiles {
        let marker = if current.as_deref() == Some(name.as_str()) {
            "*"
        } else {
            " "
        };
        let user_suffix = if profile.email.trim().is_empty() {
            String::new()
        } else {
            format!(" [{}]", profile.email)
        };
        println!(
            "{marker} {name} — {} · {} · api v{}{}",
            deployment_label(&profile.deployment),
            auth_type_label(&profile.auth_type),
            profile.api_version,
            user_suffix,
        );
    }

    Ok(())
}

async fn use_profile(profile: String) -> Result<()> {
    let mut store = JiraProfilesFile::load().unwrap_or_default();
    store
        .set_current_profile(&profile)
        .context("Failed to switch profile")?;
    store.save().context("Failed to save config")?;

    println!("✓ Active profile set to '{profile}'.");
    Ok(())
}

fn prompt_deployment(default: JiraDeployment) -> Result<JiraDeployment> {
    let options = vec!["Cloud", "Data Center"];
    let selection = Select::new("Deployment type:", options)
        .with_starting_cursor(match default {
            JiraDeployment::Cloud => 0,
            JiraDeployment::DataCenter => 1,
        })
        .prompt()?;

    Ok(match selection {
        "Cloud" => JiraDeployment::Cloud,
        _ => JiraDeployment::DataCenter,
    })
}

fn prompt_auth_type(deployment: &JiraDeployment, default: JiraAuthType) -> Result<JiraAuthType> {
    let options = match deployment {
        JiraDeployment::Cloud => vec!["Cloud API token"],
        JiraDeployment::DataCenter => vec!["Data Center PAT", "Data Center basic"],
    };

    let selection = Select::new("Authentication mode:", options)
        .with_starting_cursor(match default {
            JiraAuthType::CloudApiToken | JiraAuthType::DataCenterPat => 0,
            JiraAuthType::DataCenterBasic => 1,
        })
        .prompt()?;

    Ok(match selection {
        "Cloud API token" => JiraAuthType::CloudApiToken,
        "Data Center basic" => JiraAuthType::DataCenterBasic,
        _ => JiraAuthType::DataCenterPat,
    })
}

fn infer_deployment(base_url: &str) -> JiraDeployment {
    if base_url.contains("atlassian.net") {
        JiraDeployment::Cloud
    } else {
        JiraDeployment::DataCenter
    }
}

fn default_auth_type(deployment: &JiraDeployment) -> JiraAuthType {
    match deployment {
        JiraDeployment::Cloud => JiraAuthType::CloudApiToken,
        JiraDeployment::DataCenter => JiraAuthType::DataCenterPat,
    }
}

fn derive_profile_name(config: &JiraConfig) -> String {
    let host = config
        .base_url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("jira")
        .replace(':', "-");

    if config.email.trim().is_empty() {
        host
    } else {
        let user = config
            .email
            .split('@')
            .next()
            .unwrap_or("user")
            .replace(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_', "-");
        format!("{host}-{user}")
    }
}

fn normalize_optional(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn deployment_label(value: &JiraDeployment) -> &'static str {
    match value {
        JiraDeployment::Cloud => "Cloud",
        JiraDeployment::DataCenter => "Data Center",
    }
}

fn auth_type_label(value: &JiraAuthType) -> &'static str {
    match value {
        JiraAuthType::CloudApiToken => "Cloud API token",
        JiraAuthType::DataCenterPat => "Data Center PAT",
        JiraAuthType::DataCenterBasic => "Data Center basic",
    }
}

impl From<DeploymentArg> for JiraDeployment {
    fn from(value: DeploymentArg) -> Self {
        match value {
            DeploymentArg::Cloud => JiraDeployment::Cloud,
            DeploymentArg::Datacenter => JiraDeployment::DataCenter,
        }
    }
}

impl From<AuthTypeArg> for JiraAuthType {
    fn from(value: AuthTypeArg) -> Self {
        match value {
            AuthTypeArg::CloudApiToken => JiraAuthType::CloudApiToken,
            AuthTypeArg::DatacenterPat => JiraAuthType::DataCenterPat,
            AuthTypeArg::DatacenterBasic => JiraAuthType::DataCenterBasic,
        }
    }
}

