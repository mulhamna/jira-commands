---
description: Change a Jira issue to another issue type using Jira's native move semantics with jirac
---

Change an issue's type (e.g. Bug → Story) using `jirac`. This uses Jira's native move semantics — the issue key is preserved.

Steps:
1. Check that `jirac` is available by running `jirac --version`. If missing, tell the user to install it (`cargo install jira-commands`).
2. Extract from the user's request:
   - issue key
   - target issue type name (e.g. Bug, Story, Task, Sub-task)
3. Confirm intent — type changes can affect required fields and screens.
4. Run `jirac issue change-type <KEY> <TYPE>`.
5. Report the updated issue and its new type. Use `jirac issue view <KEY>` if confirmation is needed.

Notes:
- Issue key is **preserved** (PROJ-123 stays PROJ-123).
- Some custom-field values may be dropped if the new type's screen excludes them — verify with `jirac issue fields -p PROJ --issue-type <TYPE>` beforehand.
- Use `jirac issue move` instead when moving across **projects**; use `change-type` when staying in the same project.

Examples:
- "change PROJ-123 to a Bug" → `jirac issue change-type PROJ-123 Bug`
- "convert PROJ-50 from Sub-task to Story" → `jirac issue change-type PROJ-50 Story`
