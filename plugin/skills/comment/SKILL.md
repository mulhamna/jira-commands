---
name: comment
description: Manage Jira comments and worklogs, including bulk comments, date-range logging, updates, and deletion.
---

Use `jirac --version` to check availability; if missing, tell the user to install `jira-commands`.

## Comments

- List: `jirac issue comment <KEY> list`
- Add: `jirac issue comment <KEY> add --body '<COMMENT>'`, or use `--file <FILE>`.
- Update: `jirac issue comment <KEY> update <COMMENT-ID> --body '<COMMENT>'`
  or use `--file <FILE>`.
- Delete: `jirac issue comment <KEY> delete <COMMENT-ID> [--force]`. Confirm
  deletion explicitly; use `--force` only after confirmation when needed.

Comment bodies, bulk comments, and worklog comments use the configured
`input_format`: Markdown by default, native `wiki` on API v2 only, or an `adf`
JSON document. Check `jirac config show`; `JIRA_INPUT_FORMAT` overrides the
profile for one command. Supply content in that format rather than assuming
`--file` means Markdown. Report formatting-loss warnings printed to stderr.
Comment list output renders API v2 wiki and API v3 ADF for display; it is not
a lossless source for editing.

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
- Log over an inclusive date range:
  `jirac issue worklog <KEY> add --time '<DURATION>' --from <DATE> --to <DATE> [--exclude-weekends] [--comment '<TEXT>']`.
  This creates one worklog per included date.
- Update: `jirac issue worklog <KEY> update <ID> [--time <DURATION>] [--comment '<TEXT>'] [--date YYYY-MM-DD] [--start HH:MM]`.
- Delete: `jirac issue worklog <KEY> delete <WORKLOG-ID> [--force]`

Use Jira duration syntax, e.g. `2h` or `1h 30m`. Confirm deletion explicitly;
use `--force` only after confirmation when the CLI prompt cannot be answered.
