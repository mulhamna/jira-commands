# MCP roadmap

Status note: `jira-mcp` is already useful, but it is not yet at CLI parity.
Current server focus is **typed tools over MCP**; it does **not** currently expose MCP prompts, resources, or UI surfaces.

## Current MCP capability snapshot

Implemented today:
- auth status / set credentials / logout
- issue list / view / create / update / delete / clone / transition
- issue type, field, transition, and issue-link-type discovery
- issue link create / delete
- bulk create
- typed batch runner (create / update / transition / archive)
- attachment upload
- comment list / add
- worklog list / add / delete
- bulk transition / bulk update / archive
- plan list
- raw Jira REST request
- stdio transport
- streamable HTTP transport
- install helpers for Claude Code, Claude Desktop, Cursor, Gemini CLI, Codex, OpenCode, generic JSON, and Zed

Not yet at CLI parity:
- sprint lifecycle tools
- standup / sprint summary helpers
- fix version browsing / create / update flows
- richer plan operations beyond `plan list`
- TUI-only workflows that need MCP-friendly equivalents
- prompts/resources/UI surfaces

## Parity principle

`jira-mcp` should aim for **workflow parity**, not 1:1 command-name parity.
That means every high-value CLI workflow should have an MCP-safe equivalent, even if the final tool shape differs from the CLI syntax.

## Phase 1 — Foundation

Done:
- shared auth/config reuse from `jirac`
- typed MCP server bootstrap on rmcp
- core issue CRUD surface
- field/transition discovery
- attachment and worklog basics
- raw API escape hatch

## Phase 2 — Agent-safe daily use

Done:
- bulk transition / update / archive with explicit confirmation
- comment add/list
- plans list
- multi-client install helpers
- streamable HTTP transport

## Phase 3 — CLI parity push

Goal: close the highest-value capability gaps so an MCP client can handle the same day-to-day Jira workflows as the CLI without falling back to shelling out.

Priority 1 is now complete on this branch.

Priority 2:
- sprint tools:
  - `jira_issue_sprints`
  - `jira_issue_sprint_create`
  - `jira_issue_sprint_start`
  - `jira_issue_sprint_update`
  - `jira_issue_sprint_complete`
  - `jira_issue_sprint_delete`
- reporting helpers:
  - `jira_issue_standup`
  - `jira_issue_sprint_summary`

Priority 3:
- fix-version tools:
  - list / preview backlog
  - create
  - rename
  - set description
  - set start date
  - set release date
  - released / archived toggles
- plan surface expansion if Jira API coverage is reliable

Guardrails:
- destructive or bulk-write tools must require `confirm: true`
- tools should prefer typed structured args over shell-like free-form text
- when a CLI flow is interactive today, MCP should expose the underlying discrete operations instead of emulating prompts
- raw API remains available, but parity should reduce the need for it

## Release/versioning direction

`jira-core` and `jira-commands` can stay in the same release lane.
`jira-mcp` should move to its own version lane because its user-facing capability surface can change independently.

Recommended target model:
- lane A: `jira-core` + `jira-commands`
- lane B: `jira-mcp`

Why this split is reasonable:
- CLI/core changes are tightly coupled already
- MCP capability growth will often happen without a meaningful CLI release
- separate release cadence avoids artificial version bumps on the MCP package
- changelog signal becomes cleaner for MCP users and client integrators

## Release split design sketch

Desired behavior:
- CLI lane keeps the existing root release flow
- MCP lane gets its own release-please component, manifest entry, changelog path, and tag prefix
- MCP publish/build jobs run only when the MCP lane releases
- npm wrapper `@mulham28/jirac-mcp` follows the MCP lane, not the root `VERSION`
- Homebrew/Scoop/Winget/Chocolatey mapping stays explicit per artifact

Expected repo changes for that split:
- stop treating `crates/jira-mcp/Cargo.toml` as part of the root version lane
- add a dedicated MCP version source (for example `crates/jira-mcp/VERSION` or a package-local release-please manifest entry)
- split `scripts/sync-npm-version.mjs` into per-lane sync logic
- update CI checks that currently hard-fail when all Rust crate versions differ
- update release workflows that currently validate one shared `VERSION`
- add separate tag handling for MCP releases
- ensure npm publish and release archives read the correct lane version

## Suggested implementation order

1. land the roadmap + parity checklist
2. implement missing Phase 3 MCP tools in small batches
3. add tests for every new tool surface
4. after parity work starts landing, split the MCP release lane
5. finally update packaging/docs/install helpers to reflect the dedicated MCP version stream

## Notes for implementation

The release-lane split is possible, but it touches:
- `Cargo.toml`
- `release-please-config.json`
- `.release-please-manifest.json`
- `VERSION`
- `scripts/sync-npm-version.mjs`
- `.github/workflows/ci.yml`
- `.github/workflows/release-please.yml`
- `.github/workflows/release-tag.yml`
- `.github/workflows/release-recover.yml`
- npm packaging checks and publish steps

So it should be done as a focused release-infra change, not mixed casually into feature work.
