use anyhow::{bail, Context, Result};
use clap::{Subcommand, ValueEnum};
use jira_core::config::JiraConfig;
use serde_json::{json, Map, Value};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Subcommand)]
pub enum McpCommand {
    /// Install jirac-mcp into a supported MCP client config
    Install {
        #[arg(long, value_enum)]
        client: McpClient,
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
    /// Check MCP install prerequisites and target readiness
    Doctor {
        #[arg(long, value_enum)]
        client: Option<McpClient>,
        #[arg(long, default_value = "jirac-mcp")]
        command: String,
    },
}

#[derive(Debug, Clone, ValueEnum)]
pub enum McpClient {
    ClaudeCode,
    ClaudeDesktop,
    Cursor,
    GeminiCli,
    Codex,
    GenericJson,
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
        } => install_client(client, &name, &command, &transport, print, dry_run, force),
        McpCommand::Doctor { client, command } => doctor(client, &command),
    }
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
    let resolved_command = resolve_command_for_client(&client, command, dry_run)?;
    let spec = server_spec(name, &resolved_command, transport);

    if matches!(client, McpClient::GenericJson) {
        print_snippet(&spec.json_snippet)?;
        return Ok(());
    }

    if let Some(adapter) = client_adapter(&client) {
        let preview = adapter.preview_command(name, &resolved_command, transport, force);
        if print || dry_run {
            println!("{}", preview);
        }
        if dry_run {
            println!("Dry run, no client command executed.");
            return Ok(());
        }
        adapter.install(name, &resolved_command, transport, force)?;
        println!("Installed MCP entry '{}' via {} CLI", name, adapter.label);
        return Ok(());
    }

    let target = install_target(&client)?;
    let mut root = load_json_object(&target.path)?;
    let mcp_servers = ensure_object_field(&mut root, &target.top_level_key)?;

    if let Some(existing) = mcp_servers.get(name) {
        if existing == &spec.file_entry {
            if print || dry_run {
                print_snippet(&spec.json_snippet)?;
            }
            println!(
                "MCP entry '{}' already configured at {}",
                name,
                target.path.display()
            );
            return Ok(());
        }

        if !force {
            bail!(
                "MCP entry '{}' already exists at {}. Re-run with --force to overwrite.",
                name,
                target.path.display()
            );
        }
    }

    mcp_servers.insert(name.to_string(), spec.file_entry.clone());

    if print || dry_run {
        print_snippet(&spec.json_snippet)?;
    }

    if dry_run {
        println!(
            "Dry run, no file written. Target: {}",
            target.path.display()
        );
        return Ok(());
    }

    backup_if_exists(&target.path)?;
    write_json_object(&target.path, &root)?;
    println!(
        "Installed MCP entry '{}' for {} at {}",
        name,
        target.label,
        target.path.display()
    );
    Ok(())
}

fn doctor(client: Option<McpClient>, command: &str) -> Result<()> {
    let mut hard_failures = 0;

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

    let clients = match client {
        Some(one) => vec![one],
        None => vec![
            McpClient::ClaudeCode,
            McpClient::ClaudeDesktop,
            McpClient::Cursor,
            McpClient::GeminiCli,
            McpClient::Codex,
            McpClient::GenericJson,
        ],
    };

    for client in clients {
        match describe_client(&client) {
            ClientDescriptor::FileTarget { label, path, note } => {
                if path.exists() {
                    println!("[ok] {} target path exists: {}", label, path.display());
                } else {
                    println!(
                        "[info] {} target path will be created: {}",
                        label,
                        path.display()
                    );
                }
                if !note.is_empty() {
                    println!("       {}", note);
                }
            }
            ClientDescriptor::Delegated {
                label,
                program,
                note,
            } => {
                if command_exists(program) {
                    println!("[ok] {} CLI found: {}", label, program);
                } else {
                    println!("[warn] {} CLI missing: {}", label, program);
                    hard_failures += 1;
                }
                if !note.is_empty() {
                    println!("       {}", note);
                }
            }
            ClientDescriptor::SnippetOnly { label, note } => {
                println!("[ok] {} available as print-only target", label);
                if !note.is_empty() {
                    println!("       {}", note);
                }
            }
        }
    }

    if hard_failures > 0 {
        bail!("MCP doctor found {} blocking issue(s)", hard_failures);
    }

    println!("MCP doctor finished. Warnings above are setup guidance, not blocking failures.");
    Ok(())
}

struct InstallTarget {
    label: &'static str,
    path: PathBuf,
    top_level_key: String,
}

struct ServerSpec {
    file_entry: Value,
    json_snippet: Value,
}

enum ClientDescriptor {
    FileTarget {
        label: &'static str,
        path: PathBuf,
        note: &'static str,
    },
    Delegated {
        label: &'static str,
        program: &'static str,
        note: &'static str,
    },
    SnippetOnly {
        label: &'static str,
        note: &'static str,
    },
}

