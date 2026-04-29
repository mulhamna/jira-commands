#!/usr/bin/env python3
from __future__ import annotations

import json
import os
from pathlib import Path

VERSION = os.environ["VERSION"]
SHA_WINDOWS_X86 = os.environ["SHA_WINDOWS_X86"]
REPOSITORY = os.environ.get("REPOSITORY", "mulhamna/jira-commands")
MANIFEST_PATH = Path(os.environ.get("SCOOP_MANIFEST_PATH", "bucket/jirac.json"))

manifest = json.loads(MANIFEST_PATH.read_text())
manifest["version"] = VERSION
manifest["url"] = f"https://github.com/{REPOSITORY}/releases/download/v{VERSION}/jirac-windows-x86_64.zip"
manifest["hash"] = SHA_WINDOWS_X86
manifest.setdefault("checkver", {})
manifest.setdefault("autoupdate", {})
manifest["autoupdate"]["url"] = "https://github.com/{repo}/releases/download/v$version/jirac-windows-x86_64.zip".format(repo=REPOSITORY)
MANIFEST_PATH.write_text(json.dumps(manifest, indent=2) + "\n")
print(f"updated {MANIFEST_PATH} -> v{VERSION}")
