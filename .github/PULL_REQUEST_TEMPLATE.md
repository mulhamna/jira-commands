## Description

<!-- What does this PR do? Why? Link related issues with "Fixes #123" if applicable. -->

## Type of change

- [ ] Bug fix
- [ ] New feature
- [ ] Refactor / cleanup
- [ ] Docs / chore
- [ ] Breaking change (bumps major version)

## Checklist

- [ ] `cargo fmt --all` passes locally
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo test --all` passes
- [ ] `cargo audit` passes (no new CVEs introduced)
- [ ] Crate changes include added or updated tests, or this PR explains why not
- [ ] For bug fixes in `crates/`, I added or updated a regression test when feasible
- [ ] No new dependency added without justification in PR description
- [ ] New dependencies use permissive licenses (MIT / Apache-2.0 / BSD)

## Security checklist (required for all PRs)

- [ ] No secrets, tokens, or credentials added to source code
- [ ] No new `unsafe` blocks without explanation
- [ ] No outbound network calls added outside of `jira-core/src/client.rs`
- [ ] No changes to `.github/workflows/` without explaining why in this PR
- [ ] If release or installer surfaces changed, docs/install notes were updated to match
- [ ] Dependencies sourced only from crates.io (no `git = "..."` deps without justification)

## Merge behavior

- [ ] Safe to auto-merge once required checks and approvals pass (add `automerge` label)
- [ ] Needs manual merge because of rollout, migration, release timing, or other risk

## Notes for reviewer

<!-- Anything the reviewer should pay special attention to? -->