fn server_spec(name: &str, command: &str, transport: &str) -> ServerSpec {
    let file_entry = json!({
        "command": command,
        "args": ["serve", "--transport", transport]
    });
    let json_snippet = json!({
        "mcpServers": {
            name: file_entry.clone()
        }
    });
    ServerSpec {
        file_entry,
        json_snippet,
    }
}

fn install_target(client: &McpClient) -> Result<InstallTarget> {
    let home = home_dir().context("Could not determine home directory")?;

    let (label, path) = match client {
        McpClient::ClaudeCode => (
            "claude-code",
            config_path_from_env_or_default("CLAUDE_CODE_CONFIG", home.join(".mcp.json")),
        ),
        McpClient::ClaudeDesktop => (
            "claude-desktop",
            config_path_from_env_or_default("CLAUDE_DESKTOP_CONFIG", home.join(".claude.json")),
        ),
        McpClient::Cursor => (
            "cursor",
            config_path_from_env_or_default("CURSOR_CONFIG", home.join(".cursor/mcp.json")),
        ),
        McpClient::GeminiCli | McpClient::Codex | McpClient::GenericJson => unreachable!(),
    };

    Ok(InstallTarget {
        label,
        path,
        top_level_key: "mcpServers".to_string(),
    })
}

fn describe_client(client: &McpClient) -> ClientDescriptor {
    match client {
        McpClient::ClaudeCode => ClientDescriptor::FileTarget {
            label: "claude-code",
            path: install_target(client)
                .map(|t| t.path)
                .unwrap_or_else(|_| PathBuf::from(".mcp.json")),
            note: "Writes project-style JSON at .mcp.json by default.",
        },
        McpClient::ClaudeDesktop => ClientDescriptor::FileTarget {
            label: "claude-desktop",
            path: install_target(client)
                .map(|t| t.path)
                .unwrap_or_else(|_| PathBuf::from("~/.claude.json")),
            note: "Writes user-level JSON at ~/.claude.json by default.",
        },
        McpClient::Cursor => ClientDescriptor::FileTarget {
            label: "cursor",
            path: install_target(client)
                .map(|t| t.path)
                .unwrap_or_else(|_| PathBuf::from("~/.cursor/mcp.json")),
            note: "Provisional path until verified in a real Cursor install.",
        },
        McpClient::GeminiCli => ClientDescriptor::Delegated {
            label: "gemini-cli",
            program: "gemini",
            note: "Delegates to `gemini mcp add -s user ...`.",
        },
        McpClient::Codex => ClientDescriptor::Delegated {
            label: "codex",
            program: "codex",
            note: "Delegates to `codex mcp add ...`.",
        },
        McpClient::GenericJson => ClientDescriptor::SnippetOnly {
            label: "generic-json",
            note: "Prints a portable JSON snippet instead of writing a file.",
        },
    }
}

fn client_adapter(client: &McpClient) -> Option<ClientAdapter> {
    match client {
        McpClient::GeminiCli => Some(ClientAdapter {
            label: "gemini",
            program: "gemini",
            build_steps: gemini_steps,
        }),
        McpClient::Codex => Some(ClientAdapter {
            label: "codex",
            program: "codex",
            build_steps: codex_steps,
        }),
        _ => None,
    }
}

struct ClientAdapter {
    label: &'static str,
    program: &'static str,
    build_steps: fn(&str, &str, &str, bool) -> Vec<Vec<String>>,
}

impl ClientAdapter {
    fn preview_command(&self, name: &str, command: &str, transport: &str, force: bool) -> String {
        let steps = (self.build_steps)(name, command, transport, force);
        steps
            .iter()
            .map(|args| format!("{} {}", self.program, shell_join(args)))
            .collect::<Vec<_>>()
            .join(" && ")
    }

    fn install(&self, name: &str, command: &str, transport: &str, force: bool) -> Result<()> {
        let steps = (self.build_steps)(name, command, transport, force);
        for args in steps {
            let status = Command::new(self.program)
                .args(&args)
                .status()
                .with_context(|| format!("Failed to launch {}", self.program))?;
            if !status.success() {
                bail!("{} exited with status {}", self.program, status);
            }
        }
        Ok(())
    }
}

fn gemini_steps(name: &str, command: &str, _transport: &str, force: bool) -> Vec<Vec<String>> {
    let mut steps = vec![];
    if force {
        steps.push(vec!["mcp".into(), "remove".into(), name.into()]);
    }
    steps.push(vec![
        "mcp".into(),
        "add".into(),
        "-s".into(),
        "user".into(),
        name.into(),
        command.into(),
        "serve".into(),
    ]);
    steps
}

fn codex_steps(name: &str, command: &str, transport: &str, force: bool) -> Vec<Vec<String>> {
    let mut steps = vec![];
    if force {
        steps.push(vec!["mcp".into(), "remove".into(), name.into()]);
    }
    steps.push(vec![
        "mcp".into(),
        "add".into(),
        name.into(),
        "--".into(),
        command.into(),
        "serve".into(),
        "--transport".into(),
        transport.into(),
    ]);
    steps
}

