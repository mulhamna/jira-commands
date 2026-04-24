# Contributing to jirac

Thanks for contributing to `jirac`.

This repository contains multiple delivery lanes that move together but are not identical:
- Rust workspace crates under `crates/`
- GitHub release automation under `.github/workflows/`
- packaging metadata under `packaging/winget/`
- Claude plugin assets under `plugin/` and `.claude-plugin/`
- ClawHub skill assets under `clawhub/jirac/`

## Local development

Prerequisites:
- Rust stable toolchain
- `cargo` and standard Rust development tooling
- a Jira Cloud account plus API token for real integration testing

Useful commands:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check --workspace --all-targets
cargo test -p jira-core
cargo test -p jira-commands
cargo test -p jira-mcp
cargo audit
```

Run the CLI locally:

```bash
cargo run -p jira-commands -- issue list
cargo run -p jira-commands -- tui -p PROJ
```

## Repo layout

- `crates/jira-core/`: shared Jira client and models
- `crates/jira/`: main `jirac` CLI and TUI
- `crates/jira-mcp/`: MCP server binary
- `.github/workflows/`: CI, security, release, and packaging automation
- `packaging/winget/`: in-repo Winget manifests
- `plugin/` and `.claude-plugin/`: Claude plugin assets and marketplace metadata
- `clawhub/jirac/`: ClawHub skill lane

## Change impact guide

When you change one area, check the related lanes too.

### Rust workspace changes

If you change code in `crates/`:
- run `fmt`, `clippy`, `check`, and relevant tests
- review README examples if user-facing behavior changed
- confirm release artifacts and install docs still match binary names

### Version or release changes

If you change versioning or release files:
- check `VERSION`
- check root `CHANGELOG.md`
- check `Cargo.toml` plus crate versions in `crates/*/Cargo.toml`
- check `release-please-config.json`
- check `.release-please-manifest.json`
- review `.github/workflows/release-please.yml`, `release-tag.yml`, and `release-recover.yml`

### Workflow changes

If you edit `.github/workflows/*.yml`:
- keep permissions least-privilege
- prefer pinned actions where practical
- verify whether the change affects PR checks, `main` pushes, tags, or manual recovery flows
- do not assume green PR CI proves tag or release paths are safe

### Winget packaging changes

If you update Windows packaging or release artifact naming:
- check `packaging/winget/jirac.yaml`
- check `packaging/winget/jirac.locale.en-US.yaml`
- check `packaging/winget/jirac.installer.yaml`
- check `.github/workflows/winget-submit.yml`
- confirm published asset names and checksums still match expectations

### Claude plugin changes

If you touch `plugin/` or `.claude-plugin/`:
- keep `plugin/VERSION` aligned with `plugin/.claude-plugin/plugin.json`
- keep `.claude-plugin/marketplace.json` version metadata in sync
- update `plugin/CHANGELOG.md` when versioned behavior changes

### ClawHub skill changes

If you update `clawhub/jirac/`:
- keep `clawhub/jirac/SKILL.md` metadata valid
- keep `clawhub/jirac/VERSION` current
- review `clawhub-publish-jirac.yml`
- update references/docs under the same skill lane if behavior changed

## Pull request guidance

Please prefer focused PRs with a clear theme:
- feature work
- workflow hardening
- release fixes
- docs-only improvements

If a PR touches release automation, call that out explicitly in the description and mention what downstream paths were considered.

## Release safety notes

Release-related workflows are intentionally conservative.
When changing them:
- preserve the current release semantics unless there is a clear reason to change behavior
- prefer the smallest fix that removes risk or noise
- verify the exact path you changed: PR, push to `main`, tag release, or manual recovery

## Documentation

If user-facing behavior changes, update the relevant docs in the same branch:
- `README.md`
- `plugin/README.md`
- `packaging/winget/README.md`
- `clawhub/jirac/references/`

That keeps release assets, docs, and automation from drifting apart.
