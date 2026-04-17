# jira-commands

A fast, cross-platform Jira terminal client built in Rust — and a Claude Code plugin to manage Jira without leaving your editor.

Replaces the limitations of existing Jira CLIs with full custom field support, native attachment upload, cursor-based pagination, and compatibility with the latest Jira REST API v3.

[![CI](https://github.com/mulhamna/jira-commands/actions/workflows/ci.yml/badge.svg)](https://github.com/mulhamna/jira-commands/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/jira-commands.svg)](https://crates.io/crates/jira-commands)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Changelog](https://img.shields.io/badge/changelog-CHANGELOG.md-blue)](CHANGELOG.md)

---

## Installation

| Method | Command |
|---|---|
| **Homebrew** (macOS/Linux) | `brew tap mulhamna/tap && brew install jira-commands` |
| **cargo** | `cargo install jira-commands` |
| **Claude Code plugin** | Add marketplace → install (see below) |
| **Binary** | Download from [GitHub Releases](https://github.com/mulhamna/jira-commands/releases) |

### Binary downloads

| Platform | File |
|---|---|
| macOS (Apple Silicon) | `jira-macos-aarch64` |
| macOS (Intel) | `jira-macos-x86_64` |
| Linux (x86_64) | `jira-linux-x86_64` |
| Linux (ARM64) | `jira-linux-aarch64` |
| Windows | `jira-windows-x86_64.exe` |

### Claude Code plugin

```bash
# 1. Install the CLI first (the plugin calls this binary)
cargo install jira-commands
jira auth login
```

```
# 2. Inside Claude Code — add marketplace and install
/plugin marketplace add mulhamna/jira-commands
/plugin install jira@jira-commands
```

Then use Jira directly from Claude Code:

| Skill | Description |
|---|---|
| `/jira:list-issues` | List issues by project or JQL |
| `/jira:view-issue` | View full issue detail |
| `/jira:create-issue` | Create a new issue (interactive) |
| `/jira:transition` | Transition an issue to a new status |
| `/jira:worklog` | List, add, or delete worklogs |
| `/jira:bulk-transition` | Transition multiple issues via JQL |
| `/jira:attach` | Upload a file to an issue |
| `/jira:jql` | Build and run a JQL query |
| `/jira:api` | Raw REST API passthrough |

---

## Use cases

**Daily standup prep**
```bash
jira issue list                          # or in Claude Code: /jira:list-issues
```
See all your in-progress issues in one command.

**Create a bug from a stack trace**
```bash
jira issue create -p PROJ --type Bug     # or: /jira:create-issue
```
Interactive prompts handle summary, description, priority, and all custom fields dynamically.

**Transition after a PR merge**
```bash
jira issue transition PROJ-123 --to Done  # or: /jira:transition
```

**Log time at end of day**
```bash
jira issue worklog add PROJ-123 --time 2h --comment "Implemented auth flow"
```

**Bulk close resolved issues**
```bash
jira issue bulk-transition -p PROJ -q 'status = Done AND updated < -30d' -t Closed
```

**Explore any Jira endpoint**
```bash
jira api get /rest/api/3/project         # raw JSON, any endpoint
```

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
- **Cross-platform** — macOS, Linux, Windows (single binary, no runtime deps)
- **Jira REST API v3** — latest endpoints (`/search/jql`, cursor pagination)

---

## Getting started

### 1. Get an API token

Go to: https://id.atlassian.com/manage-profile/security/api-tokens → **Create API token**

### 2. Login

```bash
jira auth login
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
jira auth status
```

---

## Usage

### Issue commands

```bash
# List issues assigned to you (default)
jira issue list

# List issues by project
jira issue list --project MYPROJ

# List issues with custom JQL
jira issue list --jql "project = MYPROJ AND status = 'In Progress'"

# View issue detail
jira issue view MYPROJ-123

# Create an issue (interactive)
jira issue create --project MYPROJ

# Update an issue
jira issue update MYPROJ-123 --summary "Updated title"
jira issue update MYPROJ-123 --assignee teammate@example.com

# Transition an issue (interactive picker)
jira issue transition MYPROJ-123

# Transition with target status
jira issue transition MYPROJ-123 --to "In Progress"

# Upload attachment
jira issue attach MYPROJ-123 ./screenshot.png

# Delete an issue
jira issue delete MYPROJ-123
```

### Worklog

```bash
jira issue worklog list MYPROJ-123
jira issue worklog add MYPROJ-123 --time 2h --comment "Fixed auth bug"
jira issue worklog delete MYPROJ-123 --id 10234
```

### Bulk operations

```bash
# Bulk transition all matching issues
jira issue bulk-transition -p MYPROJ -q 'status = "To Do"' -t "In Progress"

# Bulk update a field
jira issue bulk-update -p MYPROJ -q 'status = Done' --field assignee --value me@example.com

# Archive issues
jira issue archive -p MYPROJ -q 'status = Done AND updated < -90d'
```

### Interactive JQL builder

```bash
jira issue jql
```

### Auth commands

```bash
jira auth login
jira auth status
jira auth update --token NEW_TOKEN
jira auth update --url https://new.atlassian.net
jira auth update --email new@email.com
jira auth logout
```

### Raw API passthrough

```bash
jira api get /rest/api/3/serverInfo
jira api get /rest/api/3/issue/MYPROJ-123
jira api post /rest/api/3/issue --body '{"fields":{...}}'
```

### Plans (Jira Premium)

```bash
jira plan list
```

### TUI

```bash
jira tui                   # Launch for assigned issues (currentUser)
jira tui --project MYPROJ  # Launch for a specific project
```

**TUI keyboard shortcuts:**

| Key | Action |
|---|---|
| `↑` / `k` | Move up |
| `↓` / `j` | Move down |
| `Enter` | View issue detail |
| `c` | Create new issue |
| `e` | Edit selected issue (summary, description, assignee, priority) |
| `a` | Assign selected issue |
| `w` | Add worklog to selected issue |
| `l` | Add / remove labels |
| `m` | Add / remove components |
| `u` | Upload attachment |
| `t` | Transition issue (interactive picker) |
| `o` | Open issue in browser |
| `r` | Refresh list |
| `/` | Search — type JQL, press Enter |
| `?` | Help popup |
| `q` / `Esc` | Quit / go back |

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
jira-core = "0.4"
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
git commit -m "feat: add new command"
git push origin main

# 2. release-please automatically creates/updates a Release PR
# 3. Merge the Release PR → tag is pushed → release workflow runs automatically
```

The release workflow will:
1. Build binaries for all 5 targets (Linux x86_64/ARM64, macOS x86_64/ARM64, Windows x86_64)
2. Publish `jira-core` to crates.io
3. Publish `jira-commands` to crates.io
4. Create a GitHub Release with binaries and SHA256 checksums
5. Update the Homebrew formula in [mulhamna/homebrew-tap](https://github.com/mulhamna/homebrew-tap)

---

## Roadmap

| Phase | Focus | Status |
|---|---|---|
| 1 — Foundation | Auth, config, HTTP client, issue CRUD, TUI | ✅ Done |
| 2 — Custom fields & Attachments | Dynamic field introspection, file upload | ✅ Done |
| 3 — Bulk ops & Advanced TUI | Bulk edit/transition, worklog, JQL builder | ✅ Done |
| 4 — Power features | Plans API, archive, raw API passthrough | ✅ Done |
| 5 — UX & Automation | bulk-create, clone, batch, `--json` mode, TUI edit actions, improved `--help` | ✅ Done |
| 6 — Distribution | Homebrew tap (macOS/Linux), automated formula updates via CI | ✅ Done |

---

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for a full history of changes per version.

---

## License

MIT
