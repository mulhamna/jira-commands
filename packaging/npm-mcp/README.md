# jirac-mcp for npm

[![npm version](https://img.shields.io/npm/v/%40mulham28%2Fjirac-mcp)](https://www.npmjs.com/package/@mulham28/jirac-mcp)
[![Docs](https://img.shields.io/badge/docs-jirac.keton.id-0ea5e9)](https://jirac.keton.id)
[![Crates.io](https://img.shields.io/crates/v/jira-mcp.svg)](https://crates.io/crates/jira-mcp)

Install the **MCP server for Jira** (`jirac-mcp`) with npm.

This package is a thin installer that downloads the matching prebuilt release for your platform during `postinstall`, so you get the same Rust binary shipped on GitHub Releases — not a separate JavaScript reimplementation.

**Docs:** <https://jirac.keton.id>

## Install

```bash
npm install -g @mulham28/jirac-mcp
```

## Register the MCP server with your CLI

If the [jirac CLI](https://www.npmjs.com/package/@mulham28/jirac) is also installed, you can register the server interactively:

```bash
jirac mcp install
```

The interactive picker checks prerequisites (binary on PATH, Jira auth configured) and writes the right config file for your client (Claude Code, Claude Desktop, Cursor, Codex, Gemini CLI, OpenCode, Zed, or a generic JSON snippet).

Without the jirac CLI, register manually — for example, OpenCode:

```jsonc
// ~/.config/opencode/opencode.jsonc
{
  "mcp": {
    "jira": {
      "type": "local",
      "command": ["jirac-mcp", "serve", "--transport", "stdio"],
      "enabled": true
    }
  }
}
```

Or Codex CLI:

```bash
codex mcp add jira -- jirac-mcp serve --transport stdio
```

## Companion packages

| Package | Crate | What it ships |
|---|---|---|
| [`@mulham28/jirac`](https://www.npmjs.com/package/@mulham28/jirac) | `jira-commands` | The Jira CLI + TUI (`jirac` binary) |
| [`@mulham28/jirac-mcp`](https://www.npmjs.com/package/@mulham28/jirac-mcp) | `jira-mcp` | The MCP server (`jirac-mcp` binary) — **this package** |

The CLI and the MCP server are independent — install whichever you need.

## License

MIT OR Apache-2.0
