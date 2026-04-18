---
description: Manage Jira worklogs — list, add, or delete time entries on an issue
---

Manage worklogs on a Jira issue using the `jirac` CLI.

Steps:
1. Check if `jirac` binary is available by running `jirac --version`. If not found, tell the user to install it with `cargo install jira-commands` (binary: `jirac`).
2. Determine the action from the user's request:

   **List worklogs:**
   - Extract issue key — ask if not mentioned
   - Run: `jirac issue worklog list <ISSUE-KEY>`

   **Add worklog:**
   - Extract issue key and time spent (e.g. "2h", "30m", "1h 30m") — ask if not mentioned
   - Optionally extract comment
   - Run: `jirac issue worklog add <ISSUE-KEY> --time '<TIME>' [--comment '<COMMENT>']`

   **Delete worklog:**
   - Extract issue key and worklog ID (get from list first if needed)
   - Run: `jirac issue worklog delete <ISSUE-KEY> --id <WORKLOG-ID>`

3. Show the CLI output clearly.

Examples:
- "show worklogs for PROJ-123" → `jirac issue worklog list PROJ-123`
- "log 2 hours on PROJ-123" → `jirac issue worklog add PROJ-123 --time '2h'`
- "log 1h 30m on PROJ-456 for fixing login bug" → `jirac issue worklog add PROJ-456 --time '1h 30m' --comment 'fixing login bug'`
- "delete worklog 10234 on PROJ-123" → `jirac issue worklog delete PROJ-123 --id 10234`
