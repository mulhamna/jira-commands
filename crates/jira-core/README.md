# jira-core

> **jira-core** is an independent Rust library for the Jira ecosystem.
> It is **not** affiliated with, endorsed by, or sponsored by Atlassian.

`jira-core` is the shared library crate behind the `jirac` CLI and `jirac-mcp`. It provides the Jira HTTP client, authentication/config loading, typed models, field metadata helpers, and ADF conversion utilities.

This crate is versioned and released from the `mulhamna/jira-commands` workspace.

[![Crates.io](https://img.shields.io/crates/v/jira-core.svg)](https://crates.io/crates/jira-core)
[![License: MIT%20OR%20Apache--2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE)

## Install

```toml
[dependencies]
jira-core = "0.12"
```

## What this crate provides

- `JiraClient` for Jira REST API v3 operations
- `JiraConfig` for config/env loading
- `FieldCache` for field metadata reuse
- issue, worklog, transition, attachment, and bulk-operation helpers
- Atlassian Document Format helpers for text and Markdown conversion

## Example

```rust
use jira_core::{JiraClient, JiraConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = JiraConfig::load()?;
    let client = JiraClient::new(config);

    let results = client
        .search_issues("project = MYPROJ AND status = 'In Progress'", None, Some(25))
        .await?;

    for issue in results.issues {
        println!("{}: {}", issue.key, issue.summary);
    }

    Ok(())
}
```

## Configuration

Credentials are loaded from:
- `~/.config/jira/config.toml`
- `JIRA_URL`
- `JIRA_EMAIL`
- `JIRA_TOKEN`

That makes it easy to share config with `jirac` and `jirac-mcp`.

## Related crates

- `jira-commands` for the CLI
- `jira-mcp` for MCP server integrations

## More docs

See the root README for workspace-level installation and feature overview.
