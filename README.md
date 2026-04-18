# jira-commands

> **jirac** is an independent CLI tool that helps you work with the Jira ecosystem from your terminal.
> It is **not** a replacement for Jira, and is **not** affiliated with, endorsed by, or sponsored by Atlassian.
> All product names and trademarks are the property of their respective owners.

A fast, cross-platform Jira terminal client built in Rust — plus a Claude Code plugin and an open-source MCP server to manage Jira from editor and agent workflows.

Built to fill the gaps left by existing Jira CLIs: full custom field support, native attachment upload, cursor-based pagination, and compatibility with the latest Jira REST API v3.

[![CI](https://github.com/mulhamna/jira-commands/actions/workflows/ci.yml/badge.svg)](https://github.com/mulhamna/jira-commands/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/jira-commands.svg)](https://crates.io/crates/jira-commands)
[![Homebrew](https://img.shields.io/badge/homebrew-mulhamna%2Ftap-orange)](https://github.com/mulhamna/homebrew-tap)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Changelog](https://img.shields.io/badge/changelog-CHANGELOG.md-blue)](CHANGELOG.md)

---

## Installation

| Method                     | Command                                                                             |
| -------------------------- | ----------------------------------------------------------------------------------- |
| **curl** (macOS/Linux)     | `curl -sSL https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.sh \| bash` |
| **Homebrew** (macOS/Linux) | `brew tap mulhamna/tap && brew install jira-commands`                               |
| **cargo**                  | `cargo install jira-commands`                                                       |
| **cargo (MCP server)**     | `cargo install jira-mcp`                                                            |
| **Claude Code plugin**     | Add marketplace → install (see below)                                               |
| **Binary**                 | Download from [GitHub Releases](https://github.com/mulhamna/jira-commands/releases) |

### Binary downloads

| Platform              | File                        |
| --------------------- | --------------------------- |
| macOS (Apple Silicon) | `jirac-macos-aarch64`       |
| macOS (Intel)         | `jirac-macos-x86_64`        |
| Linux (x86_64)        | `jirac-linux-x86_64`        |
| Linux (ARM64)         | `jirac-linux-aarch64`       |
| Windows               | `jirac-windows-x86_64.exe`  |

> Legacy `jira-*` binaries are also included in each release for backward compatibility.

### MCP server

Use `jirac-mcp` when you want Jira available as MCP tools inside an editor, agent, or desktop app that supports the Model Context Protocol.

#### 1. Install the server

```bash
# Install from crates.io
cargo install jira-mcp

# Or install the release binary directly
curl -sSL https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.sh | BINARY=jirac-mcp bash
```

#### 2. Configure Jira credentials

`jirac-mcp` reads the same credentials as `jirac`.

Option A — use the CLI once:

```bash
jirac auth login
```

Option B — use environment variables:

```bash
export JIRA_URL=https://yourcompany.atlassian.net
export JIRA_EMAIL=you@example.com
export JIRA_TOKEN=your_api_token
```

#### 3. Start the MCP server

```bash
# Local MCP clients
jirac-mcp serve --transport stdio

# Remote MCP clients
jirac-mcp serve --transport streamable-http --host 127.0.0.1 --port 8787 --path /mcp
```

#### 4. Register it in your MCP client

Most local MCP clients use `stdio`. A typical config looks like this:

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

For HTTP-based MCP clients, point them to:

```text
http://127.0.0.1:8787/mcp
```

#### 5. Use the Jira tools

Typed MCP tools include:

- `jira_auth_status`, `jira_auth_set_credentials`, `jira_auth_logout`
- `jira_issue_list`, `jira_issue_view`, `jira_issue_types_list`, `jira_issue_fields`, `jira_issue_transitions_list`
- `jira_issue_create`, `jira_issue_update`, `jira_issue_delete`, `jira_issue_transition`, `jira_issue_attach`
- `jira_worklog_list`, `jira_worklog_add`, `jira_worklog_delete`
- `jira_issue_bulk_transition`, `jira_issue_bulk_update`, `jira_issue_archive`
- `jira_plan_list`, `jira_api_request`

Example prompts for an MCP client:

- "List my assigned Jira issues"
- "Show issue `PROJ-123`"
- "Create a bug in `PROJ` titled `Login fails on Safari`"
- "Transition `PROJ-123` to Done"
- "Log `2h` on `PROJ-123` with comment `Implemented auth flow`"

Notes:

- Destructive tools like delete, archive, and bulk updates require `confirm=true`.
- `jira_issue_attach` supports local file paths and inline base64 uploads.
- `jira_api_request` is the escape hatch for Jira REST endpoints not yet covered by typed tools.

### Claude Code plugin

```bash
# 1. Install the CLI first (the plugin calls this binary)
cargo install jira-commands
jirac auth login
```

```
# 2. Inside Claude Code — add marketplace and install
/plugin marketplace add mulhamna/jira-commands
/plugin install jira@jira-commands
```

Then use Jira directly from Claude Code:

| Skill                   | Description                         |
| ----------------------- | ----------------------------------- |
| `/jira:list-issues`     | List issues by project or JQL       |
| `/jira:view-issue`      | View full issue detail              |
| `/jira:create-issue`    | Create a new issue (interactive)    |
| `/jira:transition`      | Transition an issue to a new status |
| `/jira:worklog`         | List, add, or delete worklogs       |
| `/jira:bulk-transition` | Transition multiple issues via JQL  |
| `/jira:attach`          | Upload a file to an issue           |
| `/jira:jql`             | Build and run a JQL query           |
| `/jira:api`             | Raw REST API passthrough            |

---

## Upgrading from v0.x

The binary has been renamed from `jira` to `jirac` starting in v0.7.0.

The old `jira` binary is still included in every release and continues to work — it will just print a deprecation warning. It will be removed in a future major release.

**Action needed:**

```bash
# Update your aliases
alias jira='jirac'   # optional transitional alias

# Or just start using the new name directly
jirac issue list
```

If you installed via Homebrew, `brew upgrade jira-commands` handles everything automatically (both `jirac` and a `jira` symlink are installed).

---

## Use cases

**Daily standup prep**
```bash
jirac issue list                         # or in Claude Code: /jira:list-issues
```
See all your in-progress issues in one command.

**Create a bug from a stack trace**
```bash
jirac issue create -p PROJ --type Bug    # or: /jira:create-issue
```
Interactive prompts handle summary, description, priority, and all custom fields dynamically.

**Transition after a PR merge**
```bash
jirac issue transition PROJ-123 --to Done  # or: /jira:transition
```

**Log time at end of day**
```bash
jirac issue worklog add PROJ-123 --time 2h --comment "Implemented auth flow"
```

**Bulk close resolved issues**
```bash
jirac issue bulk-transition -p PROJ -q 'status = Done AND updated < -30d' -t Closed
```

**Explore any Jira endpoint**
```bash
jirac api get /rest/api/3/project        # raw JSON, any endpoint
```

**Use Jira from any MCP client**
```bash
jirac-mcp serve --transport stdio
```
Expose typed Jira tools to MCP-compatible editors and agents while reusing the same Jira config as `jirac`.

---

## Features

- **Issue CRUD** — list, view, create, update, delete, transition
- **JQL search** — full JQL support with cursor-based pagination
- **Custom fields** — dynamic field introspection, no hardcoded `customfield_xxxxx`
- **Attachments** — native multipart upload from terminal
- **Worklog** — list, add, delete time entries
- **Bulk ops** — bulk transition, bulk update, archive via JQL
- **Interactive TUI** — keyboard-driven issue browser via ratatui
- **Raw API** — passthrough to any Jira REST endpoint
- **Plans API** — Jira Premium / Advanced Roadmaps support
- **Claude Code plugin** — 9 skills to manage Jira from within Claude Code
- **MCP server** — typed tools for MCP clients over stdio or Streamable HTTP
- **Cross-platform** — macOS, Linux, Windows (single binary, no runtime deps)
- **Jira REST API v3** — latest endpoints (`/search/jql`, cursor pagination)

---

## Getting started

### 1. Get an API token

Go to: https://id.atlassian.com/manage-profile/security/api-tokens → **Create API token**

### 2. Login

```bash
jirac auth login
```

```
Jira base URL: https://yourcompany.atlassian.net
Email: you@example.com
API token: ****************************
✓ Credentials saved to ~/.config/jira/config.toml
```

Credentials are stored in `~/.config/jira/config.toml` with `600` permissions (owner read/write only).

### 3. Verify

```bash
jirac auth status
```

---

## Usage

### Issue commands

```bash
# List issues assigned to you (default)
jirac issue list

# List issues by project
jirac issue list --project MYPROJ

# List issues with custom JQL
jirac issue list --jql "project = MYPROJ AND status = 'In Progress'"

# View issue detail
jirac issue view MYPROJ-123

# Create an issue (interactive)
jirac issue create --project MYPROJ

# Update an issue
jirac issue update MYPROJ-123 --summary "Updated title"
jirac issue update MYPROJ-123 --assignee teammate@example.com

# Transition an issue (interactive picker)
jirac issue transition MYPROJ-123

# Transition with target status
jirac issue transition MYPROJ-123 --to "In Progress"

# Upload attachment
jirac issue attach MYPROJ-123 ./screenshot.png

# Delete an issue
jirac issue delete MYPROJ-123
```

### Worklog

```bash
jirac issue worklog list MYPROJ-123
jirac issue worklog add MYPROJ-123 --time 2h --comment "Fixed auth bug"
jirac issue worklog delete MYPROJ-123 --id 10234
```

### Bulk operations

```bash
# Bulk transition all matching issues
jirac issue bulk-transition -p MYPROJ -q 'status = "To Do"' -t "In Progress"

# Bulk update a field
jirac issue bulk-update -p MYPROJ -q 'status = Done' --field assignee --value me@example.com

# Archive issues
jirac issue archive -p MYPROJ -q 'status = Done AND updated < -90d'
```

### Interactive JQL builder

```bash
jirac issue jql
```

### Auth commands

```bash
jirac auth login
jirac auth status
jirac auth update --token NEW_TOKEN
jirac auth update --url https://new.atlassian.net
jirac auth update --email new@email.com
jirac auth logout
```

### Raw API passthrough

```bash
jirac api get /rest/api/3/serverInfo
jirac api get /rest/api/3/issue/MYPROJ-123
jirac api post /rest/api/3/issue --body '{"fields":{...}}'
```

### Plans (Jira Premium)

```bash
jirac plan list
```

### TUI

```bash
jirac tui                   # Launch for assigned issues (currentUser)
jirac tui --project MYPROJ  # Launch for a specific project
```

**TUI keyboard shortcuts:**

| Key         | Action                                                         |
| ----------- | -------------------------------------------------------------- |
| `↑` / `k`   | Move up                                                        |
| `↓` / `j`   | Move down                                                      |
| `Enter`     | View issue detail                                              |
| `c`         | Create new issue                                               |
| `e`         | Edit selected issue (summary, description, assignee, priority) |
| `a`         | Assign selected issue                                          |
| `w`         | Add worklog to selected issue                                  |
| `l`         | Add / remove labels                                            |
| `m`         | Add / remove components                                        |
| `u`         | Upload attachment                                              |
| `t`         | Transition issue (interactive picker)                          |
| `o`         | Open issue in browser                                          |
| `r`         | Refresh list                                                   |
| `/`         | Search — type JQL, press Enter                                 |
| `?`         | Help popup                                                     |
| `q` / `Esc` | Quit / go back                                                 |

### Environment variables

```bash
export JIRA_URL=https://yourcompany.atlassian.net
export JIRA_EMAIL=you@example.com
export JIRA_TOKEN=your_api_token
```

---

## Configuration

Config file location: `~/.config/jira/config.toml`

```toml
base_url = "https://yourcompany.atlassian.net"
email = "you@example.com"
token = "your_api_token"
project = "MYPROJ"        # optional default project
timeout_secs = 30
```

---

## Using jira-core as a library

`jira-core` is published separately on crates.io and can be used as a standalone library:

```toml
[dependencies]
jira-core = "0.7"
```

```rust
use jira_core::{JiraClient, config::JiraConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = JiraConfig::load()?;
    let client = JiraClient::new(config);

    let results = client.search_issues("project = MYPROJ", None, Some(10)).await?;
    for issue in results.issues {
        println!("{}: {}", issue.key, issue.summary);
    }
    Ok(())
}
```

---

## Development

```bash
git clone https://github.com/mulhamna/jira-commands
cd jira-commands
cargo build --all

# Smoke test
cargo fmt --all -- --check && \
cargo clippy --all-targets --all-features -- -D warnings && \
cargo test --all && \
cargo build --all
```

### Workspace structure

```
jira-commands/
├── Cargo.toml                  # workspace root
├── install.sh                  # one-line curl installer
├── plugin/                     # Claude Code plugin
│   ├── .claude-plugin/
│   │   └── plugin.json
│   └── skills/                 # 9 skills (list, view, create, transition, ...)
├── crates/
│   ├── jira-core/              # public library (published to crates.io)
│   │   └── src/
│   │       ├── client.rs       # JiraClient — all HTTP calls
│   │       ├── config.rs       # figment config (file + env vars)
│   │       ├── auth.rs         # credential helpers
│   │       ├── adf.rs          # Atlassian Document Format parser
│   │       ├── error.rs        # thiserror error types
│   │       └── model/          # Issue, Field, Sprint, Worklog types
│   ├── jira-mcp/               # MCP server binary (published as jira-mcp)
│   │   └── src/
│   │       ├── app.rs          # Jira adapter and validation layer
│   │       ├── models.rs       # MCP tool input schemas
│   │       ├── server.rs       # MCP tool router + transports
│   │       └── main.rs
│   └── jira/                   # binary (published to crates.io as jira-commands)
│       └── src/
│           ├── main.rs
│           ├── cli/            # clap subcommands
│           └── tui/            # ratatui TUI
└── .github/workflows/
    ├── ci.yml                  # runs on every push/PR to main
    └── release.yml             # runs on git tag v*
```

### Release process

Releases are fully automated via [release-please](https://github.com/googleapis/release-please):

```bash
# 1. Push commits to main using Conventional Commits
git commit -m "feat: rename binary to jirac"
git push origin main

# 2. release-please automatically creates/updates a Release PR
# 3. Merge the Release PR → tag is pushed → release workflow runs automatically
```

The release workflow will:
1. Build binaries for all 5 targets (Linux x86_64/ARM64, macOS x86_64/ARM64, Windows x86_64)
2. Include `jirac-*`, `jirac-mcp-*`, and legacy `jira-*` binaries in each release
3. Publish `jira-core` to crates.io
4. Publish `jira-mcp` and `jira-commands` to crates.io
5. Create a GitHub Release with binaries and SHA256 checksums
6. Update the Homebrew formula in [mulhamna/homebrew-tap](https://github.com/mulhamna/homebrew-tap)

---

## Roadmap

| Phase                           | Focus                                                                         | Status |
| ------------------------------- | ----------------------------------------------------------------------------- | ------ |
| 1 — Foundation                  | Auth, config, HTTP client, issue CRUD, TUI                                    | ✅ Done |
| 2 — Custom fields & Attachments | Dynamic field introspection, file upload                                      | ✅ Done |
| 3 — Bulk ops & Advanced TUI     | Bulk edit/transition, worklog, JQL builder                                    | ✅ Done |
| 4 — Power features              | Plans API, archive, raw API passthrough                                       | ✅ Done |
| 5 — UX & Automation             | bulk-create, clone, batch, `--json` mode, TUI edit actions, improved `--help` | ✅ Done |
| 6 — Distribution                | Homebrew tap (macOS/Linux), automated formula updates via CI                  | ✅ Done |
| 7 — Rename & Install            | Binary rename `jira` → `jirac`, curl install script, ecosystem positioning    | ✅ Done |

---

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for a full history of changes per version.

---

## License

MIT
