---
description: Execute a raw Jira REST API call — GET, POST, PUT, DELETE, or PATCH any endpoint
---

Execute a raw Jira REST API call using the `jirac` CLI passthrough.

Steps:
1. Check if `jirac` binary is available by running `jirac --version`. If not found, tell the user to install it with `cargo install jira-commands` (binary: `jirac`).
2. Extract from the user's request:
   - HTTP method (GET, POST, PUT, DELETE, PATCH)
   - API path (e.g. `/rest/api/3/issue/PROJ-123`)
   - JSON body (optional, for POST/PUT/PATCH)
3. Run the appropriate command:
   - GET: `jirac api get <PATH>`
   - POST: `jirac api post <PATH> --body '<JSON>'`
   - PUT: `jirac api put <PATH> --body '<JSON>'`
   - DELETE: `jirac api delete <PATH>`
   - PATCH: `jirac api patch <PATH> --body '<JSON>'`
4. Display the pretty-printed JSON response.
5. If the user doesn't know the exact endpoint, help them find it based on what they want to do.

Examples:
- "get server info" → `jirac api get /rest/api/3/serverInfo`
- "get issue PROJ-123" → `jirac api get /rest/api/3/issue/PROJ-123`
- "get all projects" → `jirac api get /rest/api/3/project`
- "post to /rest/api/3/issue with body {...}" → `jirac api post /rest/api/3/issue --body '{...}'`
