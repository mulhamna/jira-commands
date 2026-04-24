#!/usr/bin/env python3
from pathlib import Path
import os
import subprocess
import tempfile

GH_TOKEN = os.environ['GH_TOKEN']
VERSION = os.environ['VERSION']
SHA_WINDOWS_X86 = os.environ['SHA_WINDOWS_X86']
RELEASE_DATE = os.environ['RELEASE_DATE']
FORK_REPO = os.environ.get('WINGET_FORK_REPO', 'mulhamna/winget-pkgs')
UPSTREAM_REPO = os.environ.get('WINGET_UPSTREAM_REPO', 'microsoft/winget-pkgs')
BRANCH = f'chore/jirac-winget-v{VERSION}'
TITLE = f'Add version: mulhamna.jirac version {VERSION}'
BODY = f'Automated submission for jirac {VERSION} generated from the published GitHub release assets.'
FORK_BRANCH_URL = f'https://github.com/{FORK_REPO}/tree/{BRANCH}'
COMPARE_URL = f'https://github.com/{UPSTREAM_REPO}/compare/master...{FORK_REPO.split("/")[0]}:{BRANCH}?expand=1'


def run(cmd, cwd=None, check=True, capture_output=False):
    return subprocess.run(cmd, cwd=cwd, check=check, text=True, capture_output=capture_output)

with tempfile.TemporaryDirectory() as tmp:
    repo_dir = Path(tmp) / 'winget-pkgs'
    clone_url = f'https://x-access-token:{GH_TOKEN}@github.com/{FORK_REPO}.git'
    run(['git', 'clone', '--depth', '1', clone_url, str(repo_dir)])
    run(['git', 'config', 'user.name', 'Mulham'], cwd=repo_dir)
    run(['git', 'config', 'user.email', 'mulhamna@gmail.com'], cwd=repo_dir)
    run(['git', 'switch', '-C', BRANCH], cwd=repo_dir)

    manifest_dir = repo_dir / 'manifests' / 'm' / 'mulhamna' / 'jirac' / VERSION
    manifest_dir.mkdir(parents=True, exist_ok=True)

    files = {
        manifest_dir / 'mulhamna.jirac.yaml': f'''# yaml-language-server: $schema=https://aka.ms/winget-manifest.version.1.9.0.schema.json
PackageIdentifier: mulhamna.jirac
PackageVersion: {VERSION}
DefaultLocale: en-US
ManifestType: version
ManifestVersion: 1.9.0
''',
        manifest_dir / 'mulhamna.jirac.installer.yaml': f'''# yaml-language-server: $schema=https://aka.ms/winget-manifest.installer.1.9.0.schema.json
PackageIdentifier: mulhamna.jirac
PackageVersion: {VERSION}
InstallerType: zip
NestedInstallerType: portable
NestedInstallerFiles:
  - RelativeFilePath: jirac.exe
    PortableCommandAlias: jirac
ReleaseDate: {RELEASE_DATE}
Installers:
  - Architecture: x64
    InstallerUrl: https://github.com/mulhamna/jira-commands/releases/download/v{VERSION}/jirac-windows-x86_64.zip
    InstallerSha256: {SHA_WINDOWS_X86}
ManifestType: installer
ManifestVersion: 1.9.0
''',
        manifest_dir / 'mulhamna.jirac.locale.en-US.yaml': f'''# yaml-language-server: $schema=https://aka.ms/winget-manifest.defaultLocale.1.9.0.schema.json
PackageIdentifier: mulhamna.jirac
PackageVersion: {VERSION}
PackageLocale: en-US
Publisher: mulhamna
PublisherUrl: https://github.com/mulhamna
PublisherSupportUrl: https://github.com/mulhamna/jira-commands/issues
Author: mulhamna
PackageName: jirac
PackageUrl: https://github.com/mulhamna/jira-commands
License: MIT OR Apache-2.0
LicenseUrl: https://github.com/mulhamna/jira-commands/blob/main/LICENSE
ShortDescription: Jira terminal client with TUI, MCP support, and release archives for Windows, macOS, and Linux.
Description: jirac is a Rust-based Jira CLI with interactive TUI flows, issue transitions, comments, worklogs, attachments, and jirac-mcp for editor and agent integrations.
Moniker: jirac
Tags:
  - jira
  - atlassian
  - cli
  - tui
  - mcp
ManifestType: defaultLocale
ManifestVersion: 1.9.0
''',
    }

    for path, content in files.items():
        path.write_text(content)

    run(['git', 'add', str(manifest_dir)], cwd=repo_dir)
    diff = subprocess.run(['git', 'diff', '--cached', '--quiet'], cwd=repo_dir)
    if diff.returncode == 0:
        print(f'No Winget manifest changes to submit for {VERSION}.')
        raise SystemExit(0)

    run(['git', 'commit', '-m', TITLE], cwd=repo_dir)
    run(['git', 'push', '--force-with-lease', 'origin', BRANCH], cwd=repo_dir)

    existing = run([
        'gh', 'pr', 'list', '--repo', UPSTREAM_REPO, '--head', f'mulhamna:{BRANCH}', '--state', 'open', '--json', 'number', '--jq', '.[0].number // empty'
    ], cwd=repo_dir, capture_output=True).stdout.strip()
    if existing:
        print(f'Upstream PR already open: #{existing}')
        raise SystemExit(0)

    created = subprocess.run([
        'gh', 'pr', 'create',
        '--repo', UPSTREAM_REPO,
        '--head', f'mulhamna:{BRANCH}',
        '--base', 'master',
        '--title', TITLE,
        '--body', BODY,
    ], cwd=repo_dir, text=True, capture_output=True)

    if created.returncode == 0:
        print(created.stdout.strip())
        raise SystemExit(0)

    stderr = (created.stderr or '').strip()
    if 'Resource not accessible by personal access token' in stderr:
        print('Upstream PR creation was blocked by token permissions, but the fork branch was pushed successfully.')
        print(f'Fork branch: {FORK_BRANCH_URL}')
        print(f'Open PR manually: {COMPARE_URL}')
        raise SystemExit(0)

    print(stderr)
    raise SystemExit(created.returncode)
