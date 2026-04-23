# Changelog

All notable changes to this project will be documented in this file.

Format: [Semantic Versioning](https://semver.org/) — `MAJOR.MINOR.PATCH`

---

## [0.15.1](https://github.com/mulhamna/jira-commands/compare/v0.15.0...v0.15.1) (2026-04-23)


### Bug Fixes

* **release:** add manual tag recovery path ([#61](https://github.com/mulhamna/jira-commands/issues/61)) ([802a5ab](https://github.com/mulhamna/jira-commands/commit/802a5abaafaefb3cc08be6e6c50a5fff0188753a))

## [0.15.0](https://github.com/mulhamna/jira-commands/compare/v0.14.0...v0.15.0) (2026-04-23)


### Features

* improve TUI pickers and split plugin release lanes ([#59](https://github.com/mulhamna/jira-commands/issues/59)) ([edc8c27](https://github.com/mulhamna/jira-commands/commit/edc8c2706b151b6b0fae26bfd9f49914f270916f))

## [0.14.0](https://github.com/mulhamna/jira-commands/compare/v0.13.0...v0.14.0) (2026-04-21)


### Features

* add started date/time prompts to TUI worklog ([af561a8](https://github.com/mulhamna/jira-commands/commit/af561a8ee98ffc490425313fd8de33ce1227ee40))
* add started timestamp options for worklog CLI ([e51ce98](https://github.com/mulhamna/jira-commands/commit/e51ce986ebbf29deec9e46284a5922b33bdc1654))

## [0.13.0](https://github.com/mulhamna/jira-commands/compare/v0.12.1...v0.13.0) (2026-04-21)


### Features

* **tui:** add saved columns and searchable assignee picker ([a865c0d](https://github.com/mulhamna/jira-commands/commit/a865c0d6a59e105f1bad5bfa036b8aa66c3fabc8))

## [0.12.1](https://github.com/mulhamna/jira-commands/compare/v0.12.0...v0.12.1) (2026-04-21)


### Bug Fixes

* **release:** clarify shipped binaries in release docs ([5683204](https://github.com/mulhamna/jira-commands/commit/5683204ba88fd646b75b06856ae1df17180db7bf))

## [0.12.0](https://github.com/mulhamna/jira-commands/compare/v0.11.0...v0.12.0) (2026-04-21)


### Features

* add Jira issue comment support ([511ec27](https://github.com/mulhamna/jira-commands/commit/511ec279591d592ef628de299e24396c20db718d))


### Bug Fixes

* align adf table tests and jirac migration docs ([f395763](https://github.com/mulhamna/jira-commands/commit/f3957632811049cc3e427195ca67fd3f14661390))
* avoid unsupported comrak text_contents helper ([a09c313](https://github.com/mulhamna/jira-commands/commit/a09c31337dfb3001e0fa04211241f68d1ad837e2))
* match rustfmt output for comment model ([daf43db](https://github.com/mulhamna/jira-commands/commit/daf43dbefe0d31d905a258cf9cdbc2f0d0bcc0f1))

## [0.11.0](https://github.com/mulhamna/jira-commands/compare/v0.10.0...v0.11.0) (2026-04-20)


### Features

* add table ADF support and plugin skill coverage ([1999948](https://github.com/mulhamna/jira-commands/commit/1999948cab85bb85ea6dc91852cf2b6cca8ef71a))


### Bug Fixes

* align ADF table conversion with comrak ([63ae954](https://github.com/mulhamna/jira-commands/commit/63ae9548fa3fc6fe3b1d35fb156640458124bfc7))
* correct paths-filter predicate setting ([899a504](https://github.com/mulhamna/jira-commands/commit/899a504ae1a48d4050c4862c21cf47f6714e9277))
* keep ADF table rendering read-side only ([b0fc2cd](https://github.com/mulhamna/jira-commands/commit/b0fc2cde8215659a0d7bd7205800c0cf216faa5c))
* remove stray blank line in adf formatter output ([b2954bd](https://github.com/mulhamna/jira-commands/commit/b2954bd0a0e405f47908e93921d7adf2f68e04fb))
* repair CI action pin and tidy crate READMEs ([150a5f5](https://github.com/mulhamna/jira-commands/commit/150a5f51ac331478b92294315d4baf5f839e5dbd))

## [0.10.0](https://github.com/mulhamna/jira-commands/compare/v0.9.0...v0.10.0) (2026-04-20)


### Features

* add plausible analytics snippet for jirac docs ([9ee32cf](https://github.com/mulhamna/jira-commands/commit/9ee32cf132e71bc43335baa9001837652b3458ae))

## [0.9.0](https://github.com/mulhamna/jira-commands/compare/v0.8.1...v0.9.0) (2026-04-19)


### Features

* add plausible analytics snippet ([93b254d](https://github.com/mulhamna/jira-commands/commit/93b254dc55d003ebbba2a207a9df77ecdc32f8fa))

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
