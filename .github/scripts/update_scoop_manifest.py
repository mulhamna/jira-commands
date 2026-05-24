#!/usr/bin/env python3
from __future__ import annotations

import json
import os
from pathlib import Path

VERSION = os.environ["VERSION"]
SHA_WINDOWS_X86 = os.environ["SHA_WINDOWS_X86"]
REPOSITORY = os.environ.get("REPOSITORY", "mulhamna/jira-commands")
MANIFEST_PATH = Path(os.environ.get("SCOOP_MANIFEST_PATH", "bucket/jirac.json"))
SCOOP_BINARY = os.environ.get("SCOOP_BINARY", "jirac")
SCOOP_RELEASE_TAG_PREFIX = os.environ.get("SCOOP_RELEASE_TAG_PREFIX", "v")
SCOOP_WINDOWS_ARCHIVE = os.environ.get(
    "SCOOP_WINDOWS_ARCHIVE",
    f"{SCOOP_BINARY}-windows-x86_64.zip",
)

manifest = json.loads(MANIFEST_PATH.read_text())
manifest["version"] = VERSION
manifest["url"] = (
    f"https://github.com/{REPOSITORY}/releases/download/"
    f"{SCOOP_RELEASE_TAG_PREFIX}{VERSION}/{SCOOP_WINDOWS_ARCHIVE}"
)
manifest["hash"] = SHA_WINDOWS_X86
manifest.setdefault("checkver", {})
manifest.setdefault("autoupdate", {})
manifest["autoupdate"]["url"] = (
    "https://github.com/{repo}/releases/download/{tag}$version/{archive}".format(
        repo=REPOSITORY,
        tag=SCOOP_RELEASE_TAG_PREFIX,
        archive=SCOOP_WINDOWS_ARCHIVE,
    )
)
MANIFEST_PATH.write_text(json.dumps(manifest, indent=2) + "\n")
print(f"updated {MANIFEST_PATH} -> v{VERSION}")
