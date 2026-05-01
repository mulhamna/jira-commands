# INSTALL

Detailed installation guide for `jirac` and `jirac-mcp`.

## Supported install paths

| Method               | macOS | Linux | Windows | Notes                                  |
| -------------------- | ----- | ----- | ------- | -------------------------------------- |
| Homebrew             | ✅     | ✅     | No      | `jira-commands` and `jira-mcp` via `mulhamna/tap` |
| Install script       | ✅     | ✅     | No      | Downloads latest release asset         |
| PowerShell installer | ❌     | ❌     | ✅       | Installs `jirac.exe` to user-local bin |
| Cargo                | ✅     | ✅     | ✅       | Best for Rust users                    |
| npm                  | ✅     | ✅     | ✅       | Downloads prebuilt release binary      |
| GitHub Releases      | ✅     | ✅     | ✅       | Manual download of archives/binaries   |
| Scoop                | ❌     | ❌     | ✅       | Custom bucket `mulhamna/scoop-bucket`  |
| Winget               | ❌     | ❌     | ✅       | Windows package manager                |
| Chocolatey           | ❌     | ❌     | ✅       | Windows package manager                |

## Homebrew (macOS / Linux)

```bash
brew tap mulhamna/tap
brew install jira-commands

# Optional MCP server
brew install jira-mcp
```

Install both binaries with Homebrew:

```bash
brew tap mulhamna/tap
brew install jira-commands jira-mcp
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

```powershell
powershell -ExecutionPolicy Bypass -Command "& ([scriptblock]::Create((Invoke-WebRequest 'https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.ps1').Content))"
```

Install `jirac-mcp` instead:

```powershell
powershell -ExecutionPolicy Bypass -Command "& ([scriptblock]::Create((Invoke-WebRequest 'https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.ps1').Content))" -Binary jirac-mcp
```

## Scoop (Windows)

```powershell
scoop bucket add mulhamna https://github.com/mulhamna/scoop-bucket
scoop install mulhamna/jirac
```

## Cargo

```bash
cargo install jira-commands
```

## npm

```bash
npm install -g @mulham28/jirac
```

The npm package downloads the matching prebuilt `jirac` release binary during install. Linux support depends on the release binary's glibc compatibility.

Install MCP binary:

```bash
cargo install jira-mcp
```

## GitHub Releases

Download from:

- <https://github.com/mulhamna/jira-commands/releases>

Preferred archives:

| Platform            | Archive                      |
| ------------------- | ---------------------------- |
| macOS Apple Silicon | `jirac-macos-aarch64.tar.gz` |
| macOS Intel         | `jirac-macos-x86_64.tar.gz`  |
| Linux x86_64        | `jirac-linux-x86_64.tar.gz`  |
| Linux ARM64         | `jirac-linux-aarch64.tar.gz` |
| Windows x86_64      | `jirac-windows-x86_64.zip`   |

## Winget (Windows)

If you prefer Scoop, use the custom bucket instead:

```powershell
scoop bucket add mulhamna https://github.com/mulhamna/scoop-bucket
scoop install mulhamna/jirac
```


```powershell
winget install mulhamna.jirac
```

## Chocolatey (Windows)

```powershell
choco install jirac
```

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

If you want Jira available inside an MCP-capable client, install `jirac-mcp` first, then use:

```bash
jirac mcp install --client claude-code
jirac mcp install --client claude-desktop
jirac mcp install --client cursor
jirac mcp install --client gemini-cli
jirac mcp install --client codex
jirac mcp install --client generic-json
```

Supported targets now:
- `claude-code` (`.mcp.json`, project-style JSON)
- `claude-desktop` (`~/.claude.json`, user-level JSON)
- `cursor` (`~/.cursor/mcp.json`, provisional path until verified in a real Cursor install)
- `gemini-cli` (delegates to `gemini mcp add -s user ...`)
- `codex` (delegates to `codex mcp add ...`)
- `generic-json` (prints a portable JSON snippet instead of writing a file)

Helpful flags:
- `--print` prints the JSON snippet or delegated client command first
- `--dry-run` previews without writing
- `--force` overwrites an existing MCP entry with the same name, or runs remove+add for delegated clients
- `--name jira` changes the MCP server name
- `--command jirac-mcp` changes the launched binary
- `--transport stdio` changes the transport args

Recommended check:

```bash
jirac mcp doctor
```

Local verification notes:
- Claude Code project scope writes `.mcp.json`
- Claude Desktop user scope writes `~/.claude.json`
- Gemini CLI currently stores user MCP config in `~/.gemini/settings.json`; this helper delegates to the Gemini CLI directly
- Codex stores MCP entries under `~/.codex/config.toml`; this helper delegates to the Codex CLI directly
