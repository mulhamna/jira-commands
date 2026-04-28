# jira-mcp

> **jirac-mcp** exposes Jira operations over the Model Context Protocol (MCP).
> It is **not** affiliated with, endorsed by, or sponsored by Atlassian.

`jira-mcp` is the MCP server crate in the `mulhamna/jira-commands` workspace. It reuses `jira-core` and exposes typed Jira tools for editors, assistants, and remote MCP clients.

[![License: MIT%20OR%20Apache--2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE)

## Install

```bash
# Homebrew (macOS / Linux)
brew tap mulhamna/tap
brew install jira-mcp

# Cargo
cargo install jira-mcp
```

You can also use the workspace shell installer on macOS/Linux, the PowerShell installer flow on Windows, or download packaged release archives from GitHub Releases.

## Run

```bash
# Local stdio transport
jirac-mcp serve --transport stdio

# Streamable HTTP transport
jirac-mcp serve --transport streamable-http --host 127.0.0.1 --port 8787 --path /mcp
```

## Shared configuration

The server reuses the same credentials/config as `jirac`:
- `~/.config/jira/config.toml`
- `JIRA_PROFILE`
- `JIRA_URL`
- `JIRA_EMAIL`
- `JIRA_TOKEN`

You can initialize and switch credentials with:

```bash
jirac auth login
jirac auth profiles
jirac auth use work-cloud
```

## Tool coverage

The MCP server includes tools for:
- auth status and credential updates
- issue list, view, create, update, delete, and clone
- field and transition discovery
- attachment upload
- worklog operations
- bulk transition, bulk update, batch, and archive flows
- plans
- raw Jira REST API requests

## Notes

- Current focus is tools, not prompts/resources/UI.
- Destructive operations require `confirm: true`.
- Attachment uploads support local file paths or inline base64 payloads.

## More docs

See the root README for example client configuration and workspace-level context.
