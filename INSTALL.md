# INSTALL

Detailed installation guide for `jirac` and `jirac-mcp`.

## Supported install paths

| Method | macOS | Linux | Windows | Notes |
| --- | --- | --- | --- | --- |
| Homebrew | Yes | Yes | No | `jirac` formula via `mulhamna/tap` |
| Install script | Yes | Yes | No | Downloads latest release asset |
| PowerShell installer | No | No | Yes | Installs `jirac.exe` to user-local bin |
| Cargo | Yes | Yes | Yes | Best for Rust users |
| GitHub Releases | Yes | Yes | Yes | Manual download of archives/binaries |
| Winget | No | No | Yes | Windows package manager |
| Chocolatey | No | No | Yes | Windows package manager |

## Homebrew (macOS / Linux)

```bash
brew tap mulhamna/tap
brew install jira-commands
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

| Platform | Archive |
| --- | --- |
| macOS Apple Silicon | `jirac-macos-aarch64.tar.gz` |
| macOS Intel | `jirac-macos-x86_64.tar.gz` |
| Linux x86_64 | `jirac-linux-x86_64.tar.gz` |
| Linux ARM64 | `jirac-linux-aarch64.tar.gz` |
| Windows x86_64 | `jirac-windows-x86_64.zip` |

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
jirac auth login
```

Then verify:

```bash
jirac --help
jirac tui --help
```
