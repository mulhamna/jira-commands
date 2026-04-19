# Changelog

All notable changes to this project will be documented in this file.

Format: [Semantic Versioning](https://semver.org/) — `MAJOR.MINOR.PATCH`

---

## [0.8.1](https://github.com/mulhamna/jira-commands/compare/v0.8.0...v0.8.1) (2026-04-19)


### Bug Fixes

* **ci:** broaden release-please package path from crates/jira to workspace root ([3e28357](https://github.com/mulhamna/jira-commands/commit/3e28357fe6e0618d76fc6d835fbee28c6278a931))
* **ci:** revert release-please package path and widen scope via include-paths ([02a7d05](https://github.com/mulhamna/jira-commands/commit/02a7d05b1159fcbf89bb87005f9859aa3f5b3af9))
* **ci:** rework release-please to scope whole repo via simple + VERSION ([87202a0](https://github.com/mulhamna/jira-commands/commit/87202a0ed383edbeb969126d3d0708af14c90098))
* **jira-mcp:** shorten keyword to fit crates.io 20-char limit ([437540e](https://github.com/mulhamna/jira-commands/commit/437540ebb16dec94c9d73d73c6647aea5da5cd0b))

## [0.8.0](https://github.com/mulhamna/jira-commands/compare/v0.7.0...v0.8.0) (2026-04-19)


### Features

* adjust release please fix ([90d3a2f](https://github.com/mulhamna/jira-commands/commit/90d3a2fe2f714b0511213123f30d6e8dac001376))
* release please config crates ([18f7646](https://github.com/mulhamna/jira-commands/commit/18f764624055e4d4bf1b35b0e09006cbf71966b1))
* stabilize release-please workspace publishing ([e1e0b56](https://github.com/mulhamna/jira-commands/commit/e1e0b561961f828f7ee162117eba3790f3ede772))

## [0.4.0] — 2026-04-16

### Fixed
- **204 No Content**: `PUT`/`PATCH`/`DELETE` responses that return 204 are now treated as
  success. Previously, the JSON parser would error on an empty body.
- **Raw API (`jira api`)**: helper no longer tries to parse JSON when the response body is
  empty. `jira api put/delete/patch` commands now succeed silently on 204.
- **Assignee `accountId`**: `create` and `update` flows now resolve assignee to the correct
  Jira Cloud `accountId` instead of the legacy `emailAddress` / `name` field.
  - Pass an email → automatically looked up via `/user/search`
  - Pass a raw accountId (no `@`) → used directly
  - Pass `"me"` → resolved to current user via `/myself`

### Added
- `JiraClient::get_myself()` — fetches the current authenticated user's `accountId`
  from `/rest/api/3/myself`. Useful for "assign to me" flows.
- Quiet mode for non-interactive environments: spinners and progress bars are now
  suppressed when stdout is not a TTY (e.g. cron jobs, CI scripts, piped output).

### Changed
- `JiraClient::raw_request` return type changed from `Result<Value>` to
  `Result<Option<Value>>`. `None` indicates a successful 204 No Content response.
- Version bumped to **0.4.0** across `jira-core`, `jira-commands`, and the Claude Code plugin.

---

## [0.3.0] — 2026-04-15

### Added
- Claude Code plugin (`plugin/`) with 9 skills: `list-issues`, `view-issue`, `create-issue`,
  `transition`, `worklog`, `bulk-transition`, `attach`, `jql`, `api`

---

## [0.2.0] — 2026-04-15

### Added
- Phase 3 & 4: worklog CRUD, bulk transition/update, archive, JQL builder, `jira api` raw
  passthrough, `jira plan list` (Jira Premium)

---

## [0.1.0] — 2026-04-15

### Added
- Phase 1 & 2: auth, config, HTTP client with cursor-based pagination, issue CRUD,
  dynamic field introspection, attachment upload, TUI (ratatui + crossterm)
