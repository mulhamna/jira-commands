---
description: List Jira issues — by project, assignee, or custom JQL query
---

List Jira issues using the `jirac` CLI.

Steps:
1. Check if `jirac` binary is available by running `jirac --version`. If not found, tell the user to install it with `cargo install jira-commands` (binary: `jirac`).
2. Determine the query parameters from the user's request:
   - If a project key is mentioned (e.g. "PROJ"), use `-p PROJ`
   - If a JQL expression is mentioned, use `--jql '<expression>'`
   - If neither, run without flags (defaults to `assignee = currentUser()`)
3. Run: `jirac issue list [flags]`
4. Display the output clearly.

TUI mode (interactive):
- `jirac issue list` (or with `-p`) opens a full-screen TUI
- Press `/` to open the JQL search bar — cursor is visible while typing
- Press Enter to execute the search, Esc to cancel
- Press `?` for full keyboard shortcut help

Examples:
- "list my issues" → `jirac issue list`
- "list issues in PROJ" → `jirac issue list -p PROJ`
- "list open bugs in PROJ" → `jirac issue list -p PROJ --jql 'project = PROJ AND issuetype = Bug AND status != Done'`
