---
description: Create a new Jira issue — interactive prompts for project, type, summary, and custom fields
---

Create a new Jira issue using the `jirac` CLI.

Steps:
1. Check if `jirac` binary is available by running `jirac --version`. If not found, tell the user to install it with `cargo install jira-commands` (binary: `jirac`).
2. Extract from the user's request:
   - Project key (required) — ask if not mentioned
   - Issue type (optional, e.g. Story, Bug, Task) — pass as `--type <TYPE>` if mentioned
   - Summary (optional) — pass as `--summary '<text>'` if mentioned
3. Run: `jirac issue create -p <PROJECT> [--type <TYPE>] [--summary '<SUMMARY>']`
4. The CLI will interactively prompt for remaining fields. Tell the user the interactive prompts will appear in their terminal.
5. After creation, show the new issue key returned by the CLI.

Examples:
- "create an issue in PROJ" → `jirac issue create -p PROJ`
- "create a bug in PROJ called login page crashes" → `jirac issue create -p PROJ --type Bug --summary 'login page crashes'`
- "buat task baru di PROJ" → `jirac issue create -p PROJ --type Task`
