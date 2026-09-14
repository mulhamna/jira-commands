use anyhow::{bail, Context, Result};
use clap::{Subcommand, ValueEnum};
use jira_core::config::JiraConfig;
use kurir::{
    doctor as kurir_doctor, register, registration::snippet_for, Harness, RegistrationOptions,
    Scope, ServerSpec,
};
use serde_json::Value;
use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, Subcommand)]
pub enum McpCommand {
    /// Install jirac-mcp into a supported MCP client config.
    ///
    /// Omit `--client` to run interactively: prereqs are checked and a picker is shown.
    Install {
        #[arg(long, value_enum)]
        client: Option<McpClient>,
        #[arg(long, default_value = "jira")]
        name: String,
        #[arg(long, default_value = "jirac-mcp")]
        command: String,
        #[arg(long, default_value = "stdio")]
        transport: String,
        #[arg(long)]
        print: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        force: bool,
    },
    /// Check MCP install prerequisites and target readiness.
    Doctor {
        #[arg(long, value_enum)]
        client: Option<McpClient>,
        #[arg(long, default_value = "jirac-mcp")]
        command: String,
    },
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum McpClient {
    ClaudeCode,
    #[value(name = "claude-code-cli")]
    ClaudeCodeCli,
    ClaudeDesktop,
    Cursor,
    GeminiCli,
    Codex,
    #[value(name = "vscode")]
    Vscode,
    #[value(name = "copilot-cli")]
    CopilotCli,
    #[value(name = "opencode")]
    OpenCode,
    Windsurf,
    Zed,
    #[value(name = "openclaw")]
    OpenClaw,
    Hermes,
    #[value(name = "antigravity-cli", alias = "antigravity")]
    AntigravityCli,
    AntigravityDesktop,
    #[value(name = "generic-json", alias = "omp")]
    GenericJson,
}

impl McpClient {
    fn harness(self) -> Harness {
        const HARNESSES: [Harness; 16] = [
            Harness::ClaudeCode,
            Harness::ClaudeCodeCli,
            Harness::ClaudeDesktop,
            Harness::Cursor,
            Harness::GeminiCli,
            Harness::Codex,
            Harness::Vscode,
            Harness::CopilotCli,
            Harness::OpenCode,
            Harness::Windsurf,
            Harness::Zed,
            Harness::OpenClaw,
            Harness::Hermes,
            Harness::AntigravityCli,
            Harness::AntigravityDesktop,
            Harness::Omp,
        ];
        HARNESSES[self as usize]
    }
}

impl std::fmt::Display for McpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const LABELS: [&str; 16] = [
            "claude-code        (Kurir: ~/.claude.json)",
            "claude-code-cli   (Kurir: delegates to `claude mcp add`)",
            "claude-desktop     (Kurir: Claude Desktop config)",
            "cursor             (Kurir: Cursor mcp.json)",
            "gemini-cli         (Kurir: delegates to `gemini mcp add`)",
            "codex              (Kurir: delegates to `codex mcp add`)",
            "vscode             (Kurir: delegates to `code --add-mcp`)",
            "copilot-cli        (Kurir: ~/.copilot/mcp-config.json)",
            "opencode           (Kurir: opencode.json)",
            "windsurf           (Kurir: Windsurf config)",
            "zed                (Kurir: Zed settings)",
            "openclaw           (Kurir: openclaw.json)",
            "hermes             (Kurir: delegates to `hermes mcp add`)",
            "antigravity-cli    (Kurir: Antigravity CLI config)",
            "antigravity-desktop (Kurir: Antigravity Desktop config)",
            "generic-json       (Kurir: print snippet only)",
        ];
        f.write_str(LABELS[*self as usize])
    }
}

pub fn handle(command: McpCommand) -> Result<()> {
    match command {
        McpCommand::Install {
            client,
            name,
            command,
            transport,
            print,
            dry_run,
            force,
        } => {
            let resolved_client = match client {
                Some(c) => c,
                None => {
                    crate::cli::interactive::require_interactive(
                        "MCP client",
                        "--client <target>",
                    )?;
                    run_interactive_prereqs_and_pick(&command)?
                }
            };
            install_client(
                resolved_client,
                &name,
                &command,
                &transport,
                print,
                dry_run,
                force,
            )
        }
        McpCommand::Doctor { client, command } => doctor(client, &command),
    }
}

