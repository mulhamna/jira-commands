---
description: Manage Jira worklogs with jirac, including list, add, and delete flows for issue time tracking
---

Manage Jira worklogs using `jirac`.

Steps:
1. Check that `jirac` is available by running `jirac --version`. If it is missing, tell the user to install it with `cargo install jira-commands`.
2. Determine the requested action.

For listing worklogs:
- extract the issue key
- run `jirac issue worklog <ISSUE-KEY> list`

For adding a worklog:
- extract the issue key and time spent
- optionally extract the comment
- optionally extract a work date (`YYYY-MM-DD`) and start time (`HH:MM` or `HH:MM:SS`) when the user wants to backfill or set the started timestamp explicitly
- run `jirac issue worklog <ISSUE-KEY> add --time '<TIME>' [--date '<YYYY-MM-DD>'] [--start '<HH:MM[:SS]>'] [--comment '<COMMENT>']`

For deleting a worklog:
- extract the issue key and worklog ID
- run `jirac issue worklog <ISSUE-KEY> delete <WORKLOG-ID>`

3. Show the result clearly.

Examples:
- "show worklogs for PROJ-123" → `jirac issue worklog PROJ-123 list`
- "log 2 hours on PROJ-123" → `jirac issue worklog PROJ-123 add --time '2h'`
- "log 1h 30m on PROJ-456 for fixing login bug" → `jirac issue worklog PROJ-456 add --time '1h 30m' --comment 'fixing login bug'`
- "log 2 hours on PROJ-123 starting 2026-04-21 09:30" → `jirac issue worklog PROJ-123 add --time '2h' --date '2026-04-21' --start '09:30'`
- "delete worklog 10234 on PROJ-123" → `jirac issue worklog PROJ-123 delete 10234`
