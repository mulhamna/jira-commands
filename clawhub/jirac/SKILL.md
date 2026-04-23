---
name: jirac
description: Jira issue management skill for OpenClaw using the jirac CLI. Requires the `jirac` binary to be installed and authenticated before use. Use when listing, viewing, creating, updating, transitioning, commenting on, attaching files to, or logging work against Jira issues from agent workflows.
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
5. Use `jirac issue transition <KEY>` without `--to` when the target status is not known yet.
6. Confirm intent before destructive or high-impact operations such as delete, bulk edits, transitions, and file attachments.

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
- Confirm intent before operations that may change workflow state, bulk-edit, delete, or overwrite issue content.
- Confirm that local files selected for attachment are intended and safe to upload.
- Keep Jira project keys, issue keys, and status names exact.
- Prefer explicit project scoping in commands when working across multiple Jira projects.

## References

- Install guide: `references/install.md`
