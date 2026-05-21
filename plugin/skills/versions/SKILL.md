---
description: Browse Jira project fix versions, preview their backlog items, or update version metadata with jirac
---

Browse and manage Jira project fix versions using `jirac issue versions`.

Steps:
1. Check that `jirac` is available by running `jirac --version`. If missing, tell the user to install it (`cargo install jira-commands`).
2. Extract from the user's request:
   - project key (required when not using the default config project)
   - specific version name (optional — narrows view to one version + its backlog)
3. Run `jirac issue versions -p <PROJECT>` to list all versions, or `--version '<NAME>'` to focus on one.
4. Report each version's status (released, archived, unreleased), start/release dates, and backlog summary.

Notes:
- Use this **before bulk updates** that touch `--fix-versions` — confirms the version name exists and is in the expected state.
- The TUI (`jirac tui` then `V`) offers an interactive version browser with backlog preview, including `Enter` to refresh and `n` / `e` to create or edit metadata.
- For shipping prep, combine with JQL on `fixVersion in unreleasedVersions()` or `fixVersion = "v1.2.0"`.

Examples:
- "list versions in PROJ" → `jirac issue versions -p PROJ`
- "show me details for v1.2.0 in PROJ" → `jirac issue versions -p PROJ --version 'v1.2.0'`
- "what's planned for the next release of PROJ" → `jirac issue versions -p PROJ` then preview the unreleased one
