---
description: Summarize the current or named Jira sprint by status and assignee using jirac
---

Generate a sprint summary with `jirac`.

Steps:
1. Check that `jirac` is available by running `jirac --version`.
2. Determine the Jira project key.
3. If the user names a sprint, pass `--sprint`; otherwise summarize the open sprint.
4. Run `jirac issue sprint-summary -p <PROJECT>`.
5. Present the totals and grouped issues clearly.

Examples:
- "summarize the current sprint in PROJ" → `jirac issue sprint-summary -p PROJ`
- "summarize Sprint 24 in PROJ" → `jirac issue sprint-summary -p PROJ --sprint 'Sprint 24'`
