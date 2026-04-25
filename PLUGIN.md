# Claude Code Plugin

The `jirac` Claude Code plugin exposes Jira operations as slash commands inside Claude Code. The plugin namespace is `/jira:*`.

The plugin has its own release lane under `plugin/`, with dedicated `plugin/VERSION` and `plugin/CHANGELOG.md` files. ClawHub publishing uses a separate skill lane under `clawhub/jirac/`.

## Setup

```bash
# 1. Install the CLI that the plugin calls
cargo install jira-commands
jirac auth login
```

```text
# 2. In Claude Code
/plugin marketplace add mulhamna/jira-commands
/plugin install jira@jira-commands
```

## Available skills

| Skill                   | Description                             |
| ----------------------- | --------------------------------------- |
| `/jira:list-issues`     | List issues by project or JQL           |
| `/jira:view-issue`      | View full issue detail                  |
| `/jira:create-issue`    | Create a new issue                      |
| `/jira:update-issue`    | Update an existing issue                |
| `/jira:transition`      | Transition an issue                     |
| `/jira:comment`         | List comments or add a Markdown comment |
| `/jira:worklog`         | List, add, or delete worklogs           |
| `/jira:fields`          | Inspect available field metadata        |
| `/jira:bulk-transition` | Bulk transition issues via JQL          |
| `/jira:attach`          | Upload a file to an issue               |
| `/jira:jql`             | Build and run a JQL query               |
| `/jira:api`             | Raw REST API passthrough                |
