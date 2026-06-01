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
PACKAGE_IDENTIFIER = os.environ.get('WINGET_PACKAGE_IDENTIFIER', 'mulhamna.jirac')
PACKAGE_NAME = os.environ.get('WINGET_PACKAGE_NAME', 'jirac')
PACKAGE_MONIKER = os.environ.get('WINGET_PACKAGE_MONIKER', PACKAGE_NAME)
PACKAGE_DESCRIPTION = os.environ.get(
    'WINGET_PACKAGE_DESCRIPTION',
    'jirac is a Rust-based Jira CLI with interactive TUI flows, issue transitions, comments, worklogs, attachments, and jirac-mcp for editor and agent integrations.',
)
PACKAGE_SHORT_DESCRIPTION = os.environ.get(
    'WINGET_PACKAGE_SHORT_DESCRIPTION',
    'Jira terminal client with TUI, MCP support, and release archives for Windows, macOS, and Linux.',
)
PACKAGE_TAGS = [tag for tag in os.environ.get('WINGET_PACKAGE_TAGS', 'jira,atlassian,cli,tui,mcp').split(',') if tag]
RELEASE_TAG_PREFIX = os.environ.get('WINGET_RELEASE_TAG_PREFIX', 'v')
WINDOWS_ARCHIVE = os.environ.get('WINGET_WINDOWS_ARCHIVE', 'jirac-windows-x86_64.zip')
NESTED_INSTALLER_FILE = os.environ.get('WINGET_NESTED_INSTALLER_FILE', 'jirac-windows-x86_64.exe')
PORTABLE_COMMAND_ALIAS = os.environ.get('WINGET_PORTABLE_COMMAND_ALIAS', 'jirac')
BRANCH_PREFIX = os.environ.get('WINGET_BRANCH_PREFIX', 'jirac-winget')
BRANCH = f'chore/{BRANCH_PREFIX}-v{VERSION}'
TITLE = f'Add version: {PACKAGE_IDENTIFIER} version {VERSION}'
BODY = f'Automated submission for {PACKAGE_NAME} {VERSION} generated from the published GitHub release assets.'
FORK_BRANCH_URL = f'https://github.com/{FORK_REPO}/tree/{BRANCH}'
COMPARE_URL = f'https://github.com/{UPSTREAM_REPO}/compare/master...{FORK_REPO.split("/")[0]}:{BRANCH}?expand=1'


def run(cmd, cwd=None, check=True, capture_output=False):
    return subprocess.run(cmd, cwd=cwd, check=check, text=True, capture_output=capture_output)


parts = PACKAGE_IDENTIFIER.split('.', 2)
if len(parts) != 2:
    raise SystemExit(f'Expected WINGET_PACKAGE_IDENTIFIER like publisher.package, got: {PACKAGE_IDENTIFIER!r}')
publisher_slug, package_slug = parts

with tempfile.TemporaryDirectory() as tmp:
    repo_dir = Path(tmp) / 'winget-pkgs'
    clone_url = f'https://x-access-token:{GH_TOKEN}@github.com/{FORK_REPO}.git'
    run(['git', 'clone', '--depth', '1', clone_url, str(repo_dir)])
    run(['git', 'config', 'user.name', 'Mulham'], cwd=repo_dir)
    run(['git', 'config', 'user.email', 'mulhamna@gmail.com'], cwd=repo_dir)
    run(['git', 'switch', '-C', BRANCH], cwd=repo_dir)

    manifest_dir = repo_dir / 'manifests' / publisher_slug[0].lower() / publisher_slug / package_slug / VERSION
    manifest_dir.mkdir(parents=True, exist_ok=True)
    tags_yaml = '\n'.join(f'  - {tag}' for tag in PACKAGE_TAGS)

    files = {
        manifest_dir / f'{PACKAGE_IDENTIFIER}.yaml': f'''# yaml-language-server: $schema=https://aka.ms/winget-manifest.version.1.9.0.schema.json
PackageIdentifier: {PACKAGE_IDENTIFIER}
PackageVersion: {VERSION}
DefaultLocale: en-US
ManifestType: version
ManifestVersion: 1.9.0
''',
        manifest_dir / f'{PACKAGE_IDENTIFIER}.installer.yaml': f'''# yaml-language-server: $schema=https://aka.ms/winget-manifest.installer.1.9.0.schema.json
PackageIdentifier: {PACKAGE_IDENTIFIER}
PackageVersion: {VERSION}
InstallerType: zip
NestedInstallerType: portable
NestedInstallerFiles:
  - RelativeFilePath: {NESTED_INSTALLER_FILE}
    PortableCommandAlias: {PORTABLE_COMMAND_ALIAS}
ReleaseDate: {RELEASE_DATE}
Installers:
  - Architecture: x64
    InstallerUrl: https://github.com/mulhamna/jira-commands/releases/download/{RELEASE_TAG_PREFIX}{VERSION}/{WINDOWS_ARCHIVE}
    InstallerSha256: {SHA_WINDOWS_X86}
ManifestType: installer
ManifestVersion: 1.9.0
''',
        manifest_dir / f'{PACKAGE_IDENTIFIER}.locale.en-US.yaml': f'''# yaml-language-server: $schema=https://aka.ms/winget-manifest.defaultLocale.1.9.0.schema.json
PackageIdentifier: {PACKAGE_IDENTIFIER}
PackageVersion: {VERSION}
PackageLocale: en-US
Publisher: mulhamna
PublisherUrl: https://github.com/mulhamna
PublisherSupportUrl: https://github.com/mulhamna/jira-commands/issues
Author: mulhamna
PackageName: {PACKAGE_NAME}
PackageUrl: https://github.com/mulhamna/jira-commands
License: MIT OR Apache-2.0
LicenseUrl: https://github.com/mulhamna/jira-commands/blob/main/LICENSE
ShortDescription: {PACKAGE_SHORT_DESCRIPTION}
Description: {PACKAGE_DESCRIPTION}
Moniker: {PACKAGE_MONIKER}
Tags:
{tags_yaml}
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
