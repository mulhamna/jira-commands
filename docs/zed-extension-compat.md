# Zed extension compatibility checklist

Use this checklist whenever `jira-commands`, `jira-mcp`, release workflows, or the Zed wrapper change.

## Core contract

`jirac-ext` is only a thin Zed wrapper. The real runtime comes from `jirac-mcp` release assets published by `mulhamna/jira-commands`.

That means the Zed experience depends on these assumptions staying true:

1. `crates/zed-jira/src/lib.rs` still downloads from `mulhamna/jira-commands`
2. the wrapper still launches `serve --transport stdio`
3. the wrapper's settings/env mapping still matches `jira-mcp`
4. release workflows still publish the asset names the wrapper expects
5. wrapper changes are synced into `github.com/mulhamna/jirac-ext`

## When changing `jira-mcp`

Check these before merging:

- [ ] `serve --transport stdio` still works
- [ ] env vars still match: `JIRA_URL`, `JIRA_EMAIL`, `JIRA_TOKEN`, `JIRA_PROJECT`
- [ ] release asset names still match the wrapper expectations
- [ ] the dedicated `jira-mcp-v*` release still includes every required `jirac-mcp-*` artifact

## When changing `crates/zed-jira`

- [ ] run `cargo build -p zed-jira --target wasm32-wasip1 --release`
- [ ] run `./scripts/sync-jirac-ext.sh`
- [ ] push the mirror update to `jirac-ext`
- [ ] if needed, bump the submodule pointer in `zed-industries/extensions`

## When changing release workflows

- [ ] keep `release-tag-mcp.yml` and `release-recover.yml` aligned with `asset_name_for_platform()`
- [ ] do not rename `jirac-mcp-*` assets without updating the wrapper
- [ ] confirm at least one real release path still publishes the expected artifacts

## User experience expectation

End users should not need to install `jira-commands` manually just to use the Zed extension.

Expected flow:

1. install the `Jira` extension in Zed
2. configure credentials or seed them with `jirac mcp install --client zed`
3. let the wrapper fetch the correct `jirac-mcp` binary from GitHub releases automatically

Manual local CLI installation is optional for development, debugging, or power-user overrides.
