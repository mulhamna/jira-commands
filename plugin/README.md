# jira — Claude Code Plugin

Manage Jira issues directly from Claude Code. Create, list, view, transition, comment, log time, and run bulk operations without leaving your editor.

> **jirac** is an independent Claude Code plugin for the Jira ecosystem. Not affiliated with or endorsed by Atlassian.

> **Requires** the `jirac` CLI to be installed: `cargo install jira-commands`
>
> **Compatibility note:** the next release requires `jirac`. If you still have old scripts or aliases that call `jira`, update them before upgrading.

> **Also available:** if you prefer MCP clients over Claude Code skills, install `jirac-mcp` with `cargo install jira-mcp`.

---

## Installation

**Step 1 — Install the CLI:**

```bash
cargo install jira-commands
jirac auth login
```

**Step 2 — Add marketplace and install plugin inside Claude Code:**

```
/plugin marketplace add mulhamna/jira-commands
/plugin install jira@jira-commands
```

---

## Skills

| Skill | Description |
|---|---|
| `/jira:list-issues` | List issues by project, assignee, or custom JQL |
| `/jira:view-issue` | View full issue detail — description, status, assignee, attachments |
| `/jira:create-issue` | Create a new issue with interactive field prompts |
| `/jira:update-issue` | Update summary, description, assignee, labels, versions, or custom fields |
| `/jira:transition` | Move an issue to a new status (e.g. In Progress, Done) |
| `/jira:comment` | List comments or add a new Markdown comment on an issue |
| `/jira:worklog` | List, add, or delete time entries on an issue |
| `/jira:bulk-transition` | Transition multiple issues at once via JQL query |
| `/jira:attach` | Upload a file or image to an issue |
| `/jira:fields` | Inspect available Jira fields for a project and issue type |
| `/jira:jql` | Build and run a JQL query interactively |
| `/jira:api` | Execute any raw Jira REST API call (GET, POST, PUT, DELETE, PATCH) |

---

## Use cases

**Daily standup prep**
> "list my in-progress issues" → `/jira:list-issues`

Claude lists all issues currently assigned to you with status In Progress.

---

**Create a bug report from a stack trace**
> "create a bug in PROJ for this null pointer exception"

Claude runs `/jira:create-issue`, sets type to Bug, and uses the stack trace as the description.

---

**Transition after a PR merge**
> "mark PROJ-123 as done"

Claude runs `/jira:transition` and moves the issue to Done in one step.

---

**Leave a follow-up comment**
> "comment on PROJ-456 that QA verified the fix in staging"

Claude runs `/jira:comment` and adds the requested Markdown comment.

---

**Log time at end of day**
> "log 3 hours on PROJ-456 for implementing the login flow"

Claude runs `/jira:worklog` with `--time 3h` and the comment filled in.

---

**Bulk close resolved issues**
> "close all done issues in PROJ that haven't been updated in 30 days"

Claude builds the JQL and runs `/jira:bulk-transition` with `--force`.

---

**Explore the API**
> "get the field schema for project PROJ"

Claude calls `/jira:api` with the appropriate REST endpoint and shows the raw JSON.

---

## Configuration

Credentials are stored at `~/.config/jira/config.toml` after running `jirac auth login`. The plugin reads from the same config — no extra setup needed.

You can also use environment variables:

```bash
export JIRA_URL=https://yourcompany.atlassian.net
export JIRA_EMAIL=you@example.com
export JIRA_TOKEN=your_api_token
```

---

## Source

[github.com/mulhamna/jira-commands](https://github.com/mulhamna/jira-commands)
