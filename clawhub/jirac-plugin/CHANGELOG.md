# Changelog

All notable changes to the ClawHub wrapper package for the `jirac` Claude plugin will be documented in this file.

## [0.1.1] - 2026-06-03

### Fixed
- Publish the ClawHub jirac plugin from a staged bundle package so ClawHub receives real plugin contents plus generated OpenClaw metadata.
- Skip re-publish on workflow-only pushes while still validating the release path.

## [0.1.0] - 2026-06-03

### Added
- Introduce a dedicated ClawHub wrapper surface under `clawhub/jirac-plugin/`.
- Keep the Claude Code plugin source in `plugin/` while publishing ClawHub through wrapper metadata.