fn run_interactive_prereqs_and_pick(server_command: &str) -> Result<McpClient> {
    use inquire::Select;

    println!("jirac mcp install — interactive setup");
    println!("─────────────────────────────────────");

    let mcp_bin = resolve_command_path(server_command);
    match &mcp_bin {
        Some(path) => println!("[ok]   MCP server binary: {}", path.display()),
        None => println!("[warn] MCP server binary not on PATH: {}", server_command),
    }

    let jirac_bin = resolve_command_path("jirac");
    match &jirac_bin {
        Some(path) => println!("[ok]   jirac CLI:         {}", path.display()),
        None => println!("[info] jirac CLI not on PATH (optional, used for TUI and auth login)"),
    }

    let jira = JiraConfig::load().unwrap_or_default();
    let auth_ok = !jira.base_url.trim().is_empty()
        && jira.token_present()
        && (!jira.requires_user_identity() || !jira.email.trim().is_empty());

    if auth_ok {
        println!("[ok]   Jira auth config present");
    } else {
        println!("[fail] Jira auth config missing or incomplete");
    }

    if mcp_bin.is_none() {
        bail!(
            "MCP server binary '{}' not found on PATH. Install it first:\n  cargo install jira-mcp\n  # or download from https://github.com/mulhamna/jira-commands/releases",
            server_command
        );
    }

    if !auth_ok {
        bail!(
            "Jira credentials not configured. Set them up first:\n\n  Option A (recommended): install the jirac CLI and run auth login\n    cargo install jira-commands\n    jirac auth login\n\n  Option B: edit the config file directly\n    ~/.config/jirac/config.toml (or platform equivalent)\n\nThen re-run `jirac mcp install` to register the MCP entry."
        );
    }

    println!();
    let choice = Select::new(
        "Pick the MCP client to install into:",
        vec![
            McpClient::ClaudeCode,
            McpClient::ClaudeCodeCli,
            McpClient::ClaudeDesktop,
            McpClient::Cursor,
            McpClient::Codex,
            McpClient::GeminiCli,
            McpClient::Vscode,
            McpClient::CopilotCli,
            McpClient::OpenCode,
            McpClient::Windsurf,
            McpClient::Zed,
            McpClient::OpenClaw,
            McpClient::Hermes,
            McpClient::AntigravityCli,
            McpClient::AntigravityDesktop,
            McpClient::GenericJson,
        ],
    )
    .prompt()
    .context("MCP client selection cancelled")?;

    Ok(choice)
}

fn install_client(
    client: McpClient,
    name: &str,
    command: &str,
    transport: &str,
    print: bool,
    dry_run: bool,
    force: bool,
) -> Result<()> {
    let harness = client.harness();
    let resolved_command = resolve_command_for_client(client, command, dry_run)?;
    let spec = ServerSpec::stdio(
        name,
        resolved_command,
        vec!["serve".into(), "--transport".into(), transport.into()],
    );

    if harness.is_snippet_only() {
        print_snippet(&snippet_for(&spec))?;
        return Ok(());
    }

    let options = RegistrationOptions {
        scope: Scope::User,
        force,
        dry_run,
        print,
        ..RegistrationOptions::default()
    };
    let result = register(harness, &spec, &options).context("Kurir MCP registration failed")?;

    if result.changed {
        println!("Installed MCP entry '{}' via Kurir for {}", name, harness);
    } else if result.action == "already-configured" {
        println!("MCP entry '{}' already configured via Kurir", name);
    } else if dry_run {
        println!("Dry run, no client config written via Kurir.");
    }
    Ok(())
}

fn doctor(client: Option<McpClient>, command: &str) -> Result<()> {
    println!("MCP doctor");
    println!("──────────");

    if let Some(path) = resolve_command_path(command) {
        println!("[ok] MCP server binary found: {}", path.display());
    } else {
        println!("[warn] MCP server binary not found on PATH: {}", command);
        println!("       Install `jirac-mcp` if you want to use the MCP helper end to end.");
    }

    let jira = JiraConfig::load().unwrap_or_default();
    if jira.base_url.trim().is_empty() {
        println!("[warn] Jira base URL not configured. Run `jirac auth login`.");
    } else if !jira.token_present() {
        println!("[warn] Jira token not configured. Run `jirac auth login`.");
    } else if jira.requires_user_identity() && jira.email.trim().is_empty() {
        println!("[warn] Jira user identity not configured. Run `jirac auth login`.");
    } else {
        println!("[ok] Jira auth config present");
    }

    let harness = client.map(McpClient::harness);
    let reports = kurir_doctor(harness, &RegistrationOptions::default())
        .context("Kurir MCP doctor failed")?;
    let mut hard_failures = 0;
    for report in reports {
        let status = if report.available {
            "ok"
        } else {
            hard_failures += 1;
            "warn"
        };
        let target = report
            .target
            .map_or_else(String::new, |path| format!(" ({})", path.display()));
        println!("[{status}] {}{}: {}", report.harness, target, report.detail);
    }

    if hard_failures > 0 {
        bail!("MCP doctor found {hard_failures} blocking issue(s)");
    }

    println!("MCP doctor finished. Warnings above are setup guidance, not blocking failures.");
    Ok(())
}

