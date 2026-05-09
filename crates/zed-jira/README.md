# zed-jira

Official Zed extension wrapper for `jirac-mcp`.

This crate is the implementation source-of-truth. The dedicated Zed-facing repository lives at <https://github.com/mulhamna/jirac-ext> and is kept as a publish/mirror repo for `zed-industries/extensions`.

This crate compiles to a Zed extension WASM module. It does not implement Jira logic itself; it downloads the matching `jirac-mcp` release artifact for the current platform, exposes Zed context-server settings, and launches the MCP server over stdio.

## Local smoke build

```bash
rustup target add wasm32-wasip1
cargo build -p zed-jira --target wasm32-wasip1 --release
```

## Settings

The extension maps these Zed settings into `jirac-mcp` environment variables:

- `jira_url` -> `JIRA_URL`
- `jira_email` -> `JIRA_EMAIL`
- `jira_token` -> `JIRA_TOKEN`
- `default_project` -> `JIRA_PROJECT`

You can seed the same settings from an existing `jirac` auth profile with:

```bash
jirac mcp install --client zed
```

To refresh the dedicated `jirac-ext` mirror repo from the repository root, run:

```bash
./scripts/sync-jirac-ext.sh
```
