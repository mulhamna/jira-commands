---
description: List Jira issues — by project, assignee, or custom JQL query
---

List Jira issues using the `jira` CLI.

Steps:
1. Check if `jira` binary is available by running `jira --version`. If not found, tell the user to install it with `cargo install jira-commands`.
2. Determine the query parameters from the user's request:
   - If a project key is mentioned (e.g. "PROJ"), use `-p PROJ`
   - If a JQL expression is mentioned, use `--jql '<expression>'`
   - If neither, run without flags (defaults to `assignee = currentUser()`)
3. Run: `jira issue list [flags]`
4. Display the output clearly.

Examples:
- "list my issues" → `jira issue list`
- "list issues in PROJ" → `jira issue list -p PROJ`
- "list open bugs in PROJ" → `jira issue list -p PROJ --jql 'project = PROJ AND issuetype = Bug AND status != Done'`
