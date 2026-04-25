# jirac

Jira on the command line.

[![CI](https://github.com/mulhamna/jira-commands/actions/workflows/ci.yml/badge.svg)](https://github.com/mulhamna/jira-commands/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/jira-commands.svg)](https://crates.io/crates/jira-commands)
[![Homebrew](https://img.shields.io/badge/homebrew-mulhamna%2Ftap-orange)](https://github.com/mulhamna/homebrew-tap)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

`jirac` is a Jira command-line client written in Rust. It ships as a single binary with no runtime dependencies and runs on macOS, Linux, and Windows. It talks directly to the Jira REST API v3 and discovers custom fields at runtime, so there is nothing to configure beyond your credentials.

![jirac TUI preview](assets/readme/sample_tui.jpeg)

## Highlights

- **Interactive TUI** — browse, create, edit, transition, and assign issues without leaving the terminal
- **Custom fields** — discovered at runtime via the API, not hardcoded
- **Attachments** — upload files to any issue from the CLI
- **Worklogs** — add, list, and delete time entries
- **Bulk operations** — transition, update, or archive many issues at once with a single JQL query

## Comparison

| Feature                           |      **jirac**       | [jira-cli](https://github.com/ankitpokhrel/jira-cli) (Go) | [jira-cmd](https://github.com/palashkulsh/jira-cmd) (Node) |
| --------------------------------- | :------------------: | :--------------------------------------------------------: | :---------------------------------------------------------: |
| Single binary, no runtime deps    |          ✅          |                             ✅                             |                          ❌ (npm)                           |
| Interactive TUI                   |          ✅          |                             ✅                             |                              ❌                             |
| Jira REST API version             |          v3          |                          v2 / v3                           |                             v2                              |
| Custom fields (runtime discovery) |          ✅          |                    Partial (config-based)                  |                     Partial (field IDs)                     |
| Attachment upload                 |          ✅          |                             ❌                             |                              ❌                             |
| Worklogs (add / list / delete)    |          ✅          |                             ❌                             |                       Add / list only                       |
| Bulk transition                   |          ✅          |                             ❌                             |                              ❌                             |
| Bulk update                       |          ✅          |                             ❌                             |                              ❌                             |
| Issue archive                     |          ✅          |                             ❌                             |                              ❌                             |
| JQL builder (interactive)         |          ✅          |                             ❌                             |                              ❌                             |
| Raw API passthrough               |          ✅          |                             ❌                             |                              ❌                             |
| Cursor-based pagination           |          ✅          |                        ❌ (offset)                         |                         ❌ (offset)                         |
| MCP server                        |  ✅ (`jirac-mcp`)    |                             ❌                             |                              ❌                             |
| macOS / Linux / Windows           |   ✅ / ✅ / ✅       |                    ✅ / ✅ / Partial                       |                      ✅ / ✅ / ✅                           |
| Jira Server (on-prem)             |      Cloud only      |                       Cloud + Server                       |                        Cloud + Server                       |
- **JQL builder** — interactive prompt that helps you construct queries
- **Raw API passthrough** — call any Jira REST endpoint directly
- **MCP server** — expose Jira as typed tools for editors and AI agents ([docs](crates/jira-mcp/README.md))

## Install

```bash
# Homebrew (macOS / Linux)
brew tap mulhamna/tap && brew install jira-commands

# Cargo
cargo install jira-commands

# Windows (winget)
winget install mulhamna.jirac

# Windows (Chocolatey)
choco install jirac
```

More methods (install script, PowerShell, GitHub Releases): [INSTALL.md](INSTALL.md)

## Quick start

```bash
# Authenticate
jirac auth login

# List your assigned issues
jirac issue list

# View an issue
jirac issue view PROJ-123

# Create an issue (interactive)
jirac issue create -p PROJ

# Transition an issue
jirac issue transition PROJ-123 --to "In Progress"

# Launch the TUI
jirac tui -p PROJ
```

## Usage

### Issues

```bash
jirac issue list                                    # assigned to you
jirac issue list -p PROJ                            # by project
jirac issue list --jql "status = 'In Progress'"     # custom JQL

jirac issue view PROJ-123                           # view detail
jirac issue create -p PROJ                          # create (interactive)
jirac issue create -p PROJ --type Bug --summary "Login fails on Safari"

jirac issue update PROJ-123 --summary "New title"
jirac issue update PROJ-123 --assignee user@co.com

jirac issue transition PROJ-123                     # interactive picker
jirac issue transition PROJ-123 --to "In Progress"

jirac issue attach PROJ-123 ./screenshot.png
jirac issue delete PROJ-123
```

### Worklogs

```bash
jirac issue worklog list PROJ-123
jirac issue worklog add PROJ-123 --time 2h --comment "Fixed auth bug"
jirac issue worklog delete PROJ-123 --id 10234
```

### Bulk operations

```bash
jirac issue bulk-transition -p PROJ -q 'status = "To Do"' -t "In Progress"
jirac issue bulk-update -p PROJ -q 'status = Done' --field assignee --value me@co.com
jirac issue archive -p PROJ -q 'status = Done AND updated < -90d'
```

### JQL builder

```bash
jirac issue jql    # interactive query builder
```

### Raw API passthrough

```bash
jirac api get /rest/api/3/serverInfo
jirac api post /rest/api/3/issue --body '{"fields":{...}}'
```

### Plans (Jira Premium)

```bash
jirac plan list
```

### Auth management

```bash
jirac auth login
jirac auth status
jirac auth update --token NEW_TOKEN
jirac auth logout
```

## Interactive TUI

The TUI is a full-screen terminal interface for browsing and managing issues. Press `?` inside the TUI for a complete shortcut reference.

```bash
jirac tui -p PROJ
```

Full keybinding reference: [TUI.md](TUI.md)

## Configuration

Config file: `~/.config/jira/config.toml`

```toml
base_url = "https://yourcompany.atlassian.net"
email = "you@example.com"
token = "your_api_token"
project = "PROJ"           # optional default project
timeout_secs = 30
```

Environment variables override the config file:

```bash
export JIRA_URL=https://yourcompany.atlassian.net
export JIRA_EMAIL=you@example.com
export JIRA_TOKEN=your_api_token
```

## MCP server

`jirac-mcp` exposes Jira as typed [Model Context Protocol](https://modelcontextprotocol.io) tools for editors, agents, and desktop apps. See the [jirac-mcp README](crates/jira-mcp/README.md) for setup and available tools.

## Using jira-core as a library

The `jira-core` crate can be used independently:

```toml
[dependencies]
jira-core = "0.12"
```

```rust
use jira_core::{JiraClient, config::JiraConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = JiraConfig::load()?;
    let client = JiraClient::new(config);

    let results = client.search_issues("project = PROJ", None, Some(10)).await?;
    for issue in results.issues {
        println!("{}: {}", issue.key, issue.summary);
    }
    Ok(())
}
```

See [jira-core on crates.io](https://crates.io/crates/jira-core) for full API documentation.

## Building from source

```bash
git clone https://github.com/mulhamna/jira-commands
cd jira-commands
cargo build --all
cargo test --all
```

### Workspace layout

```
crates/
├── jira-core/     # Rust library — API client, auth, models, ADF parser
├── jira/          # CLI binary (jirac) — clap commands + ratatui TUI
└── jira-mcp/      # MCP server binary (jirac-mcp) — rmcp-based
```

Releases are automated via [release-please](https://github.com/googleapis/release-please). See [CHANGELOG.md](CHANGELOG.md) for version history.

## Upgrading from `jira` to `jirac`

The binary was renamed from `jira` to `jirac`. Update any scripts or aliases:

```bash
alias jira='jirac'
```

## More documentation

- [INSTALL.md](INSTALL.md) — all installation methods
- [TUI guide](TUI.md) — full keybinding reference
- [MCP server](crates/jira-mcp/README.md) — setup and tool list
- [Claude Code plugin](PLUGIN.md) — slash commands for Claude Code
- [ClawHub skill](https://clawhub.ai/mulhamna/jirac) — OpenClaw integration
- [CONTRIBUTING.md](CONTRIBUTING.md) — contributor guide

## License

Licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE).

---

<sub>**jirac** is an independent, community-built tool. It is not affiliated with, endorsed by, or sponsored by Atlassian. "Jira" is a trademark of Atlassian.</sub>
