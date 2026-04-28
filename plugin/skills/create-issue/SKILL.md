---
description: Create new Jira issues with jirac, including interactive prompts for project, issue type, summary, and custom fields
---

Create a new Jira issue using `jirac`.

Steps:
1. Check that `jirac` is available by running `jirac --version`. If it is missing, tell the user to install it with `cargo install jira-commands`.
2. Extract from the user's request:
   - project key
   - issue type
   - summary
   - any explicit assignee, labels, priority, parent, or other fields
3. Map the request to supported flags:
   - `-p / --project`, `-s / --summary`, `-t / --issue-type`
   - `--assignee` (email or "me"), `--priority`, `--labels`, `--components`
   - `--parent <ISSUE-KEY>` for sub-tasks
   - `--sprint <ID|NAME>` to assign to a sprint on create
   - `--fix-versions`, `--description-file <FILE>`
   - `--field <ID=VALUE>` for custom fields (repeatable)
   - `--no-custom-fields` to skip required custom field prompts
4. Run `jirac issue create` with the fields that are already known.
5. Let `jirac` prompt for any missing required fields.
6. If custom fields are unclear, run `jirac issue fields -p <PROJECT> --issue-type '<TYPE>'` first.
7. Confirm the created issue key clearly.

Examples:
- "create an issue in PROJ" → `jirac issue create -p PROJ`
- "create a bug in PROJ called login page crashes" → `jirac issue create -p PROJ -t Bug -s 'login page crashes'`
- "create sub-task under PROJ-100" → `jirac issue create -p PROJ -t Sub-task -s 'sub-task title' --parent PROJ-100`
- "create story in Sprint 24" → `jirac issue create -p PROJ -t Story -s 'story title' --sprint 'Sprint 24'`
- "create with story points 5" → `jirac issue create -p PROJ -t Story -s 'title' --field story_points=5`
- "buat task baru di PROJ" → `jirac issue create -p PROJ -t Task`
