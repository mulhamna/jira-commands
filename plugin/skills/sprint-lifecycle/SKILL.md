---
description: Manage Jira sprint lifecycle with jirac — list, create, start, complete, update, or delete sprints for a project
---

Manage the full sprint lifecycle for a Jira scrum board using `jirac`.

Steps:
1. Check that `jirac` is available by running `jirac --version`. If missing, tell the user to install it (`cargo install jira-commands`).
2. Identify the operation:
   - **List**: `jirac issue sprints -p <PROJECT>`
   - **Create**: `jirac issue sprint-create -p <PROJECT> --name '<NAME>' [--board-id <ID>] [--goal '<TEXT>'] [--start-date YYYY-MM-DD] [--end-date YYYY-MM-DD]`
   - **Start**: `jirac issue sprint-start -p <PROJECT> --sprint <SPRINT_ID> --end-date YYYY-MM-DD`
   - **Complete**: `jirac issue sprint-complete -p <PROJECT> --sprint <SPRINT_ID>`
   - **Update**: `jirac issue sprint-update -p <PROJECT> --sprint <SPRINT_ID> [--name '<NAME>'] [--goal '<TEXT>'] [--start-date YYYY-MM-DD] [--end-date YYYY-MM-DD]`
   - **Delete**: `jirac issue sprint-delete -p <PROJECT> --sprint <SPRINT_ID>` (destructive — confirm intent)
3. Run `jirac issue sprints -p <PROJECT>` first when the user does not give a sprint ID — the list shows IDs and current state (future / active / closed).
4. Confirm intent before `sprint-start`, `sprint-complete`, and `sprint-delete` — these change visible board state for the whole team.

Notes:
- `--sprint` accepts a sprint name or numeric ID. Use `-p` to select the project, or rely on the configured default project. Starting a sprint also requires `--end-date`; obtain it from the user if missing.
- A board can only have **one active sprint** at a time. `sprint-start` will fail if another sprint is already active.
- `sprint-complete` closes the sprint; any unfinished issues should be moved to a future sprint **before** completing.
- `sprint-delete` permanently removes a sprint and detaches its issues from the sprint field — irreversible.
- For a snapshot of work in a sprint (without changing state), use `jirac issue sprint-summary -p <PROJECT> --sprint '<NAME>'` instead.

Examples:
- "list sprints in PROJ" → `jirac issue sprints -p PROJ`
- "create Sprint 25 in PROJ" → `jirac issue sprint-create -p PROJ --name 'Sprint 25' --goal 'Ship auth v2'`
- "start sprint 42 in PROJ until 2026-06-03" → `jirac issue sprint-start -p PROJ --sprint 42 --end-date 2026-06-03`
- "complete the current sprint in PROJ" → list sprints to find the active ID, then `jirac issue sprint-complete -p PROJ --sprint <ID>`
- "rename sprint 42 in PROJ to 'Sprint 25 (extended)'" → `jirac issue sprint-update -p PROJ --sprint 42 --name 'Sprint 25 (extended)'`
- "delete sprint 99 in PROJ" → `jirac issue sprint-delete -p PROJ --sprint 99` (after confirming)
