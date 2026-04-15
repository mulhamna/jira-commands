# jira-commands

A fast, cross-platform Jira terminal client built in Rust. Replaces the limitations of existing Jira CLIs with full custom field support, native attachment upload, and compatibility with the latest Jira REST API v3.

[![CI](https://github.com/mulhamna/jira-commands/actions/workflows/ci.yml/badge.svg)](https://github.com/mulhamna/jira-commands/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/jira-commands.svg)](https://crates.io/crates/jira-commands)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

---

## Features

- **Issue CRUD** — list, view, create, update, delete, transition
- **JQL search** — full JQL support with cursor-based pagination
- **Interactive TUI** — keyboard-driven issue browser via ratatui
- **Cross-platform** — macOS, Linux, Windows (single binary, no runtime deps)
- **Jira REST API v3** — uses the latest endpoints (`/search/jql`, cursor pagination)
- **Dynamic field resolution** — no hardcoded `customfield_xxxxx`

---

## Installation

### Via cargo

```bash
cargo install jira-commands
```

### Download binary

Download the pre-built binary for your platform from [GitHub Releases](https://github.com/mulhamna/jira-commands/releases).

| Platform | Binary |
|---|---|
| macOS (Apple Silicon) | `jira-macos-aarch64` |
| macOS (Intel) | `jira-macos-x86_64` |
| Linux (x86_64) | `jira-linux-x86_64` |
| Linux (ARM64) | `jira-linux-aarch64` |
| Windows | `jira-windows-x86_64.exe` |

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

# Create an issue
jira issue create --project MYPROJ --summary "Fix login bug" --issue-type Bug --priority High

# Update an issue
jira issue update MYPROJ-123 --summary "Updated title"
jira issue update MYPROJ-123 --assignee teammate@example.com

# Transition an issue (interactive picker)
jira issue transition MYPROJ-123

# Delete an issue
jira issue delete MYPROJ-123
```

### Auth commands

```bash
jira auth login            # Full login setup
jira auth status           # Show current credentials
jira auth update --token NEW_TOKEN     # Update token only
jira auth update --url https://new.atlassian.net  # Update URL only
jira auth update --email new@email.com  # Update email only
jira auth logout           # Remove credentials
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
| `t` | Transition issue (interactive picker) |
| `o` | Open issue in browser |
| `r` | Refresh list |
| `/` | Search — type JQL, press Enter |
| `?` | Help popup |
| `q` / `Esc` | Quit / go back |

### Environment variables

You can set credentials via env vars instead of config file:

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
jira-core = "0.1"
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

# Run smoke tests
cargo fmt --all -- --check && \
cargo clippy --all-targets --all-features -- -D warnings && \
cargo test --all && \
cargo build --all
```

### Workspace structure

```
jira-commands/
├── Cargo.toml                  # workspace root
├── crates/
│   ├── jira-core/              # public library (published to crates.io)
│   │   └── src/
│   │       ├── client.rs       # JiraClient — all HTTP calls
│   │       ├── config.rs       # figment config (file + env vars)
│   │       ├── auth.rs         # credential helpers
│   │       ├── adf.rs          # Atlassian Document Format parser
│   │       ├── error.rs        # thiserror error types
│   │       └── model/          # Issue, Field, Sprint types
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

```bash
# 1. Bump versions in Cargo.toml files
# 2. Commit
git add . && git commit -m "chore: bump version to v0.x.x"

# 3. Tag — this triggers the release workflow
git tag v0.x.x
git push origin main
git push origin v0.x.x
```

The release workflow will:
1. Build binaries for all 5 targets (Linux x86_64/ARM64, macOS x86_64/ARM64, Windows x86_64)
2. Publish `jira-core` to crates.io
3. Publish `jira-commands` (binary) to crates.io
4. Create a GitHub Release with binaries and SHA256 checksums

---

## Roadmap

| Phase | Focus | Status |
|---|---|---|
| 1 — Foundation | Auth, config, HTTP client, issue CRUD, TUI | ✅ Done |
| 2 — Custom fields & Attachments | Dynamic field introspection, file upload | ✅ Done |
| 3 — Bulk ops & Advanced TUI | Bulk edit/transition, worklog, JQL builder | ✅ Done |
| 4 — Power features | Plans API, archive, raw API passthrough | ✅ Done |

---

## License

MIT
