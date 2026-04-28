---
description: Bulk update assignee or priority on multiple Jira issues matching a JQL query with jirac
---

Bulk update Jira issues using `jirac`.

Steps:
1. Check that `jirac` is available by running `jirac --version`. If it is missing, tell the user to install it with `cargo install jira-commands`.
2. Extract from the user's request:
   - JQL filter, or enough context to build one
   - new assignee (email or "me") and/or new priority
   - whether to skip confirmation
3. At least one of `--assignee` or `--priority` must be provided.
4. Run `jirac issue bulk-update --jql '<JQL>' [--assignee EMAIL|me] [--priority LEVEL] [--force]`.
5. Confirm how many issues were updated.

Priority levels: Highest, High, Medium, Low, Lowest

Examples:
- "assign all unassigned issues in PROJ to me" → `jirac issue bulk-update --jql 'project = PROJ AND assignee = EMPTY' --assignee me`
- "set all Low priority bugs in PROJ to High" → `jirac issue bulk-update --jql 'project = PROJ AND issuetype = Bug AND priority = Low' --priority High --force`
- "bulk assign sprint issues to alice@org.com" → `jirac issue bulk-update --jql 'sprint = openSprints()' --assignee alice@org.com`
