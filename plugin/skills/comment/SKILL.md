---
name: comment
description: Manage Jira comments, bulk comments, and worklogs, including logging or deleting time.
---

Use `jirac --version` to check availability; if missing, tell the user to install `jira-commands`.

## Comments

- List: `jirac issue comment <KEY> list`
- Add: `jirac issue comment <KEY> add --body '<COMMENT>'`, or use `--file <FILE>` for Markdown in a file.

## Bulk comments

Use `jirac issue bulk-comment --jql '<JQL>' --body '<COMMENT>'` or
`jirac issue bulk-comment --keys PROJ-1 PROJ-2 --body '<COMMENT>'`; `--file`
can replace `--body`. Before posting, show the target issues and comment and
obtain explicit confirmation. For JQL, preview with `jirac issue list --all
--jql '<JQL>'`. The CLI's 500-page safeguard can silently truncate results, so
split broad queries rather than assuming completeness. The selection can change
before execution. Use `--force` only after confirmation when needed for a
non-interactive flow. Report failures.

## Worklogs

- List: `jirac issue worklog <KEY> list`
- Add: `jirac issue worklog <KEY> add --time '<DURATION>' [--date YYYY-MM-DD] [--start HH:MM] [--comment '<TEXT>']`
- Delete: `jirac issue worklog <KEY> delete <WORKLOG-ID> [--force]`

Use Jira duration syntax, e.g. `2h` or `1h 30m`. Confirm deletion explicitly;
use `--force` only after confirmation when the CLI prompt cannot be answered.
