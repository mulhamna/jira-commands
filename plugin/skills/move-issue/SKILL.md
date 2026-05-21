---
description: Move a Jira issue to another project using Jira's native move semantics with jirac (not clone+delete)
---

Move an issue across projects using `jirac`. This uses Jira's native move — the issue keeps its history, comments, attachments, and worklogs. This is different from `clone --move`, which creates a new key and deletes the original.

Steps:
1. Check that `jirac` is available by running `jirac --version`. If missing, tell the user to install it (`cargo install jira-commands`).
2. Extract from the user's request:
   - source issue key
   - target project key
   - target issue type (optional — defaults to the current issue type name in the destination)
3. Confirm intent — moves can drop incompatible custom-field values and reassign workflow.
4. Run `jirac issue move <KEY> <PROJECT> [--issue-type <TYPE>]`.
5. Report the new key (Jira always re-keys on move) and link to view it with `jirac issue view <NEW_KEY>`.

Notes:
- Issue **key changes** (PROJ-123 → OTHER-45 or similar).
- Comments, attachments, worklogs, links, and history are **preserved**.
- If the target project does not have the same issue type, pass `--issue-type` with the closest equivalent (e.g. `Task`).
- Use `change-type` instead when staying inside the same project.

Examples:
- "move PROJ-123 to OTHER project" → `jirac issue move PROJ-123 OTHER`
- "move PROJ-50 to NEW project as a Task" → `jirac issue move PROJ-50 NEW --issue-type Task`
