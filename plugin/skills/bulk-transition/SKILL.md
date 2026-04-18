---
description: Bulk transition multiple Jira issues to a new status using a JQL query
---

Bulk transition multiple Jira issues using the `jirac` CLI.

Steps:
1. Check if `jirac` binary is available by running `jirac --version`. If not found, tell the user to install it with `cargo install jira-commands` (binary: `jirac`).
2. Extract from the user's request:
   - Project key (required) — ask if not mentioned
   - Target status (required) — ask if not mentioned
   - JQL filter (optional) — if not mentioned, default to all issues in project
   - `--force` flag — use if user explicitly says "without confirmation" or "force"
3. Build the JQL: if no custom JQL, use `project = <PROJECT>`
4. Run: `jirac issue bulk-transition -p <PROJECT> -q '<JQL>' -t '<STATUS>' [--force]`
5. The CLI will ask for confirmation unless `--force` is used. Inform the user how many issues will be affected.

Examples:
- "transition all To Do issues in PROJ to In Progress" → `jirac issue bulk-transition -p PROJ -q 'project = PROJ AND status = "To Do"' -t 'In Progress'`
- "close all done issues in PROJ without asking" → `jirac issue bulk-transition -p PROJ -q 'project = PROJ AND status = Done' -t 'Closed' --force`
