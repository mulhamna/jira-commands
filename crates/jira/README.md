# jira-commands

> **jirac** is an independent CLI tool that helps you work with the Jira ecosystem from your terminal.
> It is **not** affiliated with, endorsed by, or sponsored by Atlassian.

A fast, cross-platform Jira terminal client built in Rust.

This crate is released from the Rust workspace in the `mulhamna/jira-commands` repository.

[![CI](https://github.com/mulhamna/jira-commands/actions/workflows/ci.yml/badge.svg)](https://github.com/mulhamna/jira-commands/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/jira-commands.svg)](https://crates.io/crates/jira-commands)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

## Installation

```bash
# curl (macOS/Linux) — quickest
curl -sSL https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.sh | bash

# Homebrew (macOS/Linux)
brew tap mulhamna/tap && brew install jira-commands

# cargo
cargo install jira-commands
```

Or download a pre-built binary from [GitHub Releases](https://github.com/mulhamna/jira-commands/releases).

## Quick start

```bash
# Set up credentials
jirac auth login

# List your issues
jirac issue list

# List issues by project
jirac issue list -p MYPROJ

# View an issue
jirac issue view MYPROJ-123

# Create an issue (interactive)
jirac issue create -p MYPROJ

# Transition an issue
jirac issue transition MYPROJ-123 --to "In Progress"

# Upload an attachment
jirac issue attach MYPROJ-123 ./screenshot.png

# Log time
jirac issue worklog add MYPROJ-123 --time 2h --comment "Fixed auth bug"

# Bulk transition
jirac issue bulk-transition -p MYPROJ -q 'status = "To Do"' -t "In Progress"

# Raw API passthrough
jirac api get /rest/api/3/serverInfo

# Interactive TUI
jirac tui -p MYPROJ
```

## Upgrading from v0.x

The binary was renamed from `jira` to `jirac` in v0.7.0. The old `jira` binary is still included for backward compatibility but will be removed in a future major release. Please update your scripts and aliases to use `jirac`.

## Claude Code plugin

Install the plugin to manage Jira directly from Claude Code:

```bash
jirac auth login   # set up credentials first
```

```
/plugin marketplace add mulhamna/jira-commands
/plugin install jira@jira-commands
```

Available skills: `/jira:list-issues`, `/jira:create-issue`, `/jira:transition`, `/jira:worklog`, `/jira:bulk-transition`, `/jira:attach`, `/jira:view-issue`, `/jira:jql`, `/jira:api`

## Full documentation

See [github.com/mulhamna/jira-commands](https://github.com/mulhamna/jira-commands) for complete documentation.
