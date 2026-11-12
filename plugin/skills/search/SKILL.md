---
name: search
description: Search and list Jira issues with JQL, prepare standups, summarize sprints, and find mentions.
---

Use `jirac --version` to check availability; if missing, tell the user to install `jira-commands`.

## List and JQL

- `jirac issue list` defaults to issues assigned to the current user.
- `jirac issue list -p <PROJECT>` lists a project; `--jql '<JQL>'` uses a custom query.
- Configured `default_issue_limit` can cap results; `--all` bypasses that
  limit. The CLI's 500-page safeguard can still silently truncate results.
  Split broad queries rather than assuming the output is complete.
- `jirac issue jql` opens an interactive query builder; in non-interactive
  sessions, compose JQL from the user's criteria instead.

## Standup and sprint summary

- `jirac issue standup [-p <PROJECT> | --jql '<JQL>']` produces standup buckets.
- `jirac issue sprint-summary -p <PROJECT> [--sprint '<NAME>']` summarizes the
  named or current sprint.

## Mentions

`jirac issue notifications [-p <PROJECT>] [--since <WINDOW>] [--limit <N>]`
scans recent issue descriptions and comments for mentions; it is not Jira's
notification feed. The default lookback is seven days, and the default scan
limit is 50 recently updated issues (maximum 100). State these limits rather
than claiming a complete inbox. Use `jirac issue view <KEY>` for details.
