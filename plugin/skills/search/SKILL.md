---
name: search
description: Search, list, and export Jira issues; use JQL, prepare standups, summarize sprints, find mentions, and list Jira Plans.
---

Use `jirac --version` to check availability; if missing, tell the user to install `jira-commands`.

## List and JQL

- `jirac issue list` uses the configured project when present; otherwise it
  defaults to issues assigned to the current user.
- `jirac issue list -p <PROJECT>` lists a project; `--jql '<JQL>'` uses a custom query.
- Configured `default_issue_limit` can cap results; `--all` bypasses that
  limit. The CLI's 500-page safeguard can still silently truncate results.
  Split broad queries rather than assuming the output is complete.
- `jirac issue jql` opens an interactive query builder. For non-interactive
  use, pass `--params '<JSON>'` or `--params @<FILE>`; `--run` executes it.
  Alternatively, compose JQL from the user's criteria.
- Export matching issues with
  `jirac issue export [-p <PROJECT>] [--jql '<JQL>'] [--all] [--limit <N>] [--format json|csv] [-o <PATH>]`.
  Without `--limit`, a configured `default_issue_limit` can cap export; use
  `--all` to bypass it. `--format` defaults to JSON and output goes to stdout
  unless `-o` is provided.

## Standup and sprint summary

- `jirac issue standup [-p <PROJECT> | --jql '<JQL>']` produces standup buckets.
- `jirac issue sprint-summary -p <PROJECT> [--sprint '<NAME>']` summarizes the
  named or current sprint.

## Jira Plans

`jirac plan list` lists Jira Plans (Advanced Roadmaps). This is a Jira Premium
feature and returns an error on Standard and Free tiers.

## Mentions

`jirac issue notifications [-p <PROJECT>] [--since <WINDOW>] [--limit <N>]`
scans recent issue descriptions and comments for mentions; it is not Jira's
notification feed. The default lookback is seven days, and the default scan
limit is 50 recently updated issues (maximum 100). Detection uses ADF mention
nodes, not mentions in API v2 wiki strings. State these limits rather than
claiming a complete inbox. Use `jirac issue view <KEY>` for details.
