# jira — Claude Code Plugin
[![OpenSSF Best Practices](https://www.bestpractices.dev/projects/12742/badge)](https://www.bestpractices.dev/projects/12742)

Manage Jira issues directly from Claude Code. Create, list, view, transition, comment, log time, and run bulk operations without leaving your editor.

This plugin is versioned and released independently from the main CLI/MCP workspace. See `plugin/VERSION` and `plugin/CHANGELOG.md` for the Claude-specific release lane.

> **jirac** is an independent Claude Code plugin for the Jira ecosystem. Not affiliated with or endorsed by Atlassian.

> **Requires** the `jirac` CLI to be installed: `cargo install jira-commands` (crates.io) or `cargo install --path crates/jira --locked` from a local checkout.
>
> You can also install `jirac` from Homebrew (`brew install mulhamna/tap/jira-commands`), npm (`npm install -g @mulham28/jirac`), the workspace shell installer on macOS/Linux, the PowerShell installer on Windows, or GitHub Releases.
>
> **Compatibility note:** the plugin requires `jirac`. Update old scripts or aliases that call `jira` to use `jirac`.

> **Also available:** if you prefer MCP clients over Claude Code skills, install `jirac-mcp` (`cargo install jira-mcp`, `npm install -g @mulham28/jirac-mcp`, or `brew install mulhamna/tap/jira-mcp`) and then run `jirac mcp install` for an interactive picker.
>
> **Positioning note:** this directory is the Claude Code surface only. OpenClaw / ClawHub should be modeled as a separate integration surface with its own packaging, docs, and release flow.

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

| Skill | Description | Replaces |
|---|---|---|
| `/jira:issue` | View and manage issues, watchers, links, and attachments | `view-issue`, `create-issue`, `update-issue`, `clone-issue`, `move-issue`, `change-type`, `delete-issue`, `watch`, `link`, `attach` |
| `/jira:comment` | Manage comments, bulk comments, and worklogs | `comment`, `bulk-comment`, `worklog` |
| `/jira:bulk` | Bulk create, transition, update, batch, and archive | `bulk-create`, `bulk-transition`, `bulk-update`, `batch`, `archive` |
| `/jira:search` | List and search issues, build JQL, prepare standups, summarize sprints, and find mentions | `list-issues`, `jql`, `daily-standup`, `sprint-summary`, `notifications` |
| `/jira:api` | Call the raw REST API, inspect fields, and preview Markdown as ADF | `api`, `render`, `fields` |
| `/jira:sprint` | Manage the sprint lifecycle | `sprint-lifecycle` |
| `/jira:versions` | Browse project fix versions and backlog previews | `versions` |
| `/jira:transition` | Transition an issue by status or ID | `transition` |

**Migration:** Some old skill names (for example `/jira:create-issue`) no
longer work. Update direct invocations to the replacement in the table;
`/jira:api`, `/jira:comment`, `/jira:transition`, and `/jira:versions` remain
available. The `jirac` CLI commands themselves are unchanged.

---

## Use cases

**Daily standup prep**
> "generate my standup for today" → `/jira:search`

Claude groups your assigned issues into recently done, in progress, next up, and blocked buckets.

---

**Create a bug report from a stack trace**
> "create a bug in PROJ for this null pointer exception"

Claude runs `/jira:issue`, sets type to Bug, and uses the stack trace as the description.

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

Claude uses `/jira:comment` to run the worklog command with `--time 3h` and the comment filled in.

---

**Bulk close resolved issues**
> "close all done issues in PROJ that haven't been updated in 30 days"

Claude uses `/jira:bulk` to build the JQL and run the bulk transition.

---

**Bulk status reminder**
> "comment on all sprint issues asking owners to post an update"

Claude can use `/jira:comment` with a JQL query or an explicit issue-key list.

---

**Explore the API**
> "get the field schema for project PROJ"

Claude calls `/jira:api` with the appropriate REST endpoint and shows the raw JSON.

---

**Sprint rollup**
> "summarize the current sprint in PROJ" → `/jira:search`

Claude summarizes sprint progress by status and assignee for a quick project pulse.

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
