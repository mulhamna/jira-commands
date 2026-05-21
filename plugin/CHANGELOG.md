# Changelog

All notable changes to the Claude Code plugin surface for jira-commands will be documented in this file.

## [0.17.0] - 2026-05-21

### Added
- New skills covering the full current CLI surface: `/jira:change-type`, `/jira:move-issue`, `/jira:link`, `/jira:archive`, `/jira:notifications`, `/jira:versions`, `/jira:render`, `/jira:sprint-lifecycle`.

### Changed
- Expand the install hint in `plugin/README.md` to cover source install (`cargo install --path crates/jira --locked`), Homebrew, and npm options.

## [0.16.0] - 2026-05-15

### Added
- Add Claude Code skills for daily standup and sprint summary workflows.

### Changed
- Document the OpenCode MCP install helper alongside the existing MCP client targets.

## [0.15.0] - 2026-04-30

### Changed
- Sync the Claude Code plugin release lane metadata with plugin version 0.15.0.

## [0.14.0] - 2026-04-23

### Added
- Track the Claude Code plugin as its own release lane with dedicated VERSION and CHANGELOG files.

### Changed
- Clarify that the Claude Code plugin release lifecycle is separate from the CLI/MCP workspace release lane.
