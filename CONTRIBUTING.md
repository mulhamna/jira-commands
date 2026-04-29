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

Useful commands (via `make` — see `make help` for full list):

```bash
make fmt              # cargo fmt --all
make lint             # cargo clippy ... -D warnings
make check            # cargo check --workspace
make test             # all crate tests
make audit            # cargo audit
make smoke            # fmt-check + lint + test + build (CI gate)
```

Or call cargo directly:

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
make run ARGS='issue list'
make run-tui P=PROJ
# or
cargo run -p jira-commands -- issue list
cargo run -p jira-commands -- tui -p PROJ
```

## Testing approach

We want a pragmatic TDD workflow for the Rust crates under `crates/`.

Baseline expectation:
- for bug fixes, add or update a failing test first when feasible, then fix the code
- for new crate behavior, add tests that lock the expected behavior before or alongside the implementation
- for refactors in `crates/`, keep existing tests green and add coverage if behavior could regress
- if a crate change ships without a test, explain why in the PR

This is intentionally practical, not dogmatic:
- docs, packaging, release metadata, and other non-crate changes do not need forced TDD ceremony
- tiny wiring changes may reuse existing test coverage when that is honestly sufficient
- we do not require 100% coverage

Priority for new tests:
- `crates/jira-core/`: parsing, auth/config behavior, request construction, response handling, regression coverage
- `crates/jira/`: CLI command behavior, argument validation, output shaping, version/update logic
- `crates/jira-mcp/`: tool contract behavior, request routing, error mapping, regression coverage

Before opening a PR with crate changes, run:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
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
