# jira-mcp

> **jirac-mcp** exposes Jira operations over the Model Context Protocol (MCP).
> It is **not** affiliated with, endorsed by, or sponsored by Atlassian.

`jira-mcp` is the MCP server crate in the `mulhamna/jira-commands` workspace. It reuses `jira-core` and exposes typed Jira tools for editors, assistants, and remote MCP clients.

[![License: MIT%20OR%20Apache--2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE)
[![OpenSSF Best Practices](https://www.bestpractices.dev/projects/12742/badge)](https://www.bestpractices.dev/projects/12742)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](http://makeapullrequest.com)

## Install

```bash
# Homebrew (macOS / Linux)
brew tap mulhamna/tap
brew install jira-mcp

# Cargo (crates.io)
cargo install jira-mcp

# npm (Node 18+)
npm install -g @mulham28/jirac-mcp

# Scoop (Windows)
scoop bucket add mulhamna https://github.com/mulhamna/scoop-bucket
scoop install mulhamna/jirac-mcp

# From a local checkout
cargo install --path crates/jira-mcp --locked
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
- issue list, view, create, update, delete, clone, batch flows, standups, sprint summaries, and notifications
- field and transition discovery
- comments (single + bulk)
- attachment upload
- worklog operations
- bulk transition, bulk update, and archive flows
- plans
- raw Jira REST API requests

## Notes

- Current focus is tools, not prompts/resources/UI.
- Destructive operations require `confirm: true`.
- `jira_issue_clone` can optionally delete the source issue with `move_original: true`, but only when `confirm: true` is also set.
- Attachment uploads support local file paths or inline base64 payloads.

## Client install helper

If you already have both `jirac` and `jirac-mcp` installed, register the MCP server into a supported client with:

```bash
jirac mcp doctor          # check prereqs only
jirac mcp install         # interactive picker (recommended)
```

The interactive flow verifies that `jirac-mcp` is on PATH and that Jira auth is configured, then lets you pick the client to install into. Pass `--client` explicitly to skip the picker in scripts:

```bash
jirac mcp install --client claude-code
jirac mcp install --client claude-desktop
jirac mcp install --client cursor
jirac mcp install --client gemini-cli
jirac mcp install --client codex
jirac mcp install --client opencode
jirac mcp install --client generic-json
```

Notes:
- `claude-code` writes user-level `~/.claude.json` (`mcpServers`); project-scope `.mcp.json` is not written by this helper
- `claude-desktop` writes the platform support dir (`claude_desktop_config.json`) — macOS Library, Windows APPDATA, Linux XDG
- `gemini-cli` and `codex` delegate to their native CLI `mcp add` flows; `opencode` writes `~/.config/opencode/opencode.jsonc` directly
- `generic-json` prints a portable JSON snippet instead of writing a file
- `cursor` remains provisional until verified in a real Cursor install

## More docs

See the root README and `INSTALL.md` for client-specific install notes, helper target details, and workspace-level context.
