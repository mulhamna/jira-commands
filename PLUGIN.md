# Claude Code Plugin

The `jirac` Claude Code plugin exposes Jira operations as slash commands inside Claude Code. The plugin namespace is `/jira:*`.

The Claude Code plugin has its own release lane under `plugin/`, with dedicated `plugin/VERSION` and `plugin/CHANGELOG.md` files. ClawHub publishing is separate: the `clawhub/jirac/` skill ships independently, and the `clawhub/jirac-plugin/` wrapper package points back to `plugin/` for ClawHub package publishing.

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

| Skill | Description |
| --- | --- |
| `/jira:issue` | Manage issues, watchers, links, and attachments |
| `/jira:comment` | Manage comments and worklogs |
| `/jira:bulk` | Run bulk and batch operations |
| `/jira:search` | Search issues, build JQL, and prepare summaries |
| `/jira:api` | Call the REST API, inspect fields, and preview Markdown as ADF |
| `/jira:sprint` | Manage sprints |
| `/jira:versions` | Browse project versions and backlog previews |
| `/jira:transition` | Transition an issue |

For mappings from removed skill names, see the [migration table](plugin/README.md#skills).
