# ClawHub publish notes for jira-commands

This repository currently ships Claude Code marketplace metadata at `.claude-plugin/marketplace.json`, plugin source metadata at `plugin/.claude-plugin/plugin.json`, and a dedicated ClawHub wrapper surface at `clawhub/jirac-plugin/`.

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
- `clawhub/jirac-plugin/marketplace.json`
- `plugin/skills/*/SKILL.md`
- CI plugin metadata check in `.github/workflows/ci.yml`

Suggested manual dry-run once `clawhub` CLI is installed and `CLAWHUB_TOKEN` is configured:

```bash
clawhub package publish clawhub/jirac-plugin/marketplace.json --dry-run
```

If ClawHub expects a specific source ref, check out that tag locally first, then dry-run the wrapper surface from disk:

```bash
git checkout v0.14.0
clawhub package publish clawhub/jirac-plugin/marketplace.json --dry-run
```

If the dry-run succeeds, publish for real:

```bash
clawhub package publish clawhub/jirac-plugin/marketplace.json
```

Notes:

- The repo currently does not have the `clawhub` CLI available in this environment.
- `CLAWHUB_TOKEN` can stay configured now even if publish is done later.
- Keep marketplace/plugin versions aligned with release-please bumps.
