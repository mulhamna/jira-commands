# jirac

A fast, polished Jira CLI and TUI built in Rust.

[![CI](https://github.com/mulhamna/jira-commands/actions/workflows/ci.yml/badge.svg)](https://github.com/mulhamna/jira-commands/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/jira-commands.svg)](https://crates.io/crates/jira-commands)
[![Homebrew](https://img.shields.io/badge/homebrew-mulhamna%2Ftap-orange)](https://github.com/mulhamna/homebrew-tap)
[![License: MIT%20OR%20Apache--2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

`jirac` is an opinionated Jira terminal client for people who want terminal speed without giving up modern Jira workflows. It supports custom fields discovered at runtime, native attachment uploads, cursor-based pagination, Jira Cloud, and Jira Data Center.

It ships as a single binary with no runtime dependencies, runs on macOS, Linux, and Windows, and includes:
- an interactive terminal UI with split master-detail, saved JQLs, themes, and native popups
- an MCP server for editor and agent integrations
- a Claude Code plugin

The OpenClaw skill is also published on ClawHub: <https://clawhub.ai/mulhamna/jirac>

## Preview

![jirac TUI preview](assets/readme/sample_tui.jpeg)
![jirac TUI preview JQL builder](assets/readme/sample-jql.jpeg)

## Installation

Choose the installer that fits your environment. For detailed step-by-step instructions, see [INSTALL.md](INSTALL.md).

### Installation matrix

| Method | macOS | Linux | Windows | Notes |
| --- | --- | --- | --- | --- |
| Homebrew | Yes | Yes | No | `jira-commands` and `jira-mcp` formulas via `mulhamna/tap` |
| Install script | Yes | Yes | No | Downloads latest release asset |
| PowerShell installer | No | No | Yes | Installs `jirac.exe` to user-local bin |
| Cargo | Yes | Yes | Yes | Best for Rust users |
| GitHub Releases | Yes | Yes | Yes | Manual archive/binary download |
| Winget | No | No | Yes | Windows package manager |
| Chocolatey | No | No | Yes | Windows package manager |

### Quick install commands

#### Homebrew (macOS / Linux)

```bash
brew tap mulhamna/tap
brew install jira-commands

# Optional MCP server
brew install jira-mcp
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

| Feature                           | **jirac** | [jira-cli](https://github.com/ankitpokhrel/jira-cli) | [jira-cmd](https://github.com/palashkulsh/jira-cmd) |
| --------------------------------- | :-------: | :--------------------------------------------------: | :-------------------------------------------------: |
| Language / runtime                | Rust (single binary) | Go (single binary) | Node.js (npm) |
| Interactive TUI                   | Yes | Yes | No |
| Jira REST API version             | v2 / v3 | v2 / v3 | v2 |
| Custom fields (runtime discovery) | Yes | Partial (config-based) | Partial (field IDs) |
| Attachment upload                 | Yes | No | No |
| Worklogs (add / list / delete)    | Yes | No | Add / list only |
| Bulk transition                   | Yes | No | No |
| Bulk update                       | Yes | No | No |
| Bulk create / batch manifests     | Yes | No | No |
| Issue archive                     | Yes | No | No |
| JQL builder (interactive)         | Yes | No | No |
| Raw API passthrough               | Yes | No | No |
| Cursor-based pagination           | Yes | No (offset) | No (offset) |
| MCP server                        | Yes (`jirac-mcp`) | No | No |
| Claude Code plugin                | Yes | No | No |
| Homebrew                          | Yes | Yes | No |
| Winget                            | Yes | No | No |
| Chocolatey                        | Yes | No | No |
| macOS / Linux / Windows           | Yes / Yes / Yes | Yes / Yes / Partial | Yes / Yes / Yes |
| Jira Data Center / self-managed   | Cloud + Data Center | Cloud + self-managed | Cloud + self-managed |

## Quick start

### 1. Authenticate

```bash
jirac auth login
```

You will be prompted for your Jira base URL, email or username, and token or password as needed. Credentials are stored in `~/.config/jira/config.toml` with `600` permissions and can be organized into multiple profiles.

### 2. Start using it

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
jirac issue clone PROJ-123
jirac issue bulk-create --manifest issues.json
jirac issue batch --manifest ops.json
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
jirac issue bulk-create --manifest issues.json
jirac issue batch --manifest ops.json
```

### JQL builder

```bash
jirac issue jql
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
jirac auth profiles
jirac auth use work-cloud
jirac auth status
jirac auth update --profile client-dc --token NEW_SECRET
jirac auth logout --profile client-dc
```

## Interactive TUI

```bash
jirac tui
jirac tui -p PROJ
```

Common shortcuts:

| Key         | Action |
| ----------- | ------ |
| `j` / `k`   | Navigate up / down |
| `Enter`     | Open split detail view |
| `p`         | Open saved JQL queries |
| `T`         | Open theme picker |
| `S`         | Show server summary |
| `g`         | Show config summary |
| `C`         | Open column settings popup |
| `c`         | Create issue |
| `e`         | Edit issue |
| `a`         | Open native assignee popup and assign issue |
| `t`         | Transition issue |
| `w`         | Add worklog |
| `l`         | Manage labels |
| `m`         | Open native component popup and set project-scoped components |
| `u`         | Upload attachment |
| `o`         | Open in browser |
| `r`         | Refresh issue list |
| `/`         | JQL search |
| `?`         | Show in-app help |
| `q` / `Esc` | Quit or go back, depending on context |

## Configuration

Config file: `~/.config/jira/config.toml`

```toml
current_profile = "work-cloud"

[profiles.work-cloud]
base_url = "https://yourcompany.atlassian.net"
email = "you@example.com"
token = "your_api_token"
project = "PROJ"
timeout_secs = 30
deployment = "cloud"
auth_type = "cloud_api_token"
api_version = 3
```

Environment variables override the active profile:

```bash
export JIRA_PROFILE=work-cloud
export JIRA_URL=https://yourcompany.atlassian.net
export JIRA_EMAIL=you@example.com
export JIRA_TOKEN=your_api_token
```

## MCP server

`jirac-mcp` exposes Jira as typed [Model Context Protocol](https://modelcontextprotocol.io) tools for editors, agents, and desktop apps.

### Install

```bash
brew tap mulhamna/tap
brew install jira-mcp

# or
cargo install jira-mcp
```

### Run

```bash
jirac-mcp serve --transport stdio
jirac-mcp serve --transport streamable-http --host 127.0.0.1 --port 8787 --path /mcp
```

### Available tools

- auth status and credential updates
- issue list, view, create, update, delete, and clone
- field and transition discovery
- attachment upload
- worklog operations
- bulk transition, bulk update, batch, and archive flows
- plans
- raw Jira REST API requests

Destructive tools require `confirm: true`.

## Claude Code plugin

The plugin namespace remains `/jira:*`, but the binary it invokes is `jirac`.

```bash
cargo install jira-commands
jirac auth login
```

```text
/plugin marketplace add mulhamna/jira-commands
/plugin install jira@jira-commands
```

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

## Source

[github.com/mulhamna/jira-commands](https://github.com/mulhamna/jira-commands)
