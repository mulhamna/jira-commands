---
description: Run mixed Jira operations (create, update, transition, archive) from a single JSON manifest file using jirac batch
---

Run mixed Jira operations in batch using `jirac`.

Steps:
1. Check that `jirac` is available by running `jirac --version`. If it is missing, tell the user to install it with `cargo install jira-commands`.
2. Build a JSON manifest array from the user's request. Each object needs an `"op"` field:
   - `"create"` — same fields as bulk-create manifest (`project`, `summary`, `type`, etc.)
   - `"update"` — `key` + fields to update (`summary`, `priority`, `assignee`, etc.)
   - `"transition"` — `key` + `to` (transition name or ID)
   - `"archive"` — `key`
3. Save the manifest to a temp file (e.g. `/tmp/ops.json`).
4. Run `jirac issue batch --manifest /tmp/ops.json`.
5. Report the per-op result summary.

Manifest format:
```json
[
  { "op": "create",     "project": "PROJ", "summary": "New task", "type": "Task" },
  { "op": "update",     "key": "PROJ-10",  "priority": "High", "assignee": "me" },
  { "op": "transition", "key": "PROJ-11",  "to": "Done" },
  { "op": "archive",    "key": "PROJ-12" }
]
```

Examples:
- "create a task and close two other issues in one go" → build batch manifest, run `jirac issue batch --manifest /tmp/ops.json`
- "run these mixed Jira ops from this list" → build manifest from the list, run `jirac issue batch --manifest /tmp/ops.json`
