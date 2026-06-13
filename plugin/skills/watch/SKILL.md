---
description: Manage Jira issue watchers with jirac, including adding the current user as a watcher, listing watchers, and removing a watcher by accountId
---

Manage Jira watchers using `jirac`.

Steps:
1. Check that `jirac` is available by running `jirac --version`. If it is missing, tell the user to install it with `cargo install jira-commands`.
2. Determine whether the user wants to add a watcher, list watchers, or remove a watcher.

For adding the current user as a watcher:
- extract the issue key
- run `jirac issue watch <ISSUE-KEY> add`

For adding a specific user as a watcher:
- extract the issue key
- extract the target user's Atlassian accountId
- run `jirac issue watch <ISSUE-KEY> add --account-id <ACCOUNT-ID>`

For listing watchers:
- extract the issue key
- run `jirac issue watch <ISSUE-KEY> list`

For removing a watcher:
- extract the issue key
- extract the target user's Atlassian accountId
- run `jirac issue watch <ISSUE-KEY> rm <ACCOUNT-ID>`
- the command prompts for confirmation; pass `--force` to skip the prompt when the operator already confirmed
