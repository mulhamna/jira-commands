---
description: Update existing Jira issues with jirac, including summary, description, assignee, priority, labels, components, fix versions, and custom fields
---

Update a Jira issue using `jirac`.

Steps:
1. Check that `jirac` is available by running `jirac --version`. If it is missing, tell the user to install it with `cargo install jira-commands`.
2. Extract the issue key and requested changes.
3. Map the request to supported flags:
   - `--summary`, `--assignee` (email or "me"), `--priority`
   - `--labels <LABEL,...>`, `--components <NAME,...>`, `--fix-versions <VERSION,...>`
   - `--parent <ISSUE-KEY>` to re-parent an issue
   - `--description-file <FILE>` (Markdown input)
   - `--field <ID=VALUE>` for custom fields (repeatable, e.g. `--field customfield_10016=5`)
4. Run `jirac issue update <ISSUE-KEY> ...` with only the requested fields.
5. If custom field IDs are unclear, run `jirac issue fields -p <PROJECT> --issue-type '<TYPE>'` first.
6. Confirm the update result clearly.

Examples:
- "update PROJ-123 summary to fix OAuth callback" → `jirac issue update PROJ-123 --summary 'fix OAuth callback'`
- "set PROJ-123 priority to High and assign to me" → `jirac issue update PROJ-123 --priority High --assignee me`
- "add labels backend,api to PROJ-123" → `jirac issue update PROJ-123 --labels backend,api`
- "set fix version to v2.0 on PROJ-123" → `jirac issue update PROJ-123 --fix-versions v2.0`
- "set story points to 8 on PROJ-123" → `jirac issue update PROJ-123 --field story_points=8`
