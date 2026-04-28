---
description: Create multiple Jira issues at once from a JSON manifest file using jirac bulk-create
---

Create multiple Jira issues in bulk using `jirac`.

Steps:
1. Check that `jirac` is available by running `jirac --version`. If it is missing, tell the user to install it with `cargo install jira-commands`.
2. Build a JSON manifest array from the user's request. Each object supports:
   - `project` (required), `summary` (required)
   - `type` (default: "Task")
   - `assignee` (email or "me"), `priority`, `labels` (array), `components` (array)
   - `parent` (issue key), `fix_versions` (array), `description` (Markdown string)
   - `fields` (object of custom field IDs → values, e.g. `{"customfield_10016": 5}`)
3. Save the manifest to a temp file (e.g. `/tmp/issues.json`).
4. Run `jirac issue bulk-create --manifest /tmp/issues.json`.
5. Confirm each created issue key and summary.

Manifest format:
```json
[
  {
    "project": "PROJ",
    "summary": "Issue title",
    "type": "Task",
    "assignee": "user@org.com",
    "priority": "High",
    "labels": ["backend"],
    "components": ["auth"],
    "parent": "PROJ-100",
    "fix_versions": ["v1.0"],
    "description": "Markdown description",
    "fields": { "customfield_10016": 5 }
  }
]
```

Examples:
- "create 3 tasks in PROJ: setup DB, setup API, setup UI" → build manifest with 3 Task entries, run `jirac issue bulk-create --manifest /tmp/issues.json`
- "buat bulk issue dari daftar ini" → build manifest from the list, run `jirac issue bulk-create --manifest /tmp/issues.json`
