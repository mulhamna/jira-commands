#!/usr/bin/env python3
from __future__ import annotations

import json
import os
from pathlib import Path

VERSION = os.environ["VERSION"]
SHA_WINDOWS_X86 = os.environ["SHA_WINDOWS_X86"]
SHA_WINDOWS_ARM = os.environ.get("SHA_WINDOWS_ARM")
REPOSITORY = os.environ.get("REPOSITORY", "mulhamna/jira-commands")
MANIFEST_PATH = Path(os.environ.get("SCOOP_MANIFEST_PATH", "bucket/jirac.json"))
SCOOP_BINARY = os.environ.get("SCOOP_BINARY", "jirac")
SCOOP_RELEASE_TAG_PREFIX = os.environ.get("SCOOP_RELEASE_TAG_PREFIX", "v")
SCOOP_WINDOWS_ARCHIVE = os.environ.get(
    "SCOOP_WINDOWS_ARCHIVE",
    f"{SCOOP_BINARY}-windows-x86_64.zip",
)
SCOOP_WINDOWS_ARCHIVE_ARM = os.environ.get(
    "SCOOP_WINDOWS_ARCHIVE_ARM",
    f"{SCOOP_BINARY}-windows-aarch64.zip",
)


def release_url(archive: str) -> str:
    return (
        f"https://github.com/{REPOSITORY}/releases/download/"
        f"{SCOOP_RELEASE_TAG_PREFIX}{VERSION}/{archive}"
    )


def autoupdate_url(archive: str) -> str:
    return (
        f"https://github.com/{REPOSITORY}/releases/download/"
        f"{SCOOP_RELEASE_TAG_PREFIX}$version/{archive}"
    )


manifest = json.loads(MANIFEST_PATH.read_text())
manifest["version"] = VERSION

if SHA_WINDOWS_ARM:
    # Per-arch schema: emit architecture.{64bit,arm64} and drop legacy
    # top-level url/hash so Scoop picks the right asset per host.
    manifest.pop("url", None)
    manifest.pop("hash", None)
    manifest["architecture"] = {
        "64bit": {
            "url": release_url(SCOOP_WINDOWS_ARCHIVE),
            "hash": SHA_WINDOWS_X86,
        },
        "arm64": {
            "url": release_url(SCOOP_WINDOWS_ARCHIVE_ARM),
            "hash": SHA_WINDOWS_ARM,
        },
    }
    autoupdate = manifest.setdefault("autoupdate", {})
    autoupdate.pop("url", None)
    autoupdate["architecture"] = {
        "64bit": {"url": autoupdate_url(SCOOP_WINDOWS_ARCHIVE)},
        "arm64": {"url": autoupdate_url(SCOOP_WINDOWS_ARCHIVE_ARM)},
    }
else:
    manifest["url"] = release_url(SCOOP_WINDOWS_ARCHIVE)
    manifest["hash"] = SHA_WINDOWS_X86
    autoupdate = manifest.setdefault("autoupdate", {})
    autoupdate["url"] = autoupdate_url(SCOOP_WINDOWS_ARCHIVE)

manifest.setdefault("checkver", {})

MANIFEST_PATH.write_text(json.dumps(manifest, indent=2) + "\n")
print(f"updated {MANIFEST_PATH} -> v{VERSION}")
