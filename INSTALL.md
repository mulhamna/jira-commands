# INSTALL

Detailed installation guide for `jirac` and `jirac-mcp`.

## Supported install paths

| Method               | macOS | Linux | Windows | Notes                                           |
| -------------------- | ----- | ----- | ------- | ----------------------------------------------- |
| Homebrew             | ✅    | ✅    | ❌      | `jira-commands` and `jira-mcp` via `mulhamna/tap` |
| Install script       | ✅    | ✅    | ❌      | Downloads the latest release asset for `jirac` or `jirac-mcp` |
| PowerShell installer | ❌    | ❌    | ✅      | Installs `jirac.exe` or `jirac-mcp.exe` to a user-local bin |
| Cargo                | ✅    | ✅    | ✅      | Best for Rust users; install `jira-commands` and/or `jira-mcp` |
| From source          | ✅    | ✅    | ✅      | `cargo install --path` from a local checkout    |
| npm                  | ✅    | ✅    | ✅      | Downloads the matching prebuilt release binary  |
| ClawHub              | ✅    | ✅    | ✅      | Install the `jirac` skill or the `jirac-plugin` bundle via OpenClaw |
| GitHub Releases      | ✅    | ✅    | ✅      | Manual download of CLI and MCP archives/binaries |
| Scoop                | ❌    | ❌    | ✅      | Custom bucket `mulhamna/scoop-bucket` for `jirac` + `jirac-mcp` |
| Winget               | ❌    | ❌    | ✅      | Windows package manager for `mulhamna.jirac` + `mulhamna.jirac-mcp` |

## Choosing a channel (and avoiding conflicts)

Every channel installs the same `jirac` / `jirac-mcp` binaries — pick whichever
you prefer. They only clash if you install the **same binary from two channels
at once**, because more than one of them writes to the same directory on `PATH`.

- **Homebrew and npm both write to `/opt/homebrew/bin`** (Apple Silicon). Only
  one can own the `jirac` symlink at a time. If `brew link` fails with
  `already exists`, either let brew take over —
  `brew link --overwrite jira-commands` — or remove the npm copy first:
  `npm uninstall -g @mulham28/jirac`. To switch back to npm later,
  `brew unlink jira-commands` then reinstall the npm package.
- **Cargo installs to `~/.cargo/bin`**, which is usually earlier in `PATH`, so a
  `cargo install`ed `jirac` shadows the brew/npm one. That's fine (same tool);
  if you want brew/npm to win, `cargo uninstall jira-commands` or put
  `/opt/homebrew/bin` ahead of `~/.cargo/bin` in your `PATH`.
- Check which one wins with `which -a jirac jirac-mcp`.

## Homebrew (macOS / Linux)

Install both packages:

```bash
brew tap mulhamna/tap
brew trust mulhamna/tap   # newer Homebrew requires trusting third-party taps
brew install jira-commands jira-mcp
```

> If you see `Refusing to load formula ... from untrusted tap`, run
> `brew trust mulhamna/tap` (one-time) and re-run the install.

Install just one:

```bash
brew install jira-commands
brew install jira-mcp
```

## Install script (macOS / Linux)

```bash
curl -sSL https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.sh | bash
```

Install `jirac-mcp` instead:

```bash
curl -sSL https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.sh | BINARY=jirac-mcp bash
```

## PowerShell installer (Windows)

The installer detects the host architecture (`AMD64` or `ARM64`) and downloads the matching release archive.

```powershell
powershell -ExecutionPolicy Bypass -Command "& ([scriptblock]::Create((Invoke-WebRequest 'https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.ps1').Content))"
```

Install `jirac-mcp` instead:

```powershell
powershell -ExecutionPolicy Bypass -Command "& ([scriptblock]::Create((Invoke-WebRequest 'https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.ps1').Content))" -Binary jirac-mcp
```

## Scoop (Windows)

The `mulhamna/jirac` manifest publishes both `64bit` and `arm64` URLs; Scoop picks the right one for the host.

```powershell
scoop bucket add mulhamna https://github.com/mulhamna/scoop-bucket
scoop install mulhamna/jirac
scoop install mulhamna/jirac-mcp
```

## Cargo

```bash
cargo install jira-commands
cargo install jira-mcp
```

## From source (local checkout)

For contributors or anyone tracking `main`:

```bash
git clone https://github.com/mulhamna/jira-commands
cd jira-commands
cargo install --path crates/jira --locked      # installs `jirac`
cargo install --path crates/jira-mcp --locked  # installs `jirac-mcp`
```

## npm

Install the CLI, the MCP server, or both:

```bash
npm install -g @mulham28/jirac        # jirac CLI + TUI
npm install -g @mulham28/jirac-mcp    # jirac-mcp MCP server
```

Each package downloads the matching prebuilt release binary during install. Linux support depends on the release binary's glibc compatibility.

## ClawHub

If you use OpenClaw, install the published ClawHub surfaces directly:

```bash
openclaw skills install jirac
openclaw plugins install clawhub:jirac-plugin
```

