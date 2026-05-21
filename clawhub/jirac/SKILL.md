---
name: jirac
description: Jira issue management skill for OpenClaw using the jirac CLI. Requires the `jirac` binary to be installed and authenticated before use. Use when listing, viewing, creating, updating, transitioning, commenting on, attaching files to, bulk-editing, cloning, deleting, linking, archiving, moving, changing types of Jira issues, logging work, generating standup or sprint summaries, managing sprint lifecycles, browsing fix versions, scanning Jira mention notifications, rendering markdown to ADF, registering jirac-mcp into MCP clients, or running raw Jira REST calls from agent workflows.
metadata: {"openclaw":{"emoji":"🎫","requires":{"bins":["jirac"]},"install":[{"id":"github-releases","kind":"download","label":"Install jirac from GitHub Releases","url":"https://github.com/mulhamna/jira-commands/releases","extract":false}],"homepage":"https://github.com/mulhamna/jira-commands"}}
---

Use `jirac` as the Jira execution surface. The CLI is a single Rust binary that supports Jira Cloud and Jira Data Center, multi-profile auth, runtime custom-field discovery, attachment upload, sprint lifecycle, and an interactive TUI with full mouse support.

## Requirements

- Require the `jirac` binary from the official `jira-commands` release source.
- Require Jira authentication to be configured before use, typically via `jirac auth login` in the target environment.
- Treat Jira credentials, local config (`~/.config/jira/config.toml`), and attachment file paths as sensitive.
- For MCP-based agent setups, additionally require the `jirac-mcp` binary (separate package).

## Pre-flight

1. Run `jirac --version` to confirm the binary is installed.
2. Run `jirac auth status` to confirm credentials are configured and reachable. If not, prompt the user to run `jirac auth login` (or `jirac auth login --profile NAME` for separate accounts).
3. For multi-profile setups, run `jirac auth profiles` and `jirac auth use <name>` to switch the active profile before issuing operations.

## Workflow

1. Prefer direct `jirac` commands over raw Jira REST API (`jirac api`) calls when the CLI already supports the action.
2. Use `jirac issue fields -p PROJ --issue-type Type` when required fields, allowed values, or custom fields are unclear.
3. Use `jirac issue transition <KEY>` without a transition argument to get an interactive picker when the target status is unknown.
4. Use `jirac issue standup` for a concise personal update from assigned work, and `jirac issue sprint-summary -p PROJ` for a project sprint rollup.
5. Use `jirac issue sprints -p PROJ` to see sprint lifecycle states before creating, starting, completing, updating, or deleting a sprint.
6. Use `jirac issue notifications` to scan Jira @mentions from issue descriptions and comments.
7. Use `jirac issue render --input file.md` to preview how Markdown will convert to Jira ADF before sending it as description or comment.
8. Use `jirac tui -p PROJ` for any interactive bulk browse / triage / quick-edit workflow — supports mouse (click rows, tabs, picker options; scroll wheel; drag splitter; click `[?]` / `[🔔]`).
9. Confirm intent before destructive or high-impact operations: `delete`, `archive`, `bulk-update`, `bulk-transition`, `bulk-comment`, `bulk-create`, `change-type`, `move`, `sprint-delete`, and attachment uploads.
10. Keep Jira project keys, issue keys, status names, and transition names exact. Use quotes for any name with spaces.

## Common commands

