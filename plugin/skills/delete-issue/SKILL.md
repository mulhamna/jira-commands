---
description: Permanently delete a Jira issue with jirac, with optional force flag to skip confirmation
---

Delete a Jira issue using `jirac`.

Steps:
1. Check that `jirac` is available by running `jirac --version`. If it is missing, tell the user to install it with `cargo install jira-commands`.
2. Extract the issue key from the user's request.
3. Warn the user: deletion is permanent and also deletes all subtasks.
4. Run `jirac issue delete <KEY>` — jirac will prompt for confirmation unless `--force` is passed.
5. Confirm deletion result clearly.

Examples:
- "delete PROJ-123" → `jirac issue delete PROJ-123`
- "delete PROJ-123 without asking" → `jirac issue delete PROJ-123 --force`
- "hapus issue PROJ-456" → `jirac issue delete PROJ-456`
