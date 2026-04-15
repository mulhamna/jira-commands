---
description: Transition a Jira issue to a new status (e.g. In Progress, Done, In Review)
---

Transition a Jira issue to a new status using the `jira` CLI.

Steps:
1. Check if `jira` binary is available by running `jira --version`. If not found, tell the user to install it with `cargo install jira-commands`.
2. Extract from the user's request:
   - Issue key (e.g. PROJ-123) — ask if not mentioned
   - Target status (e.g. "In Progress", "Done", "In Review") — if not mentioned, run without `--to` flag so CLI prompts interactively
3. Run: `jira issue transition <ISSUE-KEY> [--to '<STATUS>']`
4. Confirm the transition was successful from the CLI output.

Examples:
- "transition PROJ-123 to In Progress" → `jira issue transition PROJ-123 --to 'In Progress'`
- "mark PROJ-456 as done" → `jira issue transition PROJ-456 --to 'Done'`
- "move PROJ-789 to review" → `jira issue transition PROJ-789 --to 'In Review'`
- "transition PROJ-123" (no target) → `jira issue transition PROJ-123` (interactive)
