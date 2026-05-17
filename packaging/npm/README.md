# jirac for npm

[![npm version](https://img.shields.io/npm/v/%40mulham28%2Fjirac)](https://www.npmjs.com/package/@mulham28/jirac)
[![Docs](https://img.shields.io/badge/docs-jirac.keton.id-0ea5e9)](https://jirac.keton.id)
[![Crates.io](https://img.shields.io/crates/v/jira-commands.svg)](https://crates.io/crates/jira-commands)
[![OpenSSF Best Practices](https://www.bestpractices.dev/projects/12742/badge)](https://www.bestpractices.dev/projects/12742)

Install the **real `jirac` binary** with npm.

This package is a thin installer that downloads the matching prebuilt release for your platform during `postinstall`, so you get the same Rust CLI shipped on GitHub Releases — not a separate JavaScript reimplementation.

**Docs:** <https://jirac.keton.id>

## Why jirac

`jirac` is a fast Jira terminal client with:

- interactive TUI for browsing and updating issues, with full mouse support (click rows, tabs, picker options; scroll wheel)
- worklog flows, attachments, and comments
- bulk actions like bulk-comment, bulk-transition, and bulk-update
- JQL builder and saved-query workflows
- MCP-friendly tooling for editor and agent setups
- Jira Cloud and Jira Data Center support

## Install

```bash
npm install -g @mulham28/jirac
jirac auth login
jirac --help
```

## Quick examples

```bash
jirac issue list
jirac issue view PROJ-123
jirac issue transition PROJ-123 --to "In Progress"
jirac issue bulk-comment --jql 'project = PROJ AND sprint = openSprints()' --body 'Please post your update before standup'
jirac tui -p PROJ
```

## What npm installs

During install, npm fetches the release archive for your platform and wires up the `jirac` executable.

Supported targets:

- macOS arm64
- macOS x64
- Linux x64
- Linux arm64
- Windows x64

## Good next links

- **Docs:** <https://jirac.keton.id>
- **Install matrix:** <https://github.com/mulhamna/jira-commands/blob/main/INSTALL.md>
- **Source:** <https://github.com/mulhamna/jira-commands>
- **Issues:** <https://github.com/mulhamna/jira-commands/issues>

## Notes

- Requires Node.js 18+ for the installer wrapper.
- The downloaded CLI itself is a standalone native binary.
- If you prefer Cargo, Homebrew, Scoop, Winget, Chocolatey, or direct releases, see the docs site for the full install matrix.
