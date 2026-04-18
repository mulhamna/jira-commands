# jira-commands

A fast, cross-platform Jira terminal client built in Rust.

[![CI](https://github.com/mulhamna/jira-commands/actions/workflows/ci.yml/badge.svg)](https://github.com/mulhamna/jira-commands/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/jira-commands.svg)](https://crates.io/crates/jira-commands)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

## Installation

```bash
# Homebrew (macOS/Linux)
brew tap mulhamna/tap && brew install jira-commands

# cargo
cargo install jira-commands
```

Or download a pre-built binary from [GitHub Releases](https://github.com/mulhamna/jira-commands/releases).

## Quick start

```bash
# Set up credentials
jira auth login

# List your issues
jira issue list

# List issues by project
jira issue list -p MYPROJ

# View an issue
jira issue view MYPROJ-123

# Create an issue (interactive)
jira issue create -p MYPROJ

# Transition an issue
jira issue transition MYPROJ-123 --to "In Progress"

# Upload an attachment
jira issue attach MYPROJ-123 ./screenshot.png

# Log time
jira issue worklog add MYPROJ-123 --time 2h --comment "Fixed auth bug"

# Bulk transition
jira issue bulk-transition -p MYPROJ -q 'status = "To Do"' -t "In Progress"

# Raw API passthrough
jira api get /rest/api/3/serverInfo

# Interactive TUI
jira tui -p MYPROJ
```

## Claude Code plugin

Install the plugin to manage Jira directly from Claude Code:

```
/plugin marketplace add mulhamna/jira-commands
/plugin install jira@jira-commands
```

Available skills: `/jira:list-issues`, `/jira:create-issue`, `/jira:transition`, `/jira:worklog`, `/jira:bulk-transition`, `/jira:attach`, `/jira:view-issue`, `/jira:jql`, `/jira:api`

## Full documentation

See [github.com/mulhamna/jira-commands](https://github.com/mulhamna/jira-commands) for complete documentation.
