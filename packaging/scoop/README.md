# Scoop packaging

This directory documents the Scoop manifest source of truth for `jirac`.

## Install

```powershell
scoop bucket add mulhamna https://github.com/mulhamna/scoop-bucket
scoop install mulhamna/jirac
```

## Release flow

1. Publish the GitHub release and checksums.
2. Release CI updates the `jirac.json` manifest in `mulhamna/scoop-bucket`.
3. Scoop users can install or upgrade with `scoop install mulhamna/jirac` or `scoop update jirac`.

The bucket repo is external by design, so this directory is docs-only and the automation lives in the release workflow.
