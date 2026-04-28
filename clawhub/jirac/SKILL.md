---
name: jirac
description: Jira issue management skill for OpenClaw using the jirac CLI. Requires the `jirac` binary to be installed and authenticated before use. Use when listing, viewing, creating, updating, transitioning, commenting on, attaching files to, bulk-editing, cloning, deleting, or logging work against Jira issues from agent workflows.
metadata: {"openclaw":{"emoji":"🎫","requires":{"bins":["jirac"]},"install":[{"id":"github-releases","kind":"download","label":"Install jirac from GitHub Releases","url":"https://github.com/mulhamna/jira-commands/releases","extract":false}],"homepage":"https://github.com/mulhamna/jira-commands"}}
---

Use `jirac` as the Jira execution surface.

## Requirements

- Require the `jirac` binary from the official `jira-commands` release source.
- Require Jira authentication to be configured before use, typically via `jirac auth login` in the target environment.
- Treat Jira credentials, local config, and attachment paths as sensitive.

## Workflow

1. Verify `jirac` is installed by running `jirac --version`.
2. Verify authentication is already configured before issue operations, for example with `jirac auth status` or a known-good prior login.
3. Prefer direct `jirac` commands over raw Jira API calls when the CLI already supports the action.
4. Use `jirac issue fields` when required fields or custom fields are unclear.
5. Use `jirac issue transition <KEY>` without a transition argument when the target status is not known yet — shows an interactive picker.
6. Confirm intent before destructive or high-impact operations such as delete, bulk edits, transitions, and file attachments.

## Common commands

```bash
# List and view
jirac issue list
jirac issue list -p PROJ
jirac issue list --jql 'project = PROJ AND status = "In Progress"'
jirac issue view PROJ-123

# Create
jirac issue create -p PROJ
jirac issue create -p PROJ -t Bug -s 'login crash' --assignee me
jirac issue create -p PROJ -t Story -s 'auth flow' --sprint 'Sprint 24' --field story_points=5
jirac issue create -p PROJ -t Sub-task -s 'sub-task' --parent PROJ-100

# Update
jirac issue update PROJ-123 --summary 'New title'
jirac issue update PROJ-123 --priority High --assignee me
jirac issue update PROJ-123 --labels backend,api --fix-versions v2.0
jirac issue update PROJ-123 --field story_points=8

# Transition (positional arg, not --to)
jirac issue transition PROJ-123              # interactive picker
jirac issue transition PROJ-123 'Done'
jirac issue transition PROJ-123 'In Progress'

# Comment and worklog
jirac issue comment add PROJ-123 --body 'QA verified in staging'
jirac issue worklog add PROJ-123 --time '2h' --comment 'Implementation work'

# Attach
jirac issue attach PROJ-123 ./screenshot.png

# Clone and delete
jirac issue clone PROJ-123
jirac issue clone PROJ-123 --project NEWPROJ --summary 'Copy: original'
jirac issue delete PROJ-123                  # prompts confirmation
jirac issue delete PROJ-123 --force

# Bulk operations
jirac issue bulk-transition --jql 'project = PROJ AND status = "To Do"' --to 'In Progress'
jirac issue bulk-update --jql 'project = PROJ AND assignee = EMPTY' --assignee me
jirac issue bulk-update --jql 'project = PROJ AND priority = Low' --priority High --force
jirac issue bulk-create --manifest issues.json

# Batch (mixed ops from manifest)
jirac issue batch --manifest ops.json

# Fields and JQL
jirac issue fields -p PROJ --issue-type Bug
jirac issue jql --run
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

## Guidance

- Prefer interactive or metadata-assisted flows when field requirements are unclear.
- Confirm intent before operations that may change workflow state, bulk-edit, delete, or overwrite issue content.
- Confirm that local files selected for attachment are intended and safe to upload.
- Keep Jira project keys, issue keys, and status names exact.
- Prefer explicit project scoping in commands when working across multiple Jira projects.
- `jirac issue transition` takes a positional transition name/ID — not `--to`.

## References

- Install guide: `references/install.md`
