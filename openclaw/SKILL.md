---
name: jirac
description: Use the jirac CLI from OpenClaw to list, view, create, update, transition, comment on, and manage Jira issues from agent workflows.
metadata: {"openclaw":{"emoji":"🎫","requires":{"bins":["jirac"]},"install":[{"id":"cargo","kind":"download","label":"Install jirac from GitHub Releases","url":"https://github.com/mulhamna/jira-commands/releases","extract":false}],"homepage":"https://github.com/mulhamna/jira-commands"}}
---

Use `jirac` as the OpenClaw-facing Jira skill surface.

This skill is the OpenClaw / ClawHub integration lane for the `jira-commands` project. It is intentionally separate from the Claude Code plugin lane in `plugin/`.

## What this skill should handle

- issue listing and JQL queries
- issue detail lookup
- issue creation and updates
- transitions, comments, worklogs, attachments
- safe guidance for when field metadata or interactive selection is needed

## Behavior

1. Verify `jirac` is available with `jirac --version` before using it.
2. Prefer direct `jirac` commands over raw Jira API calls when the CLI already supports the action.
3. Use `jirac issue fields` when required fields or custom fields are unclear.
4. Use `jirac issue transition <KEY>` without `--to` when the target status is not known.
5. Be explicit about destructive operations.

## Common command patterns

```bash
jirac issue list
jirac issue list -p PROJ
jirac issue list --jql 'project = PROJ AND status = "In Progress"'
jirac issue view PROJ-123
jirac issue create -p PROJ
jirac issue update PROJ-123 --summary 'New title'
jirac issue transition PROJ-123 --to 'Done'
jirac issue comment add PROJ-123 --body 'QA verified in staging'
jirac issue worklog add PROJ-123 --time '2h' --comment 'Implementation work'
jirac issue attach PROJ-123 ./screenshot.png
```

## Installation note

This OpenClaw skill should document OpenClaw-native installation and usage. Do not reuse Claude marketplace instructions here.
