---
description: Scan recent Jira @mention notifications from issue descriptions and comments using jirac
---

Scan recent Jira `@mention` events targeting the current user from issue descriptions and comments using `jirac`.

Steps:
1. Check that `jirac` is available by running `jirac --version`. If missing, tell the user to install it (`cargo install jira-commands`).
2. Run `jirac issue notifications` to list recent mention events.
3. For each mention, the output shows the issue key, the author, and the source (description vs comment).
4. To inspect a specific mention in detail, follow up with `jirac issue view <KEY>`.

Notes:
- This scans **Jira-level @mentions** in description/comment ADF — not Jira's mobile/desktop notification feed.
- Mentions are detected by matching the current user's account ID against the ADF mention nodes.
- For a regular triage / standup flow, pair this with `jirac issue standup`.

Examples:
- "show my recent Jira mentions" → `jirac issue notifications`
- "did anyone mention me this week" → `jirac issue notifications`
