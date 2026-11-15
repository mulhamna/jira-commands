# CLAUDE.md — Rust Jira CLI

## Repository safeguards

- Git reads are allowed; request explicit repo-owner approval before git writes, including commits, pushes, and history rewrites.
- Do not manually bump versions, edit generated changelogs, or create/push tags without an explicit repo-owner request. Do not publish crates manually.
- `jira-core` is a public library; preserve compatibility for external callers.
- When creating a pull request, use `.github/PULL_REQUEST_TEMPLATE.md`; preserve its sections and checklists and fill them accurately.
- When creating a bug report or feature request, follow the appropriate template in `.github/ISSUE_TEMPLATE/`.
- Report security vulnerabilities privately, not in public issues; follow `SECURITY.md`.

## Jira API

- Choose deployment-specific behavior by deployment type, not API version: Cloud can also use API v2. Use client methods for the configured Platform API and the separate Agile API.
- Cloud search uses `/search/jql` with cursor tokens, not `/search`; Data Center uses `POST /rest/api/2/search` with `startAt`. The client exposes both through `next_page_token`; bulk search loops have a 500-page safeguard.
- On Data Center, user search uses `username`, not `query`; download attachments using the `content` URL from attachment metadata, not `/attachment/content/{id}`. The client's Cloud bulk-move and Plans endpoints are unavailable on Data Center.
- Rich-text fields use ADF on API v3 and Jira wiki markup on API v2. Reuse existing conversions and preserve native content when it is unchanged.
- Do not use Cloud `/rest/api/3/fieldconfiguration*` endpoints; they were removed. Resolve custom fields at runtime, never by hardcoded `customfield_*` IDs.
- Handle HTTP 429 using `Retry-After`. For asynchronous operations, submit, poll, and wait for completion.
- Check `JiraClient::is_premium()` before using Cloud premium features such as the Plans API.

## Validation

Before reporting completion, run:

```bash
cargo fmt --all -- --check && \
cargo clippy --all-targets --all-features -- -D warnings && \
cargo test --all && \
cargo build --all
```

## Releases

- CLI/core and MCP have separate release-please lanes, configs, and manifests. Lane routing is path-based, not commit-scope-based; check `release-please-config.json`, `release-please-config-mcp.json`, and `.github/workflows/release-please.yml` before splitting release-sensitive changes.
- Use Conventional Commits. Release-please is dispatched manually for a selected lane; a push to `main` alone does not run it. Do not dispatch releases or merge release PRs without explicit approval.
- The CLI lane publishes `jira-core` before `jira-commands`; the MCP lane publishes separately and requires the corresponding `jira-core` version to exist on crates.io.
- MCP changes using new `jira-core` APIs must wait for a compatible core release and use a dependency version that includes those APIs.
- Changes under `plugin/` require coordinated updates to `plugin/VERSION`, `plugin/.claude-plugin/plugin.json`, both versions in `.claude-plugin/marketplace.json`, and `plugin/CHANGELOG.md`. They also trigger the ClawHub plugin wrapper publish on merge to `main`: update `clawhub/jirac-plugin/VERSION`, both versions in its `marketplace.json`, and its changelog before merging. Obtain explicit repo-owner approval for these manual version and changelog changes.
- Changes under `clawhub/jirac/` trigger its ClawHub publish on merge to `main`; update `clawhub/jirac/VERSION` with explicit repo-owner approval before merging.
- Changes under `clawhub/jirac-plugin/` also trigger its publish on merge to `main`; synchronize its `VERSION`, both versions in `marketplace.json`, and changelog with explicit repo-owner approval before merging.
- When changing skills, keep relevant user-facing documentation in sync.
