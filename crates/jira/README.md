# jira-commands

> **jirac** is an independent CLI tool for working with Jira from the terminal.
> It is **not** affiliated with, endorsed by, or sponsored by Atlassian.

`jira-commands` is the CLI crate in the `mulhamna/jira-commands` workspace. It ships the `jirac` binary, the legacy `jira` compatibility shim, interactive issue workflows, bulk operations, and the TUI.

[![CI](https://github.com/mulhamna/jira-commands/actions/workflows/ci.yml/badge.svg)](https://github.com/mulhamna/jira-commands/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/jira-commands.svg)](https://crates.io/crates/jira-commands)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

## Install

```bash
cargo install jira-commands
```

Or use one of the workspace-level install options from the root README:
- Homebrew
- install script
- GitHub Releases binaries

## What this crate provides

- `jirac` primary CLI binary
- `jira` legacy binary shim for backward compatibility
- issue listing, viewing, create, update, transition, delete
- worklog, attachment, bulk-update, bulk-transition, archive
- interactive TUI flows
- raw Jira REST API passthrough

## Quick start

```bash
jirac auth login
jirac issue list
jirac issue view MYPROJ-123
jirac issue create -p MYPROJ
jirac tui -p MYPROJ
```

## Binary rename note

The preferred binary name is `jirac`.

The old `jira` binary is still included as a compatibility shim, but it is deprecated and may be removed in a future major release. Update scripts and aliases to use `jirac`.

## Claude Code plugin

If you want Claude Code integration, install the plugin from the main workspace docs:

```text
/plugin marketplace add mulhamna/jira-commands
/plugin install jira@jira-commands
```

Useful skills include:
- `/jira:list-issues`
- `/jira:view-issue`
- `/jira:create-issue`
- `/jira:update-issue`
- `/jira:transition`
- `/jira:fields`
- `/jira:worklog`
- `/jira:bulk-transition`
- `/jira:attach`
- `/jira:jql`
- `/jira:api`

## More docs

See the root README for full installation, MCP usage, release artifacts, and examples.
