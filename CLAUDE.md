# CLAUDE.md — Rust Jira CLI

Guide for Claude (and contributors) when working in this repo.

---

## Claude rules — MUST follow

Claude may:
- Create and edit files on the filesystem
- Run `cargo` commands for build/test/check
- Run `git` commands for read and write operations when explicitly requested by the repo owner, including `git add`, `git commit`, and `git push`
- Do not manually bump versions, edit generated changelogs, create/push tags, or rewrite git history unless explicitly requested by the repo owner

### TASK.md — work checklist

`TASK.md` (gitignored) is Claude's work checklist.
1. Read it at the start of every new session
2. Update `[ ]` → `[x]` immediately after task is done and smoke test passes
3. If missing, recreate from conversation context or ask repo owner

---

## Project overview

Rust CLI for Atlassian Jira (`jirac` binary). Focus: full custom field via dynamic introspection, attachment upload, Jira REST API v3, interactive TUI (ratatui), single binary.

### Workspace structure

```
crates/
├── jira-core/    # PUBLIC LIBRARY: API client, auth, model, ADF parser (crates.io: "jira-core")
├── jira/         # BINARY: clap CLI + TUI (crates.io: "jira-commands", binary: jirac)
└── jira-mcp/     # MCP SERVER: typed Jira tools via rmcp (crates.io: "jira-mcp", binary: jirac-mcp)
plugin/
├── .claude-plugin/  # Claude Code plugin metadata (plugin.json)
└── skills/          # 12 skills: api, attach, bulk-transition, comment, create-issue, fields, jql, list-issues, transition, update-issue, view-issue, worklog
```

### Crate responsibilities

- **`jira-core`** — public API: `JiraClient`, model types, ADF parser, auth, error types. Can be used as a library dependency.
- **`jira/`** — clap commands, TUI (ratatui + crossterm), interactive prompts (inquire). Binary: `jirac`.
- **`jira-mcp/`** — MCP server via `rmcp`, exposes `jira-core` as MCP tools for LLM clients.

---

## Jira API — implementation rules

### Endpoint rules

| Use                                         | Do NOT use                                           |
| ------------------------------------------- | ---------------------------------------------------- |
| `GET/POST /rest/api/3/search/jql`           | `/rest/api/3/search` (dead since Oct 2025)           |
| `POST /rest/api/3/search/approximate-count` | `/rest/api/3/fieldconfiguration*` (removed Jul 2026) |
| `GET /rest/api/3/projects/fields`           |                                                      |
| `GET /rest/api/3/priorityscheme`            |                                                      |

### Implementation principles

- **Pagination**: cursor-based (`next_page_token`), not offset (`startAt`). Max 500 iterations as safeguard.
- **Rate limiting**: handle 429 with `Retry-After` header, retry after delay.
- **Field resolution**: always runtime via API, never hardcode `customfield_*`.
- **Async tasks**: submit → poll → complete pattern for heavy operations (archive, bulk ops).
- **Tier detection**: check `server_info.is_premium()` before using premium features (Plans API).
- **Base URL**: Platform API (`/rest/api/3`) vs Agile API (`/rest/agile/1.0`) — don't mix, use client methods.

---

## Smoke test

Claude runs this before reporting to repo owner. Fix until all green.

```bash
cargo fmt --all -- --check && \
cargo clippy --all-targets --all-features -- -D warnings && \
cargo test --all && \
cargo build --all
```

---

## Release flow — release-please (automated)

**Never manually bump versions, update CHANGELOG, or push tags.**

### How it works

1. Push commits to `main` with **Conventional Commits**
2. release-please creates/updates a Release PR (version bump + CHANGELOG)
3. Merge Release PR → pushes tag → `release.yml` triggers build + publish

### Conventional Commits

Format: `<type>(<scope>): <description>` — in English.

| Type                              | Bump       |
| --------------------------------- | ---------- |
| `feat:`                           | MINOR      |
| `fix:`, `perf:`, `refactor:`      | PATCH      |
| `feat!:` / `BREAKING CHANGE:`     | MAJOR      |
| `chore:`, `docs:`, `ci:`, `test:` | No release |

### crates.io publish order

`jira-core` → sparse index ready → `jira-mcp` → `jira-commands`. Never publish manually.

### CI workflows

See `.github/workflows/` for details — actual files are source of truth.
- **ci.yml**: fmt + clippy + test + build (matrix: ubuntu/macos/windows)
- **security.yml**: `cargo audit`
- **release-please.yml**: auto version bump + CHANGELOG + tag
- **release-tag.yml**: build binaries + publish to crates.io (trigger: tag `v*`)
- **release-recover.yml**: recovery workflow for failed releases
- **clawhub-publish-jirac.yml**: publish to ClawHub plugin marketplace
- **pr-automerge.yml**: auto-merge release-please PRs
- **winget-submit.yml**: submit to Windows Package Manager

### Release strategy

Uses `simple` release type with a `VERSION` file at repo root. Cargo.toml files updated via `generic` updater in `release-please-config.json`. Plugin version in `plugin/.claude-plugin/plugin.json` is separate and NOT auto-bumped by release-please.

### Plugin marketplace

When adding/changing skills, update `plugin/skills/<skill>/SKILL.md` and the table in README.
