---
name: jirac
description: Jira issue management skill for OpenClaw using the jirac CLI. Use when listing, viewing, creating, updating, transitioning, commenting on, attaching files to, or logging work against Jira issues from agent workflows.
---

Use `jirac` as the Jira execution surface.

## Workflow

1. Verify `jirac` is installed by running `jirac --version`.
2. Prefer direct `jirac` commands over raw Jira API calls when the CLI already supports the action.
3. Use `jirac issue fields` when required fields or custom fields are unclear.
4. Use `jirac issue transition <KEY>` without `--to` when the target status is not known yet.
5. Be explicit before destructive or high-impact operations.

## Common commands

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

## Guidance

- Prefer interactive or metadata-assisted flows when field requirements are unclear.
- Confirm intent before operations that may change workflow state, bulk-edit, or overwrite issue content.
- Keep Jira project keys, issue keys, and status names exact.