Pages:
- Skill: <https://clawhub.ai/mulhamna/jirac>
- Plugin: <https://clawhub.ai/plugins/jirac-plugin>

## GitHub Releases

Download from:

- <https://github.com/mulhamna/jira-commands/releases>

Preferred CLI (`vX.Y.Z`) archives:

| Platform            | Archive                      |
| ------------------- | ---------------------------- |
| macOS Apple Silicon | `jirac-macos-aarch64.tar.gz` |
| macOS Intel         | `jirac-macos-x86_64.tar.gz`  |
| Linux x86_64        | `jirac-linux-x86_64.tar.gz`  |
| Linux ARM64         | `jirac-linux-aarch64.tar.gz` |
| Windows x86_64      | `jirac-windows-x86_64.zip`   |
| Windows ARM64       | `jirac-windows-aarch64.zip`  |

Preferred MCP (`jira-mcp-vX.Y.Z`) archives:

| Platform            | Archive                          |
| ------------------- | -------------------------------- |
| macOS Apple Silicon | `jirac-mcp-macos-aarch64.tar.gz` |
| macOS Intel         | `jirac-mcp-macos-x86_64.tar.gz`  |
| Linux x86_64        | `jirac-mcp-linux-x86_64.tar.gz`  |
| Linux ARM64         | `jirac-mcp-linux-aarch64.tar.gz` |
| Windows x86_64      | `jirac-mcp-windows-x86_64.zip`   |
| Windows ARM64       | `jirac-mcp-windows-aarch64.zip`  |

## Winget (Windows)

```powershell
winget install mulhamna.jirac
winget install mulhamna.jirac-mcp
```

If you prefer Scoop, see the dedicated Scoop section above.

## After install

Authenticate first:

```bash
# Simple login (Cloud or Data Center)
jirac auth login

# Save separate accounts
jirac auth login --profile work-cloud
jirac auth login --profile client-dc

# Switch active account later
jirac auth use client-dc
```

Then verify:

```bash
jirac auth status
jirac auth profiles
jirac --help
jirac tui --help
```

## MCP client install helper

If you want Jira available inside an MCP-capable client, install `jirac-mcp` first, then run the helper.

Interactive mode (recommended) — verifies prereqs and shows a picker:

```bash
jirac mcp install
```

Non-interactive mode for scripts — pass `--client` explicitly:

```bash
jirac mcp install --client claude-code
jirac mcp install --client claude-desktop
jirac mcp install --client cursor
jirac mcp install --client gemini-cli
jirac mcp install --client codex
jirac mcp install --client opencode
jirac mcp install --client generic-json
jirac mcp install --client antigravity
jirac mcp install --client antigravity-cli
```

Supported targets now:
- `claude-code` (`~/.claude.json`, user-level JSON with `mcpServers`)
- `claude-desktop` (`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS, `%APPDATA%\Claude\claude_desktop_config.json` on Windows)
- `cursor` (`~/.cursor/mcp.json`, provisional path until verified in a real Cursor install)
- `gemini-cli` (delegates to `gemini mcp add -s user ...`)
- `codex` (delegates to `codex mcp add ...`)
- `opencode` (`~/.config/opencode/opencode.jsonc`, direct JSONC write)
- `generic-json` (prints a portable JSON snippet instead of writing a file)
- `antigravity` (`~/.gemini/antigravity/mcp_config.json`, user-level JSON with `mcpServers`)
- `antigravity-cli` (`~/.gemini/config/mcp_config.json`, user-level JSON with `mcpServers`)

Helpful flags:
- `--print` prints the JSON snippet or delegated client command first
- `--dry-run` previews without writing
- `--force` overwrites an existing MCP entry with the same name, or runs remove+add for delegated clients
- install helpers use the active Jira profile by default; if you only have one Jira login configured, that is the profile the MCP server will use
- `--name jira` changes the MCP server name
- `--command jirac-mcp` changes the launched binary
- `--transport stdio` changes the transport args

Recommended check:

```bash
jirac mcp doctor
```

Local verification notes:
- Claude Code user scope writes `~/.claude.json` (top-level `mcpServers`); project-scoped MCP servers live in `<repo>/.mcp.json` and are not written by this helper
- Claude Desktop user scope writes the platform support directory (`claude_desktop_config.json`); override with `CLAUDE_DESKTOP_CONFIG`
- Gemini CLI currently stores user MCP config in `~/.gemini/settings.json`; this helper delegates to the Gemini CLI directly
- Codex stores MCP entries under `~/.codex/config.toml`; this helper delegates to the Codex CLI directly
- Antigravity user scope writes `~/.gemini/antigravity/mcp_config.json` (top-level `mcpServers`); override with `ANTIGRAVITY_CONFIG`
- Antigravity CLI user scope writes `~/.gemini/config/mcp_config.json` (`mcpServers`); override with `ANTIGRAVITY_CLI_CONFIG`
- For local-binary clients, `jirac mcp install` requires `jirac-mcp` on PATH. Install it with `cargo install jira-mcp`, download a release binary, or pass `--command /path/to/jirac-mcp`.
