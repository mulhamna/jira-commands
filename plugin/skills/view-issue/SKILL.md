---
description: View a Jira issue — show full detail, description, comments, and attachments
---

View the full detail of a Jira issue using the `jira` CLI.

Steps:
1. Check if `jira` binary is available by running `jira --version`. If not found, tell the user to install it with `cargo install jira-commands`.
2. Extract the issue key from the user's request (e.g. "PROJ-123"). If not provided, ask the user for it.
3. Run: `jira issue view <ISSUE-KEY>`
4. Display the output. If the user asks follow-up questions about the issue content, answer based on the output shown.

Examples:
- "view PROJ-123" → `jira issue view PROJ-123`
- "show me the details of PROJ-456" → `jira issue view PROJ-456`
