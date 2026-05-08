#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ZED_LIB="$ROOT/crates/zed-jira/src/lib.rs"
README="$ROOT/README.md"
INSTALL="$ROOT/INSTALL.md"
RELEASE_TAG="$ROOT/.github/workflows/release-tag.yml"
RELEASE_RECOVER="$ROOT/.github/workflows/release-recover.yml"
MIRROR_WORKFLOW="$ROOT/.github/workflows/zed-extension-mirror.yml"
CHECKLIST="$ROOT/docs/zed-extension-compat.md"
SYNC_SCRIPT="$ROOT/scripts/sync-jirac-ext.sh"

require_grep() {
  local pattern="$1"
  local file="$2"
  if ! grep -q "$pattern" "$file"; then
    echo "Missing pattern '$pattern' in $file" >&2
    exit 1
  fi
}

require_grep 'const REPO: &str = "mulhamna/jira-commands";' "$ZED_LIB"
require_grep '"serve".into(), "--transport".into(), "stdio".into()' "$ZED_LIB"
require_grep 'push_string(&mut env, settings.get("jira_url"), "JIRA_URL")' "$ZED_LIB"
require_grep 'push_string(&mut env, settings.get("jira_email"), "JIRA_EMAIL")' "$ZED_LIB"
require_grep 'push_string(&mut env, settings.get("jira_token"), "JIRA_TOKEN")' "$ZED_LIB"
require_grep 'push_string(&mut env, settings.get("default_project"), "JIRA_PROJECT")' "$ZED_LIB"
require_grep 'mulhamna/jirac-ext' "$README"
require_grep 'mulhamna/jirac-ext' "$INSTALL"
require_grep 'Verify jirac-ext mirror stays in sync' "$MIRROR_WORKFLOW"
require_grep 'Zed extension compatibility checklist' "$CHECKLIST"

affected_assets=(
  jirac-mcp-linux-x86_64
  jirac-mcp-linux-aarch64
  jirac-mcp-macos-x86_64
  jirac-mcp-macos-aarch64
  jirac-mcp-windows-x86_64.exe
)

for asset in "${affected_assets[@]}"; do
  require_grep "$asset" "$ZED_LIB"
  require_grep "$asset" "$RELEASE_TAG"
  if [[ "$asset" != "jirac-mcp-windows-x86_64.exe" ]]; then
    require_grep "$asset" "$RELEASE_RECOVER"
  fi
done

if [ ! -x "$SYNC_SCRIPT" ]; then
  echo "Sync script is not executable: $SYNC_SCRIPT" >&2
  exit 1
fi

echo "Zed compatibility checks passed."