fn print_snippet(snippet: &Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(snippet)?);
    Ok(())
}

fn resolve_command_for_client(client: McpClient, command: &str, dry_run: bool) -> Result<String> {
    if client.harness().is_snippet_only() {
        return Ok(command.to_string());
    }

    if let Some(path) = resolve_command_path(command) {
        return Ok(path.display().to_string());
    }

    if dry_run {
        eprintln!(
            "[warn] MCP server command '{}' was not found on PATH.",
            command
        );
        eprintln!(
            "       Install it with `cargo install jira-mcp`, download `jirac-mcp` from https://github.com/mulhamna/jira-commands/releases, or pass `--command /path/to/jirac-mcp`."
        );
        return Ok(command.to_string());
    }

    bail!(
        "MCP server command '{}' was not found on PATH. Install it first:\n  cargo install jira-mcp\n  # or download `jirac-mcp` from https://github.com/mulhamna/jira-commands/releases\n  # or pass --command /path/to/jirac-mcp",
        command
    )
}

fn resolve_command_path(command: &str) -> Option<PathBuf> {
    let path = PathBuf::from(command);
    if path.components().count() > 1 || path.is_absolute() {
        return executable_path_candidates(&path)
            .into_iter()
            .find(|candidate| candidate.is_file());
    }

    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths).find_map(|dir| {
            let candidate = dir.join(command);
            executable_path_candidates(&candidate)
                .into_iter()
                .find(|resolved| resolved.is_file())
        })
    })
}

#[cfg(target_os = "windows")]
fn executable_path_candidates(path: &Path) -> Vec<PathBuf> {
    let mut candidates = vec![path.to_path_buf()];
    if path.extension().is_some() {
        return candidates;
    }

    let pathext = env::var_os("PATHEXT")
        .and_then(|value| value.into_string().ok())
        .unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".to_string());

    for ext in pathext
        .split(';')
        .map(str::trim)
        .filter(|ext| !ext.is_empty())
    {
        let trimmed = ext.trim_start_matches('.');
        if trimmed.is_empty() {
            continue;
        }
        candidates.push(path.with_extension(trimmed));
    }

    candidates
}

#[cfg(not(target_os = "windows"))]
fn executable_path_candidates(path: &Path) -> Vec<PathBuf> {
    vec![path.to_path_buf()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_spec_uses_stdio_and_preserves_server_transport() {
        let spec = ServerSpec::stdio(
            "jira",
            "/tmp/jirac-mcp",
            vec![
                "serve".into(),
                "--transport".into(),
                "streamable-http".into(),
            ],
        );

        assert_eq!(spec.name, "jira");
        assert_eq!(spec.command.as_deref(), Some("/tmp/jirac-mcp"));
        assert_eq!(spec.args[2], "streamable-http");
        assert_eq!(spec.transport, kurir::Transport::Stdio);
    }

    #[test]
    fn existing_client_names_map_to_kurir_harnesses() {
        assert_eq!(McpClient::ClaudeCode.harness(), Harness::ClaudeCode);
        assert_eq!(McpClient::Codex.harness(), Harness::Codex);
        assert_eq!(
            McpClient::AntigravityDesktop.harness(),
            Harness::AntigravityDesktop
        );
        assert_eq!(McpClient::GenericJson.harness(), Harness::Omp);
    }

    #[test]
    fn resolve_command_path_finds_absolute_path() {
        let path = resolve_command_path("/bin/sh").expect("shell exists");
        assert_eq!(path, PathBuf::from("/bin/sh"));
    }

    #[test]
    fn resolve_command_for_file_clients_rejects_missing_binary() {
        let err = resolve_command_for_client(
            McpClient::AntigravityCli,
            "definitely-not-a-real-binary",
            false,
        )
        .expect_err("missing binary");
        assert!(err.to_string().contains("cargo install jira-mcp"));
    }
}
