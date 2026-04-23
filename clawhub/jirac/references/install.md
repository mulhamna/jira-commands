# Install jirac

Choose the installer that fits your environment. Prefer package managers or verified release archives before using installer scripts.

## Recommended options

### Homebrew (macOS / Linux)

```bash
brew tap mulhamna/tap
brew install jira-commands
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

## Installer scripts (optional)

Use these only if you prefer the project-provided installer flow. Review the script before running it.

### Windows PowerShell installer

```powershell
Invoke-WebRequest 'https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.ps1' -OutFile install.ps1
Get-Content ./install.ps1
powershell -ExecutionPolicy Bypass -File ./install.ps1
```

### macOS / Linux installer

```bash
curl -fsSLo install.sh https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.sh
cat ./install.sh
bash ./install.sh
```

## Verify install

```bash
jirac --version
jirac auth login
jirac auth status
```
