#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/crates/zed-jira"
DEST="${1:-$ROOT/../jirac-ext}"

if [ ! -d "$SRC" ]; then
  echo "Source crate not found: $SRC" >&2
  exit 1
fi

mkdir -p "$DEST"

python3 - "$SRC" "$DEST" <<'PY'
from pathlib import Path
import shutil
import sys

src = Path(sys.argv[1])
dest = Path(sys.argv[2])

managed_files = ["Cargo.toml", "README.md", "extension.toml"]
managed_dirs = ["src", "configuration"]

for name in managed_files:
    shutil.copy2(src / name, dest / name)

for name in managed_dirs:
    target = dest / name
    if target.exists():
        shutil.rmtree(target)
    shutil.copytree(src / name, target)

readme = (dest / "README.md").read_text()
readme = readme.replace(
    "Official Zed extension wrapper for `jirac-mcp`.\n",
    "Official Zed extension wrapper for `jirac-mcp`, published as the standalone `jirac-ext` Zed extension repository.\n",
    1,
)
readme = readme.replace(
    "To refresh the dedicated `jirac-ext` mirror repo from the repository root, run:\n\n```bash\n./scripts/sync-jirac-ext.sh\n```\n",
    "This repository is mirrored from `jira-commands`. Refresh it from a local `jira-commands` checkout with:\n\n```bash\n../jira-commands/scripts/sync-jirac-ext.sh .\n```\n",
    1,
)
if "## Repository model" not in readme:
    readme += """

## Repository model

- `jira-commands` is the source-of-truth repository for the wrapper implementation (`crates/zed-jira`) and the `jirac-mcp` binary releases.
- `jirac-ext` is the dedicated mirror repository used for Zed marketplace submission and submodule consumption from `zed-industries/extensions`.
- This repository does **not** publish its own GitHub releases. At runtime, the extension downloads `jirac-mcp` assets from <https://github.com/mulhamna/jira-commands/releases>.
"""

(dest / "README.md").write_text(readme)

extension_toml = (dest / "extension.toml").read_text()
extension_toml = extension_toml.replace(
    'repository = "https://github.com/mulhamna/jira-commands"',
    'repository = "https://github.com/mulhamna/jirac-ext"',
)
extension_toml = extension_toml.replace(
    'description = "Jira MCP server for Zed — browse issues, run JQL, transition, create, and more via the Agent Panel."',
    'description = "Jira MCP wrapper for Zed — uses jirac-mcp releases from jira-commands for issue browsing, JQL, transitions, creation, and more via the Agent Panel."',
)
(dest / "extension.toml").write_text(extension_toml)
PY

echo "Synced zed-jira crate into $DEST"
