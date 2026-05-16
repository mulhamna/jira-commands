---
description: Add the same Markdown comment to multiple Jira issues with jirac, either by JQL query or explicit issue keys
---

Bulk comment on Jira issues using `jirac`.

Steps:
1. Check that `jirac` is available by running `jirac --version`. If it is missing, tell the user to install it with `cargo install jira-commands`.
2. Extract from the user's request:
   - a JQL filter, or an explicit issue-key list
   - the Markdown comment body
   - whether to skip confirmation
3. Run one of:
   - `jirac issue bulk-comment --jql '<JQL>' --body '<COMMENT>' [--force]`
   - `jirac issue bulk-comment --keys PROJ-1 PROJ-2 --body '<COMMENT>' [--force]`
   - use `--file <PATH>` instead of `--body` when the comment lives in a file
4. Confirm how many issues were updated and report any failures.

Examples:
- "comment on all in-progress PROJ issues that QA started verification" → `jirac issue bulk-comment --jql 'project = PROJ AND status = "In Progress"' --body 'QA started verification'`
- "post this note on PROJ-1 and PROJ-2" → `jirac issue bulk-comment --keys PROJ-1 PROJ-2 --body 'Please add your update before standup'`
