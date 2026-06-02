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
    #[value(name = "opencode")]
    OpenCode,
    GenericJson,
    Antigravity,
    #[value(name = "antigravity-cli")]
    AntigravityCli,
}

impl std::fmt::Display for McpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            McpClient::ClaudeCode => "claude-code        (writes ~/.claude.json mcpServers)",
            McpClient::ClaudeDesktop => {
                "claude-desktop     (writes claude_desktop_config.json in Claude support dir)"
            }
            McpClient::Cursor => "cursor             (writes ~/.cursor/mcp.json)",
            McpClient::Codex => "codex              (delegates to `codex mcp add`)",
            McpClient::GeminiCli => "gemini-cli         (delegates to `gemini mcp add`)",
            McpClient::OpenCode => "opencode           (writes opencode.jsonc)",
            McpClient::Antigravity => {
                "antigravity        (writes ~/.gemini/antigravity/mcp_config.json)"
            }
            McpClient::AntigravityCli => {
                "antigravity-cli    (writes ~/.gemini/config/mcp_config.json)"
            }
            McpClient::GenericJson => "generic-json       (print snippet only, no file changes)",
        };
        f.write_str(label)
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
                None => run_interactive_prereqs_and_pick(&command)?,
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
            McpClient::ClaudeDesktop,
            McpClient::Cursor,
            McpClient::Codex,
            McpClient::GeminiCli,
            McpClient::OpenCode,
            McpClient::Antigravity,
            McpClient::AntigravityCli,
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
    let resolved_command = resolve_command_for_client(&client, command, dry_run)?;
    let spec = server_spec(&client, name, &resolved_command, transport);

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
    let mcp_servers = ensure_object_path(&mut root, &target.top_level_key)?;

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
            McpClient::OpenCode,
            McpClient::GenericJson,
            McpClient::Antigravity,
            McpClient::AntigravityCli,
        ],
    };

    for client in clients {
        match describe_client(&client) {
            ClientDescriptor::FileTarget {
                label,
                path,
                note,
                top_level_key,
            } => {
                if path.exists() {
                    println!("[ok] {} target path exists: {}", label, path.display());
                    match load_json_object(&path) {
                        Ok(root) => match lookup_object_path(&root, top_level_key) {
                            Ok(Some(entries)) if !entries.is_empty() => {
                                let suffix = if entries.len() == 1 { "y" } else { "ies" };
                                println!(
                                    "[ok] {} config has {} entr{} under {}",
                                    label,
                                    entries.len(),
                                    suffix,
                                    top_level_key
                                );
                            }
                            Ok(Some(_)) => {
                                println!(
                                    "[warn] {} config exists but {} is empty",
                                    label, top_level_key
                                );
                            }
                            Ok(None) => {
                                println!(
                                    "[warn] {} config exists but has no {} object yet",
                                    label, top_level_key
                                );
                            }
                            Err(err) => {
                                println!("[warn] {} config exists but {}", label, err);
                                hard_failures += 1;
                            }
                        },
                        Err(err) => {
                            println!("[warn] {} config could not be parsed: {err:#}", label);
                            hard_failures += 1;
                        }
                    }
                } else {
                    println!(
                        "[info] {} target path will be created: {}",
                        label,
                        path.display()
                    );
                }
                if matches!(client, McpClient::AntigravityCli) {
                    if let Some(legacy_path) = antigravity_cli_legacy_settings_path() {
                        if legacy_path.exists() && legacy_path != path {
                            println!(
                                "[info] antigravity-cli legacy config path also exists: {}",
                                legacy_path.display()
                            );
                            println!(
                                "       Current helper target is {} because newer antigravity-cli builds appear to read that file instead.",
                                path.display()
                            );
                        }
                    }
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
        top_level_key: &'static str,
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

fn server_spec(client: &McpClient, name: &str, command: &str, transport: &str) -> ServerSpec {
    match client {
        McpClient::OpenCode => {
            let file_entry = json!({
                "type": "local",
                "command": [command, "serve", "--transport", transport],
                "enabled": true,
            });
            let json_snippet = json!({
                "mcp": {
                    name: file_entry.clone()
                }
            });
            ServerSpec {
                file_entry,
                json_snippet,
            }
        }
        _ => {
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
    }
}

fn install_target(client: &McpClient) -> Result<InstallTarget> {
    let home = home_dir().context("Could not determine home directory")?;

    let (label, path, top_level_key) = match client {
        McpClient::ClaudeCode => (
            "claude-code",
            config_path_from_env_or_default("CLAUDE_CODE_CONFIG", home.join(".claude.json")),
            "mcpServers",
        ),
        McpClient::ClaudeDesktop => (
            "claude-desktop",
            config_path_from_env_or_default(
                "CLAUDE_DESKTOP_CONFIG",
                claude_desktop_settings_path(&home),
            ),
            "mcpServers",
        ),
        McpClient::Cursor => (
            "cursor",
            config_path_from_env_or_default("CURSOR_CONFIG", home.join(".cursor/mcp.json")),
            "mcpServers",
        ),
        McpClient::OpenCode => ("opencode", opencode_settings_path(&home), "mcp"),
        McpClient::GeminiCli | McpClient::Codex | McpClient::GenericJson => unreachable!(),
        McpClient::Antigravity => (
            "antigravity",
            config_path_from_env_or_default(
                "ANTIGRAVITY_CONFIG",
                home.join(".gemini/antigravity/mcp_config.json"),
            ),
            "mcpServers",
        ),
        McpClient::AntigravityCli => (
            "antigravity-cli",
            config_path_from_env_or_default(
                "ANTIGRAVITY_CLI_CONFIG",
                home.join(".gemini/config/mcp_config.json"),
            ),
            "mcpServers",
        ),
    };

    Ok(InstallTarget {
        label,
        path,
        top_level_key: top_level_key.to_string(),
    })
}

fn describe_client(client: &McpClient) -> ClientDescriptor {
    match client {
        McpClient::ClaudeCode => ClientDescriptor::FileTarget {
            label: "claude-code",
            path: install_target(client)
                .map(|t| t.path)
                .unwrap_or_else(|_| PathBuf::from("~/.claude.json")),
            note: "Writes user-level config at ~/.claude.json (mcpServers).",
            top_level_key: "mcpServers",
        },
        McpClient::ClaudeDesktop => ClientDescriptor::FileTarget {
            label: "claude-desktop",
            path: install_target(client)
                .map(|t| t.path)
                .unwrap_or_else(|_| PathBuf::from("claude_desktop_config.json")),
            note: "macOS: ~/Library/Application Support/Claude/claude_desktop_config.json; Windows: %APPDATA%\\Claude\\claude_desktop_config.json.",
            top_level_key: "mcpServers",
        },
        McpClient::Cursor => ClientDescriptor::FileTarget {
            label: "cursor",
            path: install_target(client)
                .map(|t| t.path)
                .unwrap_or_else(|_| PathBuf::from("~/.cursor/mcp.json")),
            note: "Provisional path until verified in a real Cursor install.",
            top_level_key: "mcpServers",
        },
        McpClient::OpenCode => ClientDescriptor::FileTarget {
            label: "opencode",
            path: install_target(client)
                .map(|t| t.path)
                .unwrap_or_else(|_| PathBuf::from("~/.config/opencode/opencode.jsonc")),
            note: "Writes JSONC at ~/.config/opencode/opencode.jsonc by default.",
            top_level_key: "mcp",
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
        McpClient::Antigravity => ClientDescriptor::FileTarget {
            label: "antigravity",
            path: install_target(client)
                .map(|t| t.path)
                .unwrap_or_else(|_| PathBuf::from("~/.gemini/antigravity/mcp_config.json")),
            note: "Writes user-level config at ~/.gemini/antigravity/mcp_config.json (mcpServers).",
            top_level_key: "mcpServers",
        },
        McpClient::AntigravityCli => ClientDescriptor::FileTarget {
            label: "antigravity-cli",
            path: install_target(client)
                .map(|t| t.path)
                .unwrap_or_else(|_| PathBuf::from("~/.gemini/config/mcp_config.json")),
            note: "Writes user-level config at ~/.gemini/config/mcp_config.json (mcpServers).",
            top_level_key: "mcpServers",
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

fn claude_desktop_settings_path(home: &Path) -> PathBuf {
    if cfg!(target_os = "macos") {
        home.join("Library/Application Support/Claude/claude_desktop_config.json")
    } else if cfg!(target_os = "windows") {
        env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("AppData/Roaming"))
            .join("Claude/claude_desktop_config.json")
    } else {
        env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"))
            .join("Claude/claude_desktop_config.json")
    }
}

fn opencode_settings_path(home: &Path) -> PathBuf {
    if let Some(path) = env::var_os("OPENCODE_CONFIG") {
        return PathBuf::from(path);
    }

    if cfg!(target_os = "macos") {
        home.join(".config/opencode/opencode.jsonc")
    } else if cfg!(target_os = "windows") {
        env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("AppData/Roaming"))
            .join("opencode/opencode.jsonc")
    } else {
        env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"))
            .join("opencode/opencode.jsonc")
    }
}

fn resolve_command_for_client(client: &McpClient, command: &str, dry_run: bool) -> Result<String> {
    if matches!(client, McpClient::GenericJson) {
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
        eprintln!("       Install it with `cargo install jira-mcp`, download `jirac-mcp` from https://github.com/mulhamna/jira-commands/releases, or pass `--command /path/to/jirac-mcp`.");
        return Ok(command.to_string());
    }

    bail!(
        "MCP server command '{}' was not found on PATH. Install it first:
  cargo install jira-mcp
  # or download `jirac-mcp` from https://github.com/mulhamna/jira-commands/releases
  # or pass --command /path/to/jirac-mcp",
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

fn antigravity_cli_legacy_settings_path() -> Option<PathBuf> {
    home_dir().map(|home| home.join(".gemini/antigravity-cli/settings.json"))
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

    let sanitized = strip_json_comments(&raw);
    let value: Value = serde_json::from_str(&sanitized)
        .with_context(|| format!("Config file {} is not valid JSON/JSONC", path.display()))?;

    match value {
        Value::Object(map) => Ok(map),
        _ => bail!(
            "Config file {} must contain a top-level JSON object",
            path.display()
        ),
    }
}

fn strip_json_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                out.push(ch);
            }
            '/' if matches!(chars.peek(), Some('/')) => {
                chars.next();
                for next in chars.by_ref() {
                    if next == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            '/' if matches!(chars.peek(), Some('*')) => {
                chars.next();
                let mut prev = '\0';
                for next in chars.by_ref() {
                    if prev == '*' && next == '/' {
                        break;
                    }
                    if next == '\n' {
                        out.push('\n');
                    }
                    prev = next;
                }
            }
            _ => out.push(ch),
        }
    }

    out
}

fn lookup_object_path<'a>(
    root: &'a Map<String, Value>,
    path: &str,
) -> Result<Option<&'a Map<String, Value>>> {
    let mut current = root;
    for key in path.split('.') {
        match current.get(key) {
            Some(Value::Object(map)) => current = map,
            Some(_) => bail!("Field '{}' in path '{}' must be a JSON object", key, path),
            None => return Ok(None),
        }
    }
    Ok(Some(current))
}

fn ensure_object_path<'a>(
    root: &'a mut Map<String, Value>,
    path: &str,
) -> Result<&'a mut Map<String, Value>> {
    let mut current = root;
    for key in path.split('.') {
        current = ensure_object_field(current, key)?;
    }
    Ok(current)
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
        let snippet =
            server_spec(&McpClient::GenericJson, "jira", "jirac-mcp", "stdio").json_snippet;
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
    fn lookup_object_path_returns_none_for_missing_path() {
        let root = Map::new();
        assert!(lookup_object_path(&root, "mcpServers").unwrap().is_none());
    }

    #[test]
    fn lookup_object_path_rejects_non_object_segment() {
        let mut root = Map::new();
        root.insert("mcpServers".into(), Value::String("bad".into()));
        let err = lookup_object_path(&root, "mcpServers.jira").unwrap_err();
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
    fn opencode_snippet_uses_local_command_array() {
        let snippet =
            server_spec(&McpClient::OpenCode, "jira", "/tmp/jirac-mcp", "stdio").json_snippet;
        let rendered = serde_json::to_string_pretty(&snippet).unwrap();
        assert!(rendered.contains("\"mcp\""));
        assert!(rendered.contains("\"type\": \"local\""));
        assert!(rendered.contains("/tmp/jirac-mcp"));
    }

    #[test]
    fn load_json_object_accepts_jsonc_comments() {
        let path =
            std::env::temp_dir().join(format!("jirac-opencode-test-{}.jsonc", std::process::id()));
        fs::write(&path, "// top-level comment\n{\n  /* block */\n  \"mcp\": {\n    \"jira\": {\"enabled\": true}\n  }\n}\n").unwrap();
        let parsed = load_json_object(&path).unwrap();
        fs::remove_file(&path).ok();
        assert!(parsed.contains_key("mcp"));
    }

    #[test]
    fn resolve_command_path_finds_absolute_path() {
        let path = resolve_command_path("/bin/sh").unwrap();
        assert_eq!(path, PathBuf::from("/bin/sh"));
    }

    #[test]
    fn antigravity_install_target_uses_mcp_servers() {
        let target = install_target(&McpClient::Antigravity).unwrap();
        assert_eq!(target.label, "antigravity");
        assert_eq!(target.top_level_key, "mcpServers");
        assert!(target.path.ends_with(".gemini/antigravity/mcp_config.json"));
    }

    #[test]
    fn antigravity_cli_install_target_uses_shared_gemini_mcp_config() {
        let target = install_target(&McpClient::AntigravityCli).unwrap();
        assert_eq!(target.label, "antigravity-cli");
        assert_eq!(target.top_level_key, "mcpServers");
        assert!(target.path.ends_with(".gemini/config/mcp_config.json"));
    }

    #[test]
    fn antigravity_snippet_uses_standard_mcp_servers_shape() {
        let snippet =
            server_spec(&McpClient::Antigravity, "jira", "jirac-mcp", "stdio").json_snippet;
        let rendered = serde_json::to_string_pretty(&snippet).unwrap();
        assert!(rendered.contains("\"mcpServers\""));
        assert!(rendered.contains("\"jira\""));
        assert!(rendered.contains("\"jirac-mcp\""));
    }

    #[test]
    fn antigravity_cli_snippet_uses_standard_mcp_servers_shape() {
        let snippet =
            server_spec(&McpClient::AntigravityCli, "jira", "jirac-mcp", "stdio").json_snippet;
        let rendered = serde_json::to_string_pretty(&snippet).unwrap();
        assert!(rendered.contains("\"mcpServers\""));
        assert!(rendered.contains("\"jira\""));
        assert!(rendered.contains("\"jirac-mcp\""));
    }

    #[test]
    fn resolve_command_for_client_rejects_missing_binary() {
        let err = resolve_command_for_client(
            &McpClient::GeminiCli,
            "definitely-not-a-real-binary",
            false,
        )
        .unwrap_err();
        assert!(err.to_string().contains("cargo install jira-mcp"));
    }

    #[test]
    fn resolve_command_for_file_clients_rejects_missing_binary() {
        let err = resolve_command_for_client(
            &McpClient::AntigravityCli,
            "definitely-not-a-real-binary",
            false,
        )
        .unwrap_err();
        assert!(err.to_string().contains("--command /path/to/jirac-mcp"));
    }
}
