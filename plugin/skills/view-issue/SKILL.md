---
description: View a Jira issue — show full detail, description, comments, and attachments
---

View the full detail of a Jira issue using the `jirac` CLI.

Steps:
1. Check if `jirac` binary is available by running `jirac --version`. If not found, tell the user to install it with `cargo install jira-commands` (binary: `jirac`).
2. Extract the issue key from the user's request (e.g. "PROJ-123"). If not provided, ask the user for it.
3. Run: `jirac issue view <ISSUE-KEY>`
4. Display the output. If the user asks follow-up questions about the issue content, answer based on the output shown.

Examples:
- "view PROJ-123" → `jirac issue view PROJ-123`
- "show me the details of PROJ-456" → `jirac issue view PROJ-456`
