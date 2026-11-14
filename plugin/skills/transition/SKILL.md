---
description: Transition Jira issues to a new workflow state with jirac, either directly or through the interactive transition picker
---

Transition a Jira issue using `jirac`.

Steps:
1. Check that `jirac` is available by running `jirac --version`. If it is missing, tell the user to install it with `cargo install jira-commands`.
2. Extract from the user's request:
   - issue key
   - target status when provided
3. Run:
   - `jirac issue transition <ISSUE-KEY> '<STATUS>'` when the target status is known
   - `jirac issue transition <ISSUE-KEY>` when the user wants the interactive picker
4. Confirm the transition result clearly.

Examples:
- "transition PROJ-123 to In Progress" → `jirac issue transition PROJ-123 'In Progress'`
- "mark PROJ-456 as done" → `jirac issue transition PROJ-456 'Done'`
- "move PROJ-789 to review" → `jirac issue transition PROJ-789 'In Review'`
