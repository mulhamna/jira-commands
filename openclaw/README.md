# jirac for OpenClaw

This directory is reserved for the OpenClaw / ClawHub integration surface for `jirac`.

It is intentionally separate from the Claude Code plugin under `plugin/` because the packaging model, installation flow, invocation style, and release lifecycle are different.

## Status

This lane is currently scaffolding only.

Planned responsibilities:

- OpenClaw-facing packaging and metadata
- ClawHub-oriented installation and discovery docs
- OpenClaw-specific examples and usage guidance
- Independent versioning and changelog tracking
- Independent CI/release automation once the artifact shape is finalized

## Current artifact shape

The first concrete artifact in this lane is a ClawHub skill-style surface:

- `openclaw/SKILL.md`
- `openclaw/VERSION`
- `openclaw/CHANGELOG.md`

This keeps the first OpenClaw-facing publish target simple while we decide whether a later native package/plugin is also needed.

## Boundaries

- `crates/` remain the shared CLI/MCP implementation lane
- `plugin/` remains the Claude Code plugin lane
- `openclaw/` is the future OpenClaw / ClawHub lane

Do not assume the Claude plugin bundle layout can be published directly as the OpenClaw / ClawHub artifact.
