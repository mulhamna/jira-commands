---
name: versions
description: Browse Jira project fix versions and their backlog, or manage version metadata.
---

Use `jirac --version` to check availability; if missing, tell the user to install `jira-commands`.

- List: `jirac issue versions -p <PROJECT>`
- Inspect backlog: `jirac issue versions -p <PROJECT> --version '<NAME>'`.
  Backlog preview defaults to 10 issues; use `--limit <N>` to change it.
- Create: `jirac issue versions -p <PROJECT> --create --version '<NAME>' [--description '<TEXT>']`
- Update: select `--version '<NAME>'` and use `--set-name`, `--description`,
  `--set-start-date`, `--set-release-date`, `--released`, `--unreleased`,
  `--archived`, or `--unarchived` as requested. Use the corresponding
  `--clear-description`, `--clear-start-date`, or `--clear-release-date` to
  clear values.

The configured project can replace `-p`. Before setting an issue's
`--fix-version`, verify the version name and state.
