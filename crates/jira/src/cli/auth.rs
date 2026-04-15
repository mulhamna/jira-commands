use anyhow::{Context, Result};
use clap::Subcommand;
use inquire::{Password, PasswordDisplayMode, Text};
use jira_core::{
    auth::Auth,
    config::{config_file_path, JiraConfig},
};

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// Log in — save URL, email, and API token
    Login,
    /// Log out — remove saved credentials
    Logout,
    /// Show current authentication status
    Status,
}

pub async fn handle(cmd: AuthCommand) -> Result<()> {
    match cmd {
        AuthCommand::Login => login().await,
        AuthCommand::Logout => logout().await,
        AuthCommand::Status => status().await,
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

    // Save to keyring
    Auth::save_token(&email, &token).context("Failed to save token to keyring")?;

    // Save config to file
    let mut config = JiraConfig::load().unwrap_or_default();
    config.base_url = base_url.trim().to_string();
    config.email = email.trim().to_string();
    config.token = None; // Token is in keyring, not config file

    config.save().context("Failed to save config")?;

    println!("\n✓ Credentials saved.");
    println!("  Config: {}", config_file_path().display());
    println!("  Token stored in OS keyring.");

    Ok(())
}

async fn logout() -> Result<()> {
    let config = JiraConfig::load().unwrap_or_default();

    if config.email.is_empty() {
        println!("No credentials found.");
        return Ok(());
    }

    match Auth::delete_token(&config.email) {
        Ok(()) => println!("✓ Token removed from keyring for {}", config.email),
        Err(e) => println!("Warning: could not remove token from keyring: {e}"),
    }

    println!("Logged out.");
    Ok(())
}

async fn status() -> Result<()> {
    let config = JiraConfig::load().unwrap_or_default();

    if config.base_url.is_empty() || config.email.is_empty() {
        println!("Not configured. Run `jira auth login` to set up.");
        return Ok(());
    }

    println!("Authentication Status");
    println!("─────────────────────");
    println!("  URL:   {}", config.base_url);
    println!("  Email: {}", config.email);

    match Auth::get_token(&config.email) {
        Ok(_) => println!("  Token: ✓ stored in keyring"),
        Err(_) => println!("  Token: ✗ not found in keyring"),
    }

    if let Some(project) = &config.project {
        println!("  Default project: {project}");
    }

    Ok(())
}
