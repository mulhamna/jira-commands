# jirac ClawHub package wrapper

This directory is the **ClawHub-only package surface** for the `jirac` Claude Code plugin.

- `plugin/` remains the source of truth for the Claude Code plugin bundle.
- `clawhub/jirac-plugin/` stores wrapper metadata used to publish that bundle through ClawHub.
- The wrapper points back to `../../plugin` as the actual plugin source.

## Files

- `marketplace.json` — ClawHub package metadata and plugin source mapping
- `VERSION` — wrapper package version (kept aligned with the published plugin version)
- `CHANGELOG.md` — wrapper package release notes