fn config_path_from_env_or_default(env_key: &str, default: PathBuf) -> PathBuf {
    env::var_os(env_key).map(PathBuf::from).unwrap_or(default)
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

fn resolve_command_for_client(client: &McpClient, command: &str, dry_run: bool) -> Result<String> {
    if !matches!(client, McpClient::GeminiCli | McpClient::Codex) {
        return Ok(command.to_string());
    }

    if let Some(path) = resolve_command_path(command) {
        return Ok(path.display().to_string());
    }

    if dry_run {
        return Ok(command.to_string());
    }

    bail!(
        "MCP server command '{}' was not found on PATH. Install it first or pass --command with an absolute path.",
        command
    )
}

fn resolve_command_path(command: &str) -> Option<PathBuf> {
    let path = PathBuf::from(command);
    if path.components().count() > 1 || path.is_absolute() {
        return path.is_file().then_some(path);
    }

    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths).find_map(|dir| {
            let candidate = dir.join(command);
            candidate.is_file().then_some(candidate)
        })
    })
}

fn command_exists(command: &str) -> bool {
    resolve_command_path(command).is_some()
}

fn load_json_object(path: &Path) -> Result<Map<String, Value>> {
    if !path.exists() {
        return Ok(Map::new());
    }

    let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file {}", path.display()))?;

    if raw.trim().is_empty() {
        return Ok(Map::new());
    }

    let value: Value = serde_json::from_str(&raw)
        .with_context(|| format!("Config file {} is not valid JSON", path.display()))?;

    match value {
        Value::Object(map) => Ok(map),
        _ => bail!(
            "Config file {} must contain a top-level JSON object",
            path.display()
        ),
    }
}

fn ensure_object_field<'a>(
    root: &'a mut Map<String, Value>,
    key: &str,
) -> Result<&'a mut Map<String, Value>> {
    let value = root
        .entry(key.to_string())
        .or_insert_with(|| Value::Object(Map::new()));

    match value {
        Value::Object(map) => Ok(map),
        _ => bail!("Top-level field '{}' must be a JSON object", key),
    }
}

fn backup_if_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let backup_path = path.with_extension(format!(
        "{}.bak",
        path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("json")
    ));

    fs::copy(path, &backup_path).with_context(|| {
        format!(
            "Failed to create backup {} from {}",
            backup_path.display(),
            path.display()
        )
    })?;
    Ok(())
}

fn write_json_object(path: &Path, root: &Map<String, Value>) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent directory for {}", path.display()))?;
    }

    let body = serde_json::to_string_pretty(root)?;
    fs::write(path, format!("{body}\n"))
        .with_context(|| format!("Failed to write config file {}", path.display()))?;
    Ok(())
}

fn print_snippet(server_entry: &Value) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(server_entry)?);
    Ok(())
}

fn shell_join(args: &[String]) -> String {
    args.iter()
        .map(|arg| shell_escape(arg))
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_escape(input: &str) -> String {
    if input
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '/' | '.' | ':' | '='))
    {
        input.to_string()
    } else {
        format!("'{}'", input.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_json_snippet_contains_server_name() {
        let snippet = server_spec("jira", "jirac-mcp", "stdio").json_snippet;
        let rendered = serde_json::to_string_pretty(&snippet).unwrap();
        assert!(rendered.contains("\"mcpServers\""));
        assert!(rendered.contains("\"jira\""));
        assert!(rendered.contains("\"jirac-mcp\""));
    }

    #[test]
    fn ensure_object_field_rejects_non_object() {
        let mut root = Map::new();
        root.insert("mcpServers".into(), Value::String("bad".into()));
        let err = ensure_object_field(&mut root, "mcpServers").unwrap_err();
        assert!(err.to_string().contains("must be a JSON object"));
    }

    #[test]
    fn codex_preview_includes_transport() {
        let adapter = client_adapter(&McpClient::Codex).unwrap();
        let preview = adapter.preview_command("jira", "jirac-mcp", "stdio", false);
        assert!(preview.contains("codex mcp add jira -- jirac-mcp serve --transport stdio"));
    }

    #[test]
    fn gemini_preview_matches_cli_shape() {
        let adapter = client_adapter(&McpClient::GeminiCli).unwrap();
        let preview = adapter.preview_command("jira", "jirac-mcp", "stdio", false);
        assert!(preview.contains("gemini mcp add -s user jira jirac-mcp serve"));
    }

    #[test]
    fn resolve_command_path_finds_absolute_path() {
        let path = resolve_command_path("/bin/sh").unwrap();
        assert_eq!(path, PathBuf::from("/bin/sh"));
    }

    #[test]
    fn resolve_command_for_client_rejects_missing_binary() {
        let err = resolve_command_for_client(&McpClient::GeminiCli, "definitely-not-a-real-binary", false)
            .unwrap_err();
        assert!(err
            .to_string()
            .contains("was not found on PATH"));
    }
}
