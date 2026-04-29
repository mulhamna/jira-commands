#!/usr/bin/env python3
from __future__ import annotations

import json
import os
import re
import sys
import urllib.request
from pathlib import Path

OWNER = os.environ.get("GITHUB_REPOSITORY_OWNER", "mulhamna")
REPO = os.environ.get("GITHUB_REPOSITORY", "mulhamna/jira-commands").split("/", 1)[-1]
SCOOP_BUCKET = os.environ.get("SCOOP_BUCKET", "mulhamna/scoop-bucket")
README = Path("README.md")
INSTALL = Path("INSTALL.md")
START = "<!-- contributors:start -->"
END = "<!-- contributors:end -->"


def fetch_json(url: str):
    req = urllib.request.Request(
        url,
        headers={
            "Accept": "application/vnd.github+json",
            "User-Agent": "jirac-release-readme-refresh",
        },
    )
    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
    if token:
        req.add_header("Authorization", f"Bearer {token}")
    with urllib.request.urlopen(req, timeout=30) as resp:
        return json.load(resp)


def fetch_contributors(limit: int = 18):
    url = f"https://api.github.com/repos/{OWNER}/{REPO}/contributors?per_page={limit}"
    data = fetch_json(url)
    cards = []
    for item in data:
        login = item.get("login")
        avatar = item.get("avatar_url")
        html = item.get("html_url")
        if not (login and avatar and html):
            continue
        cards.append(
            f'<a href="{html}" title="@{login}"><img src="{avatar}&s=72" width="36" height="36" alt="{login}" /></a>'
        )
    return cards


def replace_install_block(text: str) -> str:
    old = """### Quick install commands\n\n#### Homebrew (macOS / Linux)\n\n```bash\nbrew tap mulhamna/tap\nbrew install jira-commands\n\n# Optional MCP server\nbrew install jira-mcp\n```\n\n#### Install script (macOS / Linux)\n\n```bash\ncurl -sSL https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.sh | bash\n```\n\n#### PowerShell installer (Windows)\n\n```powershell\npowershell -ExecutionPolicy Bypass -Command \"& ([scriptblock]::Create((Invoke-WebRequest 'https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.ps1').Content))\"\n```\n\nInstalls `jirac.exe` to `%LOCALAPPDATA%\\Programs\\jirac\\bin` and adds that directory to your user `PATH`.\n\n#### Cargo\n\n```bash\ncargo install jira-commands\n```\n\n#### Winget (Windows)\n\n```powershell\nwinget install mulhamna.jirac\n```\n\n#### Chocolatey (Windows)\n\n```powershell\nchoco install jirac\n```\n\nChocolatey packages are published automatically from official GitHub releases.\n\n#### GitHub Releases\n"""
    new = f"""### Quick install commands\n\n#### Homebrew (macOS / Linux)\n\n```bash\nbrew tap mulhamna/tap\nbrew install jira-commands\n\n# Optional MCP server\nbrew install jira-mcp\n```\n\n#### Install script (macOS / Linux)\n\n```bash\ncurl -sSL https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.sh | bash\n```\n\n#### PowerShell installer (Windows)\n\n```powershell\npowershell -ExecutionPolicy Bypass -Command \"& ([scriptblock]::Create((Invoke-WebRequest 'https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.ps1').Content))\"\n```\n\nInstalls `jirac.exe` to `%LOCALAPPDATA%\\Programs\\jirac\\bin` and adds that directory to your user `PATH`.\n\n#### Scoop (Windows)\n\n```powershell\nscoop bucket add mulhamna https://github.com/{SCOOP_BUCKET}\nscoop install mulhamna/jirac\n```\n\n#### Cargo\n\n```bash\ncargo install jira-commands\n```\n\n#### Winget (Windows)\n\n```powershell\nwinget install mulhamna.jirac\n```\n\n#### Chocolatey (Windows)\n\n```powershell\nchoco install jirac\n```\n\nChocolatey packages are published automatically from official GitHub releases.\n\n#### GitHub Releases\n"""
    if old not in text:
        raise SystemExit("expected quick install block not found in README.md")
    return text.replace(old, new, 1)


def replace_footer(text: str, contributors: list[str]) -> str:
    if START not in text or END not in text:
        raise SystemExit("contributors markers missing from README.md")
    body = "\n".join([
        START,
        "## Contributors",
        "",
        "Thanks to everyone helping shape `jirac`. This footer is refreshed automatically during the release lane.",
        "",
        '<p align="left">',
        *(contributors or ["_Contributor avatars will appear after the first successful refresh._"]),
        "</p>",
        END,
    ])
    return re.sub(re.escape(START) + r".*?" + re.escape(END), body, text, flags=re.S)


def refresh_install_md(text: str) -> str:
    text = re.sub(
        r"\| Winget\s+\| ❌\s+\| ❌\s+\| ✅\s+\| Windows package manager\s+\|\n\| Chocolatey",
        "| Scoop                | ❌     | ❌     | ✅       | Custom bucket `mulhamna/scoop-bucket` |\n| Winget               | ❌     | ❌     | ✅       | Windows package manager                |\n| Chocolatey",
        text,
        count=1,
    )
    old = """## PowerShell installer (Windows)\n\n```powershell\npowershell -ExecutionPolicy Bypass -Command \"& ([scriptblock]::Create((Invoke-WebRequest 'https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.ps1').Content))\"\n```\n\nInstall `jirac-mcp` instead:\n\n```powershell\npowershell -ExecutionPolicy Bypass -Command \"& ([scriptblock]::Create((Invoke-WebRequest 'https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.ps1').Content))\" -Binary jirac-mcp\n```\n\n## Cargo\n"""
    new = f"""## PowerShell installer (Windows)\n\n```powershell\npowershell -ExecutionPolicy Bypass -Command \"& ([scriptblock]::Create((Invoke-WebRequest 'https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.ps1').Content))\"\n```\n\nInstall `jirac-mcp` instead:\n\n```powershell\npowershell -ExecutionPolicy Bypass -Command \"& ([scriptblock]::Create((Invoke-WebRequest 'https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.ps1').Content))\" -Binary jirac-mcp\n```\n\n## Scoop (Windows)\n\n```powershell\nscoop bucket add mulhamna https://github.com/{SCOOP_BUCKET}\nscoop install mulhamna/jirac\n```\n\n## Cargo\n"""
    if old not in text:
        raise SystemExit("expected INSTALL.md section not found")
    return text.replace(old, new, 1)


def main() -> int:
    readme = README.read_text()
    install = INSTALL.read_text()
    contributors = fetch_contributors()
    readme = replace_install_block(readme)
    readme = replace_footer(readme, contributors)
    install = refresh_install_md(install)
    README.write_text(readme)
    INSTALL.write_text(install)
    return 0


if __name__ == "__main__":
    sys.exit(main())
