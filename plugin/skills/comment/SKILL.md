---
description: Manage Jira issue comments with jirac, including listing existing comments and adding a new Markdown comment to an issue
---

Manage Jira comments using `jirac`.

Steps:
1. Check that `jirac` is available by running `jirac --version`. If it is missing, tell the user to install it with `cargo install jira-commands`.
2. Determine whether the user wants to list comments or add a comment.

For listing comments:
- extract the issue key
- run `jirac issue comment <ISSUE-KEY> list`

For adding a comment:
- extract the issue key
- extract the comment body
- run `jirac issue comment <ISSUE-KEY> add --body '<COMMENT>'`
- if the comment is long or already in a file, use `--file <FILE>` instead

3. Show the result clearly.

Examples:
- "show comments on PROJ-123" → `jirac issue comment PROJ-123 list`
- "add a comment to PROJ-123 saying QA verified this" → `jirac issue comment PROJ-123 add --body 'QA verified this'`
- "comment on PROJ-456 with the contents of note.md" → `jirac issue comment PROJ-456 add --file note.md`
