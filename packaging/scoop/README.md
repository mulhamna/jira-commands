# Scoop packaging

This directory documents the Scoop manifest source of truth for `jirac` and `jirac-mcp`.

## Install

```powershell
scoop bucket add mulhamna https://github.com/mulhamna/scoop-bucket
scoop install mulhamna/jirac
scoop install mulhamna/jirac-mcp
```

## Release flow

1. Publish the GitHub release and checksums.
2. Release CI updates the relevant manifest in `mulhamna/scoop-bucket` (`jirac.json` or `jirac-mcp.json`).
3. Scoop users can install or upgrade with `scoop install mulhamna/jirac`, `scoop install mulhamna/jirac-mcp`, or the matching `scoop update` command.

The bucket repo is external by design, so this directory is docs-only and the automation lives in the release workflow.
