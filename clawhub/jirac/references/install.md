# Install jirac

Choose the installer that fits your environment.

## Homebrew (macOS / Linux)

```bash
brew tap mulhamna/tap
brew install jira-commands
```

## Cargo

```bash
cargo install jira-commands
```

## GitHub Releases

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

## Windows install script

```powershell
powershell -ExecutionPolicy Bypass -Command "& ([scriptblock]::Create((Invoke-WebRequest 'https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.ps1').Content))"
```

## macOS / Linux install script

```bash
curl -sSL https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.sh | bash
```

## Verify install

```bash
jirac --version
jirac auth login
jirac auth status
```
