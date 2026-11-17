---
name: api
description: Use Jira raw REST API requests, inspect issue fields, and preview Markdown as ADF.
---

Use `jirac --version` to check availability; if missing, tell the user to install `jira-commands`.

## Raw REST API

Prefer a dedicated `jirac` subcommand when it supports the operation. For raw
platform API calls, check the active profile and API version with `jirac auth
status`; this may contact Jira to look up the current user. Keep the same
profile and `JIRA_PROFILE` environment setting for the request. If the version
cannot be determined, ask the user rather than assuming Cloud.

Use `/rest/api/3/...` for API v3 (normally Cloud) and `/rest/api/2/...` for
API v2 (normally Data Center or Server). Other API families, such as
`/rest/agile/1.0/...`, have their own versions. Raw requests do not convert
bodies: descriptions and comments generally use wiki-markup strings in v2 and
ADF objects in v3. Check the endpoint schema for its request and response.

Use `jirac api get <PATH>` or `jirac api delete <PATH>`. For a JSON body, use
`jirac api <METHOD> <PATH> --body '<JSON>'`, where METHOD is `post`, `put`, or
`patch`. Map an unknown endpoint before running a request. Examples:

- Cloud/v3: `jirac api get /rest/api/3/serverInfo`
- Data Center/v2: `jirac api get /rest/api/2/serverInfo`
- For an issue or project, use `/rest/api/2/issue/PROJ-123` or
  `/rest/api/2/project` on v2, and the corresponding `/rest/api/3/...` path on v3.

## Render Markdown

Use `jirac issue render --input <FILE>` or pipe text to `jirac issue render`
to preview Markdown as ADF. The preview does not resolve `@username` into a
Jira mention and is not a preview of the wiki markup sent to API v2. After
review, send Markdown via `--description-file` or comment `--file`.

## Inspect fields

Use `jirac issue fields -p <PROJECT> --issue-type '<TYPE>'` to discover field
IDs and types before create/update; add `--required-only` when needed. If the
issue type is omitted, the command may prompt interactively.
