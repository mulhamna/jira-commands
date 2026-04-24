# jirac

A fast, polished Jira CLI and TUI built in Rust.

[![CI](https://github.com/mulhamna/jira-commands/actions/workflows/ci.yml/badge.svg)](https://github.com/mulhamna/jira-commands/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/jira-commands.svg)](https://crates.io/crates/jira-commands)
[![Homebrew](https://img.shields.io/badge/homebrew-mulhamna%2Ftap-orange)](https://github.com/mulhamna/homebrew-tap)
[![License: MIT%20OR%20Apache--2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

`jirac` is an opinionated Jira terminal client for people who want terminal speed without giving up modern Jira workflows. It supports **custom fields discovered at runtime**, **native attachment uploads**, **cursor-based pagination**, and broad **Jira REST API v3** coverage.

It ships as a single binary with no runtime dependencies, runs on macOS, Linux, and Windows, and includes:
- an interactive terminal UI for browsing and updating issues,
- an [MCP server](#mcp-server) for editor and agent integrations,
- a [Claude Code plugin](#claude-code-plugin), and
- a separate ClawHub skill lane under `clawhub/jirac/`.

The OpenClaw skill is also published on ClawHub: <https://clawhub.ai/mulhamna/jirac>

## Preview

![jirac TUI preview](assets/readme/sample_tui.jpeg)
![jirac TUI preview JQL builder](assets/readme/sample-jql.jpeg)

## Installation

Choose the installer that fits your environment. For detailed step-by-step instructions, see [INSTALL.md](INSTALL.md).

### Installation matrix

| Method | macOS | Linux | Windows | Notes |
| --- | --- | --- | --- | --- |
| Homebrew | Yes | Yes | No | `jirac` formula via `mulhamna/tap` |
| Install script | Yes | Yes | No | Downloads latest release asset |
| PowerShell installer | No | No | Yes | Installs `jirac.exe` to user-local bin |
| Cargo | Yes | Yes | Yes | Best for Rust users |
| GitHub Releases | Yes | Yes | Yes | Manual archive/binary download |
| Winget | No | No | Yes | Windows package manager |
| Chocolatey | No | No | Yes | Windows package manager |

### Quick install commands

#### Homebrew (macOS / Linux)

```bash
brew tap mulhamna/tap && brew install jira-commands
```

#### Install script (macOS / Linux)

```bash
curl -sSL https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.sh | bash
```

#### PowerShell installer (Windows)

```powershell
powershell -ExecutionPolicy Bypass -Command "& ([scriptblock]::Create((Invoke-WebRequest 'https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.ps1').Content))"
```

Installs `jirac.exe` to `%LOCALAPPDATA%\Programs\jirac\bin` and adds that directory to your user `PATH`.

#### Cargo

```bash
cargo install jira-commands
```

#### Winget (Windows)

```powershell
winget install mulhamna.jirac
```

#### Chocolatey (Windows)

```powershell
choco install jirac
```

Chocolatey packages are published automatically from official GitHub releases.

#### GitHub Releases

Download pre-built archives from [GitHub Releases](https://github.com/mulhamna/jira-commands/releases).

Prefer the packaged archives over raw binaries. They include the executable, licenses, and README.

Supported release artifacts:

| Platform              | Raw binary                 | Preferred archive            |
| --------------------- | -------------------------- | ---------------------------- |
| macOS (Apple Silicon) | `jirac-macos-aarch64`      | `jirac-macos-aarch64.tar.gz` |
| macOS (Intel)         | `jirac-macos-x86_64`       | `jirac-macos-x86_64.tar.gz`  |
| Linux (x86_64)        | `jirac-linux-x86_64`       | `jirac-linux-x86_64.tar.gz`  |
| Linux (ARM64)         | `jirac-linux-aarch64`      | `jirac-linux-aarch64.tar.gz` |
| Windows (x86_64)      | `jirac-windows-x86_64.exe` | `jirac-windows-x86_64.zip`   |

Releases publish `jirac` and `jirac-mcp`. The legacy `jira` binary is no longer shipped in release artifacts.

## Why jirac

| Feature                           |      **jirac**       | [jira-cli](https://github.com/ankitpokhrel/jira-cli) | [jira-cmd](https://github.com/palashkulsh/jira-cmd) |
| --------------------------------- | :------------------: | :--------------------------------------------------: | :-------------------------------------------------: |
| Language / runtime                | Rust (single binary) |                  Go (single binary)                  |                    Node.js (npm)                    |
| Interactive TUI                   |         Yes          |                         Yes                          |                         No                          |
| Jira REST API version             |          v3          |                       v2 / v3                        |                         v2                          |
| Custom fields (runtime discovery) |         Yes          |                Partial (config-based)                |                 Partial (field IDs)                 |
| Attachment upload                 |         Yes          |                          No                          |                         No                          |
| Worklogs (add / list / delete)    |         Yes          |                          No                          |                   Add / list only                   |
| Bulk transition                   |         Yes          |                          No                          |                         No                          |
| Bulk update                       |         Yes          |                          No                          |                         No                          |
| Issue archive                     |         Yes          |                          No                          |                         No                          |
| JQL builder (interactive)         |         Yes          |                          No                          |                         No                          |
| Raw API passthrough               |         Yes          |                          No                          |                         No                          |
| Cursor-based pagination           |         Yes          |                     No (offset)                      |                     No (offset)                     |
| MCP server                        |  Yes (`jirac-mcp`)   |                          No                          |                         No                          |
| Claude Code plugin                |   Yes (12 skills)    |                          No                          |                         No                          |
| Homebrew                          |         Yes          |                         Yes                          |                         No                          |
| Winget                            |         Yes          |                          No                          |                         No                          |
| Chocolatey                        |         Yes          |                          No                          |                         No                          |
| macOS / Linux / Windows           |   Yes / Yes / Yes    |                 Yes / Yes / Partial                  |                   Yes / Yes / Yes                   |
| Jira Server (on-prem)             |      Cloud only      |                    Cloud + Server                    |                   Cloud + Server                    |

## Quick start

### 1. Create an API token

Go to [Atlassian API tokens](https://id.atlassian.com/manage-profile/security/api-tokens) and create a new token.

### 2. Authenticate

```bash
jirac auth login
```

You will be prompted for your Jira base URL, email, and API token. Credentials are stored in `~/.config/jira/config.toml` with `600` permissions.

### 3. Start using it

```bash
jirac issue list                          # your assigned issues
jirac issue list -p PROJ                  # issues in a project
jirac issue view PROJ-123                 # view issue detail
jirac issue create -p PROJ                # create (interactive)
jirac issue transition PROJ-123 --to Done # transition
jirac tui -p PROJ                         # interactive TUI
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
jirac issue jql                                     # interactive
```

![jirac JQL builder](assets/readme/sample-jql.jpeg)

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
jirac auth update --url https://new.atlassian.net
jirac auth logout
```

## Interactive TUI

```bash
jirac tui
jirac tui -p PROJ
```

Common shortcuts:

| Key         | Action                                                              |
| ----------- | ------------------------------------------------------------------- |
| `j` / `k`   | Navigate up / down                                                  |
| `Enter`     | View issue                                                          |
| `C`         | Open column settings popup                                          |
| `c`         | Create issue                                                        |
| `e`         | Edit issue                                                          |
| `a`         | Open native assignee popup and assign issue                         |
| `t`         | Transition issue                                                    |
| `w`         | Add worklog                                                         |
| `l`         | Manage labels                                                       |
| `m`         | Open native component popup and set project-scoped components       |
| `u`         | Upload attachment                                                   |
| `o`         | Open in browser                                                     |
| `r`         | Refresh issue list                                                  |
| `/`         | JQL search                                                          |
| `?`         | Show in-app help                                                    |
| `q` / `Esc` | Quit or go back, depending on context                               |

Inside column settings:

| Key           | Action                       |
| ------------- | ---------------------------- |
| `Space`       | Toggle selected column       |
| `a`           | Select all available columns |
| `r`           | Reset to default columns     |
| `s` / `Enter` | Save preferences             |
| `Esc`         | Cancel without saving        |

Inside assignee popup:

| Key         | Action |
| ----------- | ------ |
| Type        | Filter assignee list from Jira |
| `j` / `k`   | Move selection |
| `Enter`     | Assign selected assignee |
| `Esc`       | Cancel |

Inside component popup:

| Key         | Action |
| ----------- | ------ |
| Type        | Filter project components |
| `j` / `k`   | Move selection |
| `Space`     | Toggle selected component |
| `Enter`     | Save selected components |
| `Esc`       | Cancel |

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

`jirac-mcp` exposes Jira as typed [Model Context Protocol](https://modelcontextprotocol.io) tools for editors, agents, and desktop apps.

### Install

```bash
cargo install jira-mcp
```

Or use the release installer flow for your platform:

```bash
# macOS / Linux
curl -sSL https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.sh | BINARY=jirac-mcp bash
```

```powershell
# Windows
powershell -ExecutionPolicy Bypass -Command "& ([scriptblock]::Create((Invoke-WebRequest 'https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.ps1').Content))" -Binary jirac-mcp
```

### Run

```bash
# stdio (local MCP clients)
jirac-mcp serve --transport stdio

# Streamable HTTP (remote clients)
jirac-mcp serve --transport streamable-http --host 127.0.0.1 --port 8787 --path /mcp
```

### Client configuration

```json
{
  "mcpServers": {
    "jira": {
      "command": "jirac-mcp",
      "args": ["serve", "--transport", "stdio"]
    }
  }
}
```

### Available tools

| Category | Tools                                                                                                        |
| -------- | ------------------------------------------------------------------------------------------------------------ |
| Auth     | `jira_auth_status`, `jira_auth_set_credentials`, `jira_auth_logout`                                          |
| Issues   | `jira_issue_list`, `jira_issue_view`, `jira_issue_create`, `jira_issue_update`, `jira_issue_delete`          |
| Metadata | `jira_issue_types_list`, `jira_issue_fields`, `jira_issue_transitions_list`                                  |
| Workflow | `jira_issue_transition`, `jira_issue_attach`, `jira_worklog_list`, `jira_worklog_add`, `jira_worklog_delete` |
| Bulk     | `jira_issue_bulk_transition`, `jira_issue_bulk_update`, `jira_issue_archive`                                 |
| Advanced | `jira_plan_list`, `jira_api_request`                                                                         |

Destructive tools (delete, archive, bulk operations) require `confirm: true`. The `jira_api_request` tool provides raw access to any Jira REST endpoint.

## ClawHub skill

`jirac` is also available as an OpenClaw skill on ClawHub:

- <https://clawhub.ai/mulhamna/jirac>

The ClawHub lane is intentionally separate from the Claude Code plugin lane. It documents the OpenClaw-facing skill surface and points users to supported `jirac` installation options before use.

## Claude Code plugin

The plugin namespace remains `/jira:*`, but the binary it invokes is `jirac`.

The Claude Code plugin now has its own release lane under `plugin/`, with dedicated `plugin/VERSION` and `plugin/CHANGELOG.md` files. ClawHub publishing uses the dedicated skill lane under `clawhub/jirac/` rather than sharing the Claude plugin packaging.

```bash
# 1. Install the CLI that the plugin calls
cargo install jira-commands
jirac auth login
```

```text
# 2. In Claude Code
/plugin marketplace add mulhamna/jira-commands
/plugin install jira@jira-commands
```

| Skill                   | Description                             |
| ----------------------- | --------------------------------------- |
| `/jira:list-issues`     | List issues by project or JQL           |
| `/jira:view-issue`      | View full issue detail                  |
| `/jira:create-issue`    | Create a new issue                      |
| `/jira:update-issue`    | Update an existing issue                |
| `/jira:transition`      | Transition an issue                     |
| `/jira:comment`         | List comments or add a Markdown comment |
| `/jira:worklog`         | List, add, or delete worklogs           |
| `/jira:fields`          | Inspect available field metadata        |
| `/jira:bulk-transition` | Bulk transition issues via JQL          |
| `/jira:attach`          | Upload a file to an issue               |
| `/jira:jql`             | Build and run a JQL query               |
| `/jira:api`             | Raw REST API passthrough                |

## Using jira-core as a library

The `jira-core` crate can be used independently as a Rust library:

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
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo build --all
```

### Workspace layout

```
crates/
├── jira-core/     # Rust library — API client, auth, models, ADF parser
├── jira/          # CLI binary (jirac) — clap commands + ratatui TUI
└── jira-mcp/      # MCP server binary (jirac-mcp) — rmcp-based
plugin/
└── .claude-plugin/  # Claude Code plugin (12 skills)
```

Releases are automated via [release-please](https://github.com/googleapis/release-please). See [CHANGELOG.md](CHANGELOG.md) for version history.

## Upgrading from `jira` to `jirac`

The supported CLI binary is `jirac`.

If you still have old scripts, aliases, or wrappers that call `jira`, update them before upgrading. Release artifacts now ship `jirac` and `jirac-mcp` only.

```bash
# Example shell alias update
alias jira='jirac'
```

## License

Licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE).

## Windows packaging

- `install.ps1` is the direct Windows installer entrypoint
- Winget manifest sources are maintained in `packaging/winget/`
- Chocolatey package is published from official GitHub releases
- GitHub Releases remain the source for release archives and checksums

---

<sub>**jirac** is an independent, community-built tool. It is not affiliated with, endorsed by, or sponsored by Atlassian. "Jira" is a trademark of Atlassian.</sub>

