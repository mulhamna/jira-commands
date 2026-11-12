---
name: sprint
description: "Manage Jira sprint lifecycle: list, create, start, complete, update, and delete sprints."
---

Use `jirac --version` to check availability; if missing, tell the user to install `jira-commands`.

- List: `jirac issue sprints -p <PROJECT>`
- Create: `jirac issue sprint-create -p <PROJECT> --name '<NAME>' [--board-id <ID>] [--goal '<TEXT>'] [--start-date YYYY-MM-DD] [--end-date YYYY-MM-DD]`
- Start: `jirac issue sprint-start -p <PROJECT> --sprint '<ID|NAME>' --end-date YYYY-MM-DD`
- Complete: `jirac issue sprint-complete -p <PROJECT> --sprint '<ID|NAME>'`
- Update: `jirac issue sprint-update -p <PROJECT> --sprint '<ID|NAME>' [--name '<NAME>'] [--goal '<TEXT>'] [--start-date YYYY-MM-DD] [--end-date YYYY-MM-DD]`
- Delete: `jirac issue sprint-delete -p <PROJECT> --sprint '<ID|NAME>' [--force]`

List sprints first when the target is unclear; use a configured default project
if appropriate. Creation needs `--board-id` when the project maps to multiple
boards. Confirm before start, complete, or delete: these affect the team, and
deletion is irreversible. Use `--force` only after confirmation when the
delete prompt cannot be answered. A board can have only one active sprint;
move unfinished issues as appropriate before completing. For a read-only
snapshot use `jirac issue sprint-summary`.
