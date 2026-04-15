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
    /// Update individual credential fields
    Update {
        /// New Jira base URL
        #[arg(long)]
        url: Option<String>,
        /// New email address
        #[arg(long)]
        email: Option<String>,
        /// New API token
        #[arg(long)]
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

    let email_trimmed = email.trim().to_string();
    let base_url_trimmed = base_url.trim().to_string();

    // Try keyring first, fall back to config file
    let keyring_ok = Auth::save_token(&email_trimmed, &token).is_ok();

    // Save config to file
    let mut config = JiraConfig::load().unwrap_or_default();
    config.base_url = base_url_trimmed;
    config.email = email_trimmed;
    // Store token in config as fallback when keyring unavailable (file will be chmod 600)
    config.token = if keyring_ok { None } else { Some(token) };

    config.save().context("Failed to save config")?;

    println!("\n✓ Credentials saved.");
    println!("  Config: {}", config_file_path().display());
    if keyring_ok {
        println!("  Token stored in OS keyring.");
    } else {
        println!("  Token stored in config file (keyring unavailable).");
    }

    Ok(())
}

async fn logout() -> Result<()> {
    let mut config = JiraConfig::load().unwrap_or_default();

    if config.email.is_empty() {
        println!("No credentials found.");
        return Ok(());
    }

    match Auth::delete_token(&config.email) {
        Ok(()) => println!("✓ Token removed from keyring for {}", config.email),
        Err(e) => println!("Note: keyring: {e}"),
    }

    // Also clear token from config file
    config.token = None;
    config.save().context("Failed to update config")?;

    println!("Logged out.");
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

    if let Some(new_email) = email {
        // Migrate token in keyring to new email key
        if let Ok(existing_token) = Auth::get_token(&config.email) {
            let _ = Auth::delete_token(&config.email);
            let _ = Auth::save_token(new_email.trim(), &existing_token);
        }
        config.email = new_email.trim().to_string();
        println!("✓ Email updated.");
    }

    if let Some(t) = token {
        let keyring_ok = Auth::save_token(&config.email, &t).is_ok();
        config.token = if keyring_ok { None } else { Some(t) };
        println!("✓ Token updated.");
    }

    config.save().context("Failed to save config")?;
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

    let token_ok = Auth::get_token(&config.email).is_ok() || config.token.is_some();
    if token_ok {
        println!("  Token: ✓ stored");
    } else {
        println!("  Token: ✗ not found — run `jira auth login`");
    }

    if let Some(project) = &config.project {
        println!("  Default project: {project}");
    }

    Ok(())
}
