# Install jirac

Choose the installer that fits your environment. Prefer package managers or verified release archives before using installer scripts.

## Recommended options

### Homebrew (macOS / Linux)

```bash
brew tap mulhamna/tap
brew install jira-commands

# Optional MCP server
brew install jira-mcp
```

### Cargo

```bash
cargo install jira-commands
```

### GitHub Releases

Download a prebuilt archive or binary from:

- https://github.com/mulhamna/jira-commands/releases

Prefer the packaged archives when available.

Common release artifacts include:

- `jirac-macos-aarch64.tar.gz`
- `jirac-macos-x86_64.tar.gz`
- `jirac-linux-aarch64.tar.gz`
- `jirac-linux-x86_64.tar.gz`
- `jirac-windows-x86_64.zip`

After extracting, place `jirac` on your `PATH`.

## Additional note

Project-provided installer scripts also exist in the repository for users who prefer to inspect them manually, but the recommended ClawHub install paths are Homebrew, Cargo, or a verified GitHub Releases download.

## Verify install

```bash
jirac --version
jirac auth login
jirac auth status
```
