# jira-core

Core library for the [jira-commands](https://crates.io/crates/jira-commands) Jira CLI — provides the HTTP client, auth, models, and ADF parser as a reusable Rust library.

[![Crates.io](https://img.shields.io/crates/v/jira-core.svg)](https://crates.io/crates/jira-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

## Usage

```toml
[dependencies]
jira-core = "0.5"
```

```rust
use jira_core::{JiraClient, JiraConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = JiraConfig::load()?;
    let client = JiraClient::new(config);

    // Search issues
    let results = client
        .search_issues("project = MYPROJ AND status = 'In Progress'", None, Some(50))
        .await?;

    for issue in results.issues {
        println!("{}: {}", issue.key, issue.summary);
    }

    // Get a single issue
    let issue = client.get_issue("MYPROJ-123").await?;
    println!("{}", issue.description.unwrap_or_default());

    // Transition an issue
    let transitions = client.get_transitions("MYPROJ-123").await?;
    client.transition_issue("MYPROJ-123", &transitions[0]["id"].as_str().unwrap()).await?;

    // Upload attachment
    client.upload_attachment("MYPROJ-123", "./screenshot.png").await?;

    // Add worklog
    client.add_worklog("MYPROJ-123", "2h", Some("Fixed auth bug")).await?;

    Ok(())
}
```

## Features

- `JiraClient` — full Jira REST API v3 client
  - Issue CRUD (create, read, update, delete, transition)
  - JQL search with cursor-based pagination (`/search/jql`)
  - Attachment upload (multipart/form-data)
  - Worklog management
  - Bulk operations and archive
  - Raw API passthrough
  - Plans API (Jira Premium)
- `JiraConfig` — config loading from file (`~/.config/jira/config.toml`) and env vars
- `FieldCache` — in-memory field metadata cache with TTL
- ADF parser — Atlassian Document Format ↔ Markdown conversion
- `thiserror`-based error types

## Auth

Credentials are loaded automatically from `~/.config/jira/config.toml` (managed by `jira auth login`) or from environment variables:

```bash
JIRA_URL=https://yourcompany.atlassian.net
JIRA_EMAIL=you@example.com
JIRA_TOKEN=your_api_token
```

## Full documentation

See [github.com/mulhamna/jira-commands](https://github.com/mulhamna/jira-commands).
