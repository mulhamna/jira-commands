use anyhow::{Context, Result};
use clap::Subcommand;
use inquire::{Password, PasswordDisplayMode, Text};
use jira_core::config::{config_file_path, JiraConfig};

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// Set up Jira credentials — URL, email, and API token
    ///
    /// Credentials are saved to ~/.config/jira/config.toml (chmod 600 on Unix).
    /// Override any field at runtime with environment variables:
    ///   JIRA_URL, JIRA_EMAIL, JIRA_TOKEN
    ///
    /// API tokens are different from your Atlassian password.
    /// Generate one at: https://id.atlassian.com/manage-profile/security/api-tokens
    Login,

    /// Remove the stored API token
    ///
    /// Clears only the token — URL and email are preserved.
    /// Run `jirac auth login` again to set a new token.
    Logout,

    /// Show current authentication status and configuration
    ///
    /// Displays: Jira URL, email, token presence, config file path,
    /// and default project key (if configured).
    Status,

    /// Update individual credential fields without re-running login
    ///
    /// Useful for rotating tokens or switching users without entering
    /// all credentials again. Only provided flags are changed.
    ///
    /// Examples:
    ///   jira auth update --token <new-token>
    ///   jira auth update --email new@example.com
    ///   jira auth update --url https://neworg.atlassian.net
    ///   jira auth update --email new@example.com --token <token>
    Update {
        /// New Jira base URL (e.g. https://yourorg.atlassian.net)
        #[arg(long, value_name = "URL")]
        url: Option<String>,
        /// New email address
        #[arg(long, value_name = "EMAIL")]
        email: Option<String>,
        /// New API token
        #[arg(long, value_name = "TOKEN")]
        token: Option<String>,
    },
}

pub async fn handle(cmd: AuthCommand) -> Result<()> {
    match cmd {
        AuthCommand::Login => login().await,
        AuthCommand::Logout => logout().await,
        AuthCommand::Status => status().await,
        AuthCommand::Update { url, email, token } => update(url, email, token).await,
    }
}

async fn login() -> Result<()> {
    println!("Jira Authentication Setup");
    println!("─────────────────────────");

    let base_url = Text::new("Jira base URL (e.g. https://yourorg.atlassian.net):")
        .prompt()
        .context("Failed to read URL")?;

    let email = Text::new("Email address:")
        .prompt()
        .context("Failed to read email")?;

    let token = Password::new(
        "API token (from https://id.atlassian.com/manage-profile/security/api-tokens):",
    )
    .with_display_mode(PasswordDisplayMode::Masked)
    .prompt()
    .context("Failed to read token")?;

    let mut config = JiraConfig::load().unwrap_or_default();
    config.base_url = base_url.trim().to_string();
    config.email = email.trim().to_string();
    config.token = Some(token);

    config.save().context("Failed to save config")?;

    println!("\n✓ Credentials saved to {}", config_file_path().display());

    Ok(())
}

async fn logout() -> Result<()> {
    let mut config = JiraConfig::load().unwrap_or_default();

    if config.email.is_empty() {
        println!("No credentials found.");
        return Ok(());
    }

    config.token = None;
    config.save().context("Failed to update config")?;

    println!("✓ Logged out ({}).", config.email);
    Ok(())
}

async fn update(url: Option<String>, email: Option<String>, token: Option<String>) -> Result<()> {
    if url.is_none() && email.is_none() && token.is_none() {
        anyhow::bail!("Nothing to update. Use --url, --email, or --token.");
    }

    let mut config = JiraConfig::load().unwrap_or_default();

    if let Some(u) = url {
        config.base_url = u.trim().to_string();
        println!("✓ URL updated.");
    }
    if let Some(e) = email {
        config.email = e.trim().to_string();
        println!("✓ Email updated.");
    }
    if let Some(t) = token {
        config.token = Some(t);
        println!("✓ Token updated.");
    }

    config.save().context("Failed to save config")?;
    Ok(())
}

async fn status() -> Result<()> {
    let config = JiraConfig::load().unwrap_or_default();

    if config.base_url.is_empty() || config.email.is_empty() {
        println!("Not configured. Run `jirac auth login` to set up.");
        return Ok(());
    }

    println!("Authentication Status");
    println!("─────────────────────");
    println!("  URL:   {}", config.base_url);
    println!("  Email: {}", config.email);

    if config.token.is_some() {
        println!("  Token: ✓ stored in {}", config_file_path().display());
    } else {
        println!("  Token: ✗ not found — run `jirac auth login`");
    }

    if let Some(project) = &config.project {
        println!("  Default project: {project}");
    }

    Ok(())
}
