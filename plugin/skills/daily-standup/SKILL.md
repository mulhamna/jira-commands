---
description: Generate a markdown-ready daily standup summary from assigned Jira issues using jirac
---

Generate a standup summary with `jirac`.

Steps:
1. Check that `jirac` is available by running `jirac --version`.
2. If the user names a project, pass `-p / --project`.
3. If the user gives custom filtering, use `--jql` instead.
4. Run `jirac issue standup`.
5. Present the generated buckets clearly.

Examples:
- "generate my standup" → `jirac issue standup`
- "standup for PROJ" → `jirac issue standup -p PROJ`
- "standup from this query" → `jirac issue standup --jql 'assignee = currentUser() AND project = PROJ ORDER BY updated DESC'`
