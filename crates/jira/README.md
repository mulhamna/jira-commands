# jira-commands

> **jirac** is an independent CLI tool for working with Jira from the terminal.
> It is **not** affiliated with, endorsed by, or sponsored by Atlassian.

`jira-commands` is the CLI crate in the `mulhamna/jira-commands` workspace. It ships the `jirac` binary, interactive issue workflows, bulk operations, and the TUI.

[![CI](https://github.com/mulhamna/jira-commands/actions/workflows/ci.yml/badge.svg)](https://github.com/mulhamna/jira-commands/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/jira-commands.svg)](https://crates.io/crates/jira-commands)
[![License: MIT%20OR%20Apache--2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../../LICENSE)

## Install

```bash
cargo install jira-commands
```

Or use one of the workspace-level install options from the root README:
- Homebrew
- shell installer on macOS/Linux
- PowerShell installer on Windows
- GitHub Releases archives and raw binaries

## What this crate provides

- `jirac` primary CLI binary
- issue listing, viewing, create, update, transition, delete
- worklog, attachment, bulk-update, bulk-transition, archive
- interactive TUI flows, including searchable assignee picker, project-scoped component picker, and saved column settings
- raw Jira REST API passthrough

## Quick start

```bash
jirac auth login
jirac issue list
jirac issue view MYPROJ-123
jirac issue create -p MYPROJ
jirac tui -p MYPROJ
```

## Migration note

The supported CLI binary is `jirac`.

If you still have old scripts, aliases, or local wrappers that call `jira`, update them to use `jirac` before upgrading.

## Claude Code plugin

If you want Claude Code integration, install the plugin from the main workspace docs:

```text
/plugin marketplace add mulhamna/jira-commands
/plugin install jira@jira-commands
```

The plugin/package is the right discovery surface for ClawHub or Claude Code marketplace flows. Keep the `jirac` binary installation on the normal CLI distribution paths.

Useful skills include:
- `/jira:list-issues`
- `/jira:view-issue`
- `/jira:create-issue`
- `/jira:update-issue`
- `/jira:transition`
- `/jira:fields`
- `/jira:comment`
- `/jira:worklog`
- `/jira:bulk-transition`
- `/jira:attach`
- `/jira:jql`
- `/jira:api`

## More docs

See the root README for full installation, MCP usage, release artifacts, and examples.
