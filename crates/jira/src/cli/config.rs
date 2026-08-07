use anyhow::{Context, Result};
use clap::Subcommand;
use jira_core::config::{config_file_path, JiraConfig, JiraProfilesFile};

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Set a configuration value for the active profile
    ///
    /// Currently supports `default_issue_limit`, the default cap (in number of
    /// issues) applied by list-like commands when no explicit `--limit` is
    /// given. Set to `0` to clear (fetch everything).
    Set {
        /// Configuration key (e.g. default_issue_limit)
        key: String,
        /// Configuration value (e.g. 100)
        value: String,
    },
    /// Remove a configuration value for the active profile,
    /// restoring the built-in default (fetch everything)
    Unset {
        /// Configuration key (e.g. default_issue_limit)
        key: String,
    },
    /// Show the active profile's effective configuration
    Show,
}

pub async fn handle(cmd: ConfigCommand) -> Result<()> {
    match cmd {
        ConfigCommand::Set { key, value } => set_value(&key, &value),
        ConfigCommand::Unset { key } => unset_value(&key),
        ConfigCommand::Show => show(),
    }
}

fn set_value(key: &str, value: &str) -> Result<()> {
    match key {
        "default_issue_limit" => {
            let parsed = value.trim().parse::<u32>().map_err(|_| {
                anyhow::anyhow!("`default_issue_limit` must be a non-negative integer")
            })?;
            let mut config = JiraConfig::load().unwrap_or_default();
            config.default_issue_limit = if parsed == 0 { None } else { Some(parsed) };
            config.save().context("Failed to save config")?;
            match config.default_issue_limit {
                Some(n) => println!("✓ default_issue_limit = {n}"),
                None => println!("✓ default_issue_limit cleared (will fetch all issues)"),
            }
            println!("  Saved to {}", config_file_path().display());
        }
        other => {
            anyhow::bail!("unsupported config key `{other}`. Supported: default_issue_limit")
        }
    }
    Ok(())
}

fn unset_value(key: &str) -> Result<()> {
    match key {
        "default_issue_limit" => {
            let mut config = JiraConfig::load().unwrap_or_default();
            config.default_issue_limit = None;
            config.save().context("Failed to save config")?;
            println!("✓ default_issue_limit removed (will fetch all issues)");
        }
        other => anyhow::bail!("unsupported config key `{other}`. Supported: default_issue_limit"),
    }
    Ok(())
}

fn show() -> Result<()> {
    let config = JiraConfig::load().unwrap_or_default();
    let profile = config
        .profile_name
        .clone()
        .unwrap_or_else(|| "default".to_string());
    println!("Profile:        {profile}");
    println!(
        "Base URL:       {}",
        if config.base_url.is_empty() {
            "(not set)"
        } else {
            &config.base_url
        }
    );
    println!(
        "Project:        {}",
        config.project.clone().unwrap_or_else(|| "(not set)".into())
    );
    println!(
        "default_issue_limit: {}",
        config
            .default_issue_limit
            .map(|n| n.to_string())
            .unwrap_or_else(|| "unset (fetch all)".into())
    );

    let _ = JiraProfilesFile::load(); // validate load path
    Ok(())
}
