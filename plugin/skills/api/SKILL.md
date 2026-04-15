---
description: Execute a raw Jira REST API call — GET, POST, PUT, DELETE, or PATCH any endpoint
---

Execute a raw Jira REST API call using the `jira` CLI passthrough.

Steps:
1. Check if `jira` binary is available by running `jira --version`. If not found, tell the user to install it with `cargo install jira-commands`.
2. Extract from the user's request:
   - HTTP method (GET, POST, PUT, DELETE, PATCH)
   - API path (e.g. `/rest/api/3/issue/PROJ-123`)
   - JSON body (optional, for POST/PUT/PATCH)
3. Run the appropriate command:
   - GET: `jira api get <PATH>`
   - POST: `jira api post <PATH> --body '<JSON>'`
   - PUT: `jira api put <PATH> --body '<JSON>'`
   - DELETE: `jira api delete <PATH>`
   - PATCH: `jira api patch <PATH> --body '<JSON>'`
4. Display the pretty-printed JSON response.
5. If the user doesn't know the exact endpoint, help them find it based on what they want to do.

Examples:
- "get server info" → `jira api get /rest/api/3/serverInfo`
- "get issue PROJ-123" → `jira api get /rest/api/3/issue/PROJ-123`
- "get all projects" → `jira api get /rest/api/3/project`
- "post to /rest/api/3/issue with body {...}" → `jira api post /rest/api/3/issue --body '{...}'`
