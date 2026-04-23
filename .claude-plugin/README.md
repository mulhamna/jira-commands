# ClawHub publish notes for jira-commands

This repository currently ships a Claude Code plugin marketplace manifest at `.claude-plugin/marketplace.json` and plugin metadata at `plugin/.claude-plugin/plugin.json`.

Recommended ClawHub publishing strategy:

- Publish the **plugin package** discovery surface, not the standalone CLI binary.
- Keep the main user installation paths as:
  - Homebrew
  - Winget
  - install scripts
  - GitHub Releases
  - Cargo
- Position ClawHub as the agent/plugin integration entrypoint.

Suggested package positioning:

- Name: `jira-commands`
- Plugin: `jira`
- Category: `productivity`
- Summary: Jira issue management for Claude Code powered by the `jirac` CLI.

Current publish prerequisites already in repo:

- `plugin/.claude-plugin/plugin.json`
- `.claude-plugin/marketplace.json`
- `plugin/skills/*/SKILL.md`
- CI plugin metadata check in `.github/workflows/ci.yml`

Suggested manual dry-run once `clawhub` CLI is installed and `CLAWHUB_TOKEN` is configured:

```bash
clawhub package publish https://github.com/mulhamna/jira-commands --dry-run
```

If ClawHub expects a specific source ref, prefer publishing a release tag:

```bash
clawhub package publish https://github.com/mulhamna/jira-commands@v0.14.0 --dry-run
```

If the dry-run succeeds, publish for real:

```bash
clawhub package publish https://github.com/mulhamna/jira-commands@v0.14.0
```

Notes:

- The repo currently does not have the `clawhub` CLI available in this environment.
- `CLAWHUB_TOKEN` can stay configured now even if publish is done later.
- Keep marketplace/plugin versions aligned with release-please bumps.
