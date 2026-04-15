---
description: Build and run a JQL query interactively — filter issues by project, status, assignee, and more
---

Build and execute a JQL query using the interactive JQL builder in the `jira` CLI.

Steps:
1. Check if `jira` binary is available by running `jira --version`. If not found, tell the user to install it with `cargo install jira-commands`.
2. If the user already has a JQL expression, run it directly: `jira issue list --jql '<expression>'`
3. If the user wants help building a JQL query, launch the interactive builder: `jira issue jql`
   - The CLI will prompt step-by-step: project → status → assignee
   - Tell the user the interactive prompts will appear in their terminal
4. Display the resulting issues from the CLI output.

Examples:
- "run JQL: project = PROJ AND status = 'In Progress'" → `jira issue list --jql 'project = PROJ AND status = "In Progress"'`
- "help me build a JQL query" → `jira issue jql`
- "find all bugs assigned to me in PROJ" → `jira issue list --jql 'project = PROJ AND issuetype = Bug AND assignee = currentUser()'`
