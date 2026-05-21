---
description: Archive Jira issues matching a JQL query with jirac
---

Archive all issues matching a JQL query using `jirac`. Archive is destructive — issues are hidden from regular search and become read-only.

Steps:
1. Check that `jirac` is available by running `jirac --version`. If missing, tell the user to install it (`cargo install jira-commands`).
2. Extract or build a JQL query from the user's request that scopes the archive cleanly (project, status, age, resolution).
3. Dry-run the JQL via `jirac issue list --jql '<JQL>'` so the user can see the affected set before archiving.
4. Confirm intent explicitly — archive is irreversible without admin assistance.
5. Run `jirac issue archive --jql '<JQL>'` (omit `--force` to keep the confirmation prompt; add `--force` only after explicit confirmation in agent flows).
6. Report how many issues were archived.

Notes:
- Archive requires sufficient Jira permissions (typically Project Administrator).
- Archived issues remain in the database but are hidden from boards, dashboards, and standard JQL searches by default.
- Prefer scoping by `project`, `resolution`, `updated`, and `status` — broad queries can archive far more than expected.
- For non-destructive cleanup, consider `bulk-transition` to a "Won't Do" or "Closed" status first.

Examples:
- "archive done issues older than 6 months in PROJ" → `jirac issue archive --jql 'project = PROJ AND resolution = Done AND updated < -180d'`
- "archive all 'Won't Do' items in PROJ older than 90 days" → `jirac issue archive --jql 'project = PROJ AND status = "Won''t Do" AND updated < -90d'`
- "archive released old work" → `jirac issue archive --jql 'project = PROJ AND fixVersion in releasedVersions() AND updated < -365d'`
