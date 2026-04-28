# INSTALL

Detailed installation guide for `jirac` and `jirac-mcp`.

## Supported install paths

| Method               | macOS | Linux | Windows | Notes                                  |
| -------------------- | ----- | ----- | ------- | -------------------------------------- |
| Homebrew             | ✅     | ✅     | No      | `jira-commands` and `jira-mcp` via `mulhamna/tap` |
| Install script       | ✅     | ✅     | No      | Downloads latest release asset         |
| PowerShell installer | ❌     | ❌     | ✅       | Installs `jirac.exe` to user-local bin |
| Cargo                | ✅     | ✅     | ✅       | Best for Rust users                    |
| GitHub Releases      | ✅     | ✅     | ✅       | Manual download of archives/binaries   |
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

## Cargo

```bash
cargo install jira-commands
```

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
