use anyhow::{Context, Result};
use clap::Subcommand;
use jira_core::config::{
    config_file_path, parse_input_format, InputFormat, JiraConfig, JiraProfilesFile,
};

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Set a configuration value for the active profile
    ///
    /// `default_issue_limit`: default cap (in number of issues) applied by list-like
    /// commands when no explicit `--limit` is given. Set to `0` to clear (fetch everything).
    ///
    /// `input_format`: how descriptions, comments and worklog comments are interpreted.
    /// `markdown` (default) is converted for the API version in use; `wiki` is native
    /// Jira wiki markup sent unchanged (Jira API v2 only); `adf` is a JSON document.
    /// JIRA_INPUT_FORMAT overrides it.
    ///
    /// Examples: `jirac config set default_issue_limit 100`, `jirac config set input_format wiki`
    Set {
        /// Configuration key (default_issue_limit or input_format)
        key: String,
        /// Configuration value (e.g. 100, or markdown / wiki / adf)
        value: String,
    },
    /// Remove a configuration value for the active profile,
    /// restoring the built-in default (fetch everything; Markdown text)
    Unset {
        /// Configuration key (default_issue_limit or input_format)
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
            let mut config = JiraConfig::load_stored().context("Failed to load config")?;
            config.default_issue_limit = if parsed == 0 { None } else { Some(parsed) };
            config.save().context("Failed to save config")?;
            match config.default_issue_limit {
                Some(n) => println!("✓ default_issue_limit = {n}"),
                None => println!("✓ default_issue_limit cleared (will fetch all issues)"),
            }
            println!("  Saved to {}", config_file_path().display());
        }
        "input_format" => {
            let parsed = parse_input_format(value).ok_or_else(|| {
                anyhow::anyhow!("`input_format` must be `markdown`, `wiki` or `adf`")
            })?;
            let mut config = JiraConfig::load_stored().context("Failed to load config")?;
            if parsed == InputFormat::Wiki && config.api_version >= 3 {
                anyhow::bail!(
                    "`input_format = wiki` requires Jira API v2, but this profile uses API v{}",
                    config.api_version
                );
            }
            config.input_format = parsed;
            config.save().context("Failed to save config")?;
            println!("✓ input_format = {}", parsed.as_str());
            println!("  Saved to {}", config_file_path().display());
        }
        other => {
            anyhow::bail!(
                "unsupported config key `{other}`. Supported: default_issue_limit, input_format"
            )
        }
    }
    Ok(())
}

fn unset_value(key: &str) -> Result<()> {
    match key {
        "default_issue_limit" => {
            let mut config = JiraConfig::load_stored().context("Failed to load config")?;
            config.default_issue_limit = None;
            config.save().context("Failed to save config")?;
            println!("✓ default_issue_limit removed (will fetch all issues)");
        }
        "input_format" => {
            let mut config = JiraConfig::load_stored().context("Failed to load config")?;
            config.input_format = InputFormat::Markdown;
            config.save().context("Failed to save config")?;
            println!("✓ input_format removed (markdown)");
        }
        other => anyhow::bail!(
            "unsupported config key `{other}`. Supported: default_issue_limit, input_format"
        ),
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
    println!("input_format: {}", config.input_format.as_str());

    let _ = JiraProfilesFile::load(); // validate load path
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempHome;

    #[test]
    fn input_format_set_and_unset_keep_env_only_values_out_of_the_profile() {
        let home = TempHome::new("config-text-format");
        home.write_config(
            r#"current_profile = "dc"

[profiles.dc]
base_url = "https://jira.internal"
token = "pat"
project = "PROJ"
timeout_secs = 30
deployment = "data_center"
auth_type = "data_center_pat"
api_version = 2
"#,
        );
        std::env::set_var("JIRA_PROJECT", "ENV");
        std::env::set_var("JIRA_TOKEN", "env-token");

        set_value("input_format", "Wiki").expect("set wiki");
        let saved = home.config_text();
        assert!(saved.contains("input_format = \"wiki\""), "{saved}");
        assert!(saved.contains("project = \"PROJ\""), "{saved}");
        assert!(saved.contains("token = \"pat\""), "{saved}");

        std::env::set_var("JIRA_INPUT_FORMAT", "markdown");
        set_value("default_issue_limit", "7").expect("set limit");
        let saved = home.config_text();
        assert!(saved.contains("input_format = \"wiki\""), "{saved}");
        assert!(saved.contains("default_issue_limit = 7"), "{saved}");

        unset_value("input_format").expect("unset");
        assert!(!home.config_text().contains("input_format"));
    }
}