```bash
# Auth and profiles
jirac auth login                              # interactive, current shell profile
jirac auth login --profile work-cloud         # named profile
jirac auth status
jirac auth profiles
jirac auth use work-cloud

# List, search, summaries
jirac issue list                              # your assigned issues
jirac issue list -p PROJ
jirac issue list --jql 'project = PROJ AND status = "In Progress"'
jirac issue standup                           # daily standup rollup
jirac issue sprint-summary -p PROJ            # current sprint
jirac issue sprint-summary -p PROJ --sprint 'Sprint 24'
jirac issue notifications                     # @mentions scan
jirac issue view PROJ-123
jirac issue view PROJ-123 --versions          # include fix-version block
jirac issue fields -p PROJ --issue-type Bug
jirac issue jql --run                         # interactive JQL builder

# Sprint lifecycle
jirac issue sprints -p PROJ
jirac issue sprint-create -p PROJ --name 'Sprint 25' --goal 'Ship auth v2'
jirac issue sprint-start <SPRINT_ID>
jirac issue sprint-complete <SPRINT_ID>
jirac issue sprint-update <SPRINT_ID> --name 'Sprint 25 (extended)' --goal 'New goal'
jirac issue sprint-delete <SPRINT_ID> --force # destructive

# Versions
jirac issue versions -p PROJ                  # browse + backlog preview
jirac issue versions -p PROJ --version 'v1.2.0'

# Create
jirac issue create -p PROJ                                                          # interactive
jirac issue create -p PROJ -t Bug -s 'login crash' --assignee me
jirac issue create -p PROJ -t Story -s 'auth flow' --sprint 'Sprint 24' --field story_points=5
jirac issue create -p PROJ -t Sub-task -s 'sub-task' --parent PROJ-100

# Update
jirac issue update PROJ-123 --summary 'New title'
jirac issue update PROJ-123 --priority High --assignee me
jirac issue update PROJ-123 --labels backend,api --fix-versions v2.0
jirac issue update PROJ-123 --field story_points=8

# Transition (positional, not --to)
jirac issue transition PROJ-123                # interactive picker
jirac issue transition PROJ-123 'Done'
jirac issue transition PROJ-123 'In Progress'

# Comment / bulk-comment
jirac issue comment list PROJ-123
jirac issue comment add PROJ-123 --body 'QA verified in staging'
jirac issue bulk-comment --jql 'project = PROJ AND sprint = openSprints()' --body 'Standup reminder'
jirac issue bulk-comment --keys PROJ-1,PROJ-2,PROJ-3 --body 'Please update'

# Worklog
jirac issue worklog list PROJ-123
jirac issue worklog add PROJ-123 --time '2h' --comment 'Implementation work'
jirac issue worklog delete PROJ-123 <WORKLOG_ID>

# Attach
jirac issue attach PROJ-123 ./screenshot.png

# Links between issues
jirac issue link list-types
jirac issue link add --link-type Blocks PROJ-123 PROJ-456            # PROJ-123 blocks PROJ-456
jirac issue link add --link-type Relates PROJ-123 PROJ-456
jirac issue link delete <LINK_ID>

# Clone, change-type, move, delete, archive
jirac issue clone PROJ-123                                # same project
jirac issue clone PROJ-123 --project NEWPROJ --summary 'Copy: original'
jirac issue change-type PROJ-123 Bug                      # native Jira move semantics
jirac issue move PROJ-123 NEWPROJ                         # cross-project, native (not clone+delete)
jirac issue delete PROJ-123                               # prompts confirmation
jirac issue delete PROJ-123 --force
jirac issue archive --jql 'project = PROJ AND resolution = Done AND updated < -180d'

# Bulk operations
jirac issue bulk-transition --jql 'project = PROJ AND status = "To Do"' --to 'In Progress'
jirac issue bulk-update --jql 'project = PROJ AND assignee = EMPTY' --assignee me
jirac issue bulk-update --jql 'project = PROJ AND priority = Low' --priority High --force
jirac issue bulk-create --manifest issues.json

# Batch (mixed ops from manifest)
jirac issue batch --manifest ops.json

# Render markdown → Jira ADF preview
jirac issue render --input description.md

# Raw REST passthrough (last resort)
jirac api GET /rest/api/3/myself
jirac api POST /rest/api/3/issue --body '{"fields":{...}}'

# TUI
jirac tui                                     # uses default project or assigned issues
jirac tui -p PROJ

# MCP server registration (optional)
jirac mcp doctor                              # check prereqs
jirac mcp install                             # interactive client picker
jirac mcp install --client claude-code
jirac mcp install --client cursor
```

## Bulk-create manifest format

```json
[
  {
    "project": "PROJ",
    "summary": "Issue title",
    "type": "Task",
    "assignee": "user@org.com",
    "priority": "High",
    "labels": ["backend"],
    "parent": "PROJ-100",
    "description": "Markdown description",
    "fields": { "customfield_10016": 5 }
  }
]
```

## Batch manifest format

```json
[
  { "op": "create",     "project": "PROJ", "summary": "New task", "type": "Task" },
  { "op": "update",     "key": "PROJ-10",  "priority": "High", "assignee": "me" },
  { "op": "transition", "key": "PROJ-11",  "to": "Done" },
  { "op": "archive",    "key": "PROJ-12" }
]
```

## JQL quoting

JQL strings often contain spaces, parentheses, and quotes — wrap the entire `--jql` value in single quotes in bash, then use double quotes inside:

```bash
jirac issue list --jql 'project = PROJ AND status = "In Progress" AND assignee = currentUser()'
```

For Windows PowerShell, the inverse — wrap in double quotes outside, escape inner doubles with `` ` ``:

```powershell
jirac issue list --jql "project = PROJ AND status = `"In Progress`""
```

See `references/jql.md` for common JQL recipes.

## Guidance

- Prefer interactive or metadata-assisted flows when field requirements are unclear (`jirac issue fields ...`, `jirac issue jql --run`, `jirac issue transition KEY`).
- Always confirm intent before operations that may change workflow state, bulk-edit, delete, archive, change type, or move issues across projects.
- Confirm that local files selected for attachment are intended and safe to upload.
- Prefer explicit project scoping (`-p PROJ`) when working across multiple Jira projects.
- `jirac issue transition` takes a positional transition name/ID — not `--to`.
- `jirac issue change-type` and `jirac issue move` use Jira's native move semantics; the issue key is preserved.
- For agents that need Jira inside an MCP client (Claude Code, Cursor, Codex, OpenCode, Gemini CLI, Zed), install `jirac-mcp` separately, then run `jirac mcp install` for an interactive registration picker.

## References

- Install guide: `references/install.md`
- JQL recipes: `references/jql.md`
