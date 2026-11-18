---
name: bulk
description: Run Jira bulk create, transition, update, mixed batch, and archive operations.
---

Use `jirac --version` to check availability; if missing, tell the user to install `jira-commands`.

Before any bulk mutation, show the selected issues and intended changes and
obtain explicit confirmation. For JQL, use `jirac issue list --all --jql
'<JQL>'` to bypass a configured result limit. The CLI has a 500-page safeguard
that can silently truncate results, so split broad queries rather than assuming
the preview is complete. Results can change between preview and execution. Use
`--force` only after confirmation when needed to bypass the CLI prompt in
non-interactive agent flows. Report successes and failures.

## Bulk create

Save a JSON array to a platform-appropriate temporary file and run
`jirac issue bulk-create --manifest <FILE>`. Each object requires `project`
and `summary`; `type` defaults to `Task`. Optional fields include `assignee`,
`priority`, `labels`, `components`, `parent`, `fix_versions`, `description`
(Markdown), and `fields` (an object keyed by discovered field IDs).

```json
[
  { "project": "PROJ", "summary": "New task", "type": "Task", "labels": ["backend"] }
]
```

## Bulk transition and update

- `jirac issue bulk-transition --jql '<JQL>' --to '<STATUS>' [--force]`
- `jirac issue bulk-update --jql '<JQL>' [--assignee EMAIL|me] [--priority LEVEL] [--force]`

Bulk update requires assignee or priority. Bulk transition resolves the
transition using the first matching issue; inspect per-issue failures if
workflows differ.

## Mixed batch

Save an array to a platform-appropriate temporary file and run
`jirac issue batch --manifest <FILE>`. Operations are applied without a
confirmation prompt, including archives: review the entire manifest and obtain
explicit confirmation first.

```json
[
  { "op": "create", "project": "PROJ", "summary": "New task", "type": "Task" },
  { "op": "update", "key": "PROJ-10", "priority": "High" },
  { "op": "transition", "key": "PROJ-11", "to": "Done" },
  { "op": "archive", "key": "PROJ-12" }
]
```

`create` supports the bulk-create fields; `update` supports `summary`,
`assignee`, `priority`, `labels`, and `components`. Batch updates replace
supplied labels and components rather than adding to them.

## Archive

Archive hides issues from normal searches and cannot be undone by this CLI.
After reviewing the preview and confirming, run
`jirac issue archive --jql '<JQL>' [--force]`. This requires sufficient Jira
permissions; restoring archived issues may require administrator assistance.
