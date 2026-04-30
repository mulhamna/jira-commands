# jira-commands

Install [`jirac`](https://github.com/mulhamna/jira-commands), a fast Jira CLI with TUI, straight from npm.

```bash
npm install -g jira-commands
jirac --help
```

## Why this package exists

`jirac` is written in Rust and ships as a single binary.

This npm package gives JavaScript and Node users a familiar install path while still using the official prebuilt release binaries from GitHub Releases.

## What you get

- fast native `jirac` binary
- no local Rust toolchain required
- install with normal npm global workflow
- checksum-verified download during install
- same release artifacts used by the main project release flow

## Install

```bash
npm install -g jira-commands
```

Then run:

```bash
jirac auth login
jirac issue list
jirac tui -p PROJ
```

## How it works

During `postinstall`, this package:

1. detects your OS and CPU architecture
2. downloads the matching `jirac` release asset
3. verifies it against the published `checksums.txt`
4. exposes the `jirac` command in your npm global bin path

## Supported targets

- macOS arm64
- macOS x64
- Linux x64
- Linux arm64
- Windows x64

## Good fit if you want

- `npm install -g jira-commands`
- a native Jira CLI without building from source
- an easier install path for dev teams already using Node

## Not included

This package installs `jirac` only.

It does not install:
- `jirac-mcp`
- a JavaScript reimplementation of the CLI
- Atlassian credentials or config for you

## Learn more

- GitHub: <https://github.com/mulhamna/jira-commands>
- Install docs: <https://github.com/mulhamna/jira-commands/blob/main/INSTALL.md>
- Releases: <https://github.com/mulhamna/jira-commands/releases>
