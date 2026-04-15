---
description: Upload a file attachment to a Jira issue
---

Upload a file attachment to a Jira issue using the `jira` CLI.

Steps:
1. Check if `jira` binary is available by running `jira --version`. If not found, tell the user to install it with `cargo install jira-commands`.
2. Extract from the user's request:
   - Issue key (e.g. PROJ-123) — ask if not mentioned
   - File path — ask if not mentioned, or help locate the file if the user is unsure of the path
3. Verify the file exists at the given path.
4. Run: `jira issue attach <ISSUE-KEY> <FILE-PATH>`
5. Confirm the upload was successful from the CLI output.

Examples:
- "attach screenshot.png to PROJ-123" → `jira issue attach PROJ-123 ./screenshot.png`
- "upload error.log to PROJ-456" → `jira issue attach PROJ-456 ./error.log`
