---
description: Clone a Jira issue with jirac, optionally into another project or with a new summary and assignee
---

Clone a Jira issue using `jirac`.

Steps:
1. Check that `jirac` is available by running `jirac --version`. If it is missing, tell the user to install it with `cargo install jira-commands`.
2. Extract from the user's request:
   - source issue key
   - target project (if different from source)
   - override summary (optional)
   - assignee for the clone (optional — source assignee is NOT copied by default)
   - whether to delete the original after cloning (`--move`)
3. Run `jirac issue clone <KEY> [--project TARGET] [--summary TEXT] [--assignee EMAIL|me] [--move]`.
4. Confirm the cloned issue key clearly.

Notes:
- Copies: summary, description, type, priority, labels, components, fix versions.
- Does NOT copy: assignee (unless `--assignee` is set), comments, attachments, worklogs.
- `--move` deletes the original issue after cloning.

Examples:
- "clone PROJ-123" → `jirac issue clone PROJ-123`
- "clone PROJ-123 to project NEWPROJ" → `jirac issue clone PROJ-123 --project NEWPROJ`
- "clone PROJ-123 with summary 'Copy: login bug'" → `jirac issue clone PROJ-123 --summary 'Copy: login bug'`
- "move PROJ-123 to OTHER project" → `jirac issue clone PROJ-123 --project OTHER --move`
