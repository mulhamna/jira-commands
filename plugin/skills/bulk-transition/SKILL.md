---
description: Bulk transition multiple Jira issues with a JQL filter through the jirac CLI
---

Bulk transition multiple Jira issues using `jirac`.

Steps:
1. Check that `jirac` is available by running `jirac --version`. If it is missing, tell the user to install it with `cargo install jira-commands`.
2. Extract from the user's request:
   - project key when relevant
   - target status
   - JQL filter, or enough context to build one
   - whether the user wants to skip confirmation
3. If no JQL is provided, build one from the request, for example `project = PROJ AND status = "To Do"`.
4. Run `jirac issue bulk-transition --jql '<JQL>' --to '<STATUS>' [--force]`.
5. Make it clear how many issues are affected and whether confirmation is still required.

Examples:
- "transition all To Do issues in PROJ to In Progress" → `jirac issue bulk-transition --jql 'project = PROJ AND status = "To Do"' --to 'In Progress'`
- "close all done issues in PROJ without asking" → `jirac issue bulk-transition --jql 'project = PROJ AND status = Done' --to 'Closed' --force`
- "move my open sprint issues to Done" → `jirac issue bulk-transition --jql 'assignee = currentUser() AND sprint = openSprints()' --to Done`
