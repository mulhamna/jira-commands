#!/usr/bin/env bash
# install.sh — one-line installer for jirac or jirac-mcp
#
# Usage:
#   curl -sSL https://raw.githubusercontent.com/mulhamna/jira-commands/main/install.sh | bash
#
# What it does:
#   1. Detects your OS and architecture
#   2. Downloads the latest binary from GitHub Releases
#   3. Verifies the SHA-256 checksum
#   4. Installs to ~/.local/bin (or /usr/local/bin if writable and ~/.local/bin not in PATH)
#
# jirac and jirac-mcp are independent tools for the Jira ecosystem.
# It is not affiliated with or endorsed by Atlassian.

set -euo pipefail

REPO="mulhamna/jira-commands"
BINARY="${BINARY:-jirac}"

# ── Colors ────────────────────────────────────────────────────────────────────
if [ -t 1 ]; then
  RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
  BOLD='\033[1m'; RESET='\033[0m'
else
  RED=''; GREEN=''; YELLOW=''; BOLD=''; RESET=''
fi

info()    { echo -e "${GREEN}==>${RESET} ${BOLD}$*${RESET}"; }
warn()    { echo -e "${YELLOW}warning:${RESET} $*"; }
error()   { echo -e "${RED}error:${RESET} $*" >&2; exit 1; }

# ── Detect OS + arch ──────────────────────────────────────────────────────────
detect_platform() {
  local os arch

  case "$BINARY" in
    jirac|jirac-mcp) ;;
    *)
      error "Unsupported BINARY='${BINARY}'. Use 'jirac' or 'jirac-mcp'."
      ;;
  esac

  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Linux)
      case "$arch" in
        x86_64)           echo "${BINARY}-linux-x86_64" ;;
        aarch64|arm64)    echo "${BINARY}-linux-aarch64" ;;
        *)                error "Unsupported Linux architecture: $arch" ;;
      esac
      ;;
    Darwin)
      case "$arch" in
        x86_64)           echo "${BINARY}-macos-x86_64" ;;
        arm64)            echo "${BINARY}-macos-aarch64" ;;
        *)                error "Unsupported macOS architecture: $arch" ;;
      esac
      ;;
    *)
      error "Unsupported OS: $os. For Windows, download the binary from https://github.com/${REPO}/releases"
      ;;
  esac
}

# ── Resolve install dir ────────────────────────────────────────────────────────
resolve_install_dir() {
  # Prefer ~/.local/bin (XDG, no sudo required)
  local local_bin="$HOME/.local/bin"
  if [ -d "$local_bin" ] || mkdir -p "$local_bin" 2>/dev/null; then
    echo "$local_bin"
    return
  fi

  # Fall back to /usr/local/bin if writable
  if [ -w "/usr/local/bin" ]; then
    echo "/usr/local/bin"
    return
  fi

  error "Cannot find a writable install directory. Try: sudo mkdir -p /usr/local/bin && sudo chmod 755 /usr/local/bin"
}

# ── Check dependencies ─────────────────────────────────────────────────────────
check_deps() {
  local missing=()
  for cmd in curl sha256sum; do
    command -v "$cmd" &>/dev/null || missing+=("$cmd")
  done

  # macOS ships shasum instead of sha256sum
  if ! command -v sha256sum &>/dev/null && command -v shasum &>/dev/null; then
    SHA_CMD="shasum -a 256"
  elif command -v sha256sum &>/dev/null; then
    SHA_CMD="sha256sum"
  else
    missing+=("sha256sum or shasum")
  fi

  if [ ${#missing[@]} -gt 0 ]; then
    error "Missing required tools: ${missing[*]}"
  fi
}

# ── Fetch latest release tag ───────────────────────────────────────────────────
fetch_latest_tag() {
  local tag
  tag="$(curl -sSf "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' \
    | sed 's/.*"tag_name": "\([^"]*\)".*/\1/')"

  [ -n "$tag" ] || error "Could not fetch latest release tag from GitHub API."
  echo "$tag"
}

# ── Main ──────────────────────────────────────────────────────────────────────
main() {
  echo ""
  echo -e "${BOLD}${BINARY} installer${RESET}"
  echo "  Jira tooling for terminals and MCP clients (not affiliated with Atlassian)"
  echo ""

  check_deps

  local platform
  platform="$(detect_platform)"
  info "Detected platform: $platform"

  local tag
  tag="$(fetch_latest_tag)"
  info "Latest release: $tag"

  local base_url="https://github.com/${REPO}/releases/download/${tag}"
  local binary_url="${base_url}/${platform}"
  local checksums_url="${base_url}/checksums.txt"

  local install_dir
  install_dir="$(resolve_install_dir)"
  local install_path="${install_dir}/${BINARY}"

  local tmp_dir
  tmp_dir="$(mktemp -d)"
  trap 'rm -rf "$tmp_dir"' EXIT

  # Download binary
  info "Downloading ${platform}..."
  curl -sSfL "$binary_url" -o "${tmp_dir}/${BINARY}"

  # Download and verify checksum
  info "Verifying checksum..."
  curl -sSfL "$checksums_url" -o "${tmp_dir}/checksums.txt"

  local expected_sha
  expected_sha="$(grep "${platform}" "${tmp_dir}/checksums.txt" | awk '{print $1}')"

  if [ -z "$expected_sha" ]; then
    error "Could not find checksum for ${platform} in checksums.txt"
  fi

  local actual_sha
  actual_sha="$($SHA_CMD "${tmp_dir}/${BINARY}" | awk '{print $1}')"

  if [ "$expected_sha" != "$actual_sha" ]; then
    error "Checksum mismatch!\n  expected: $expected_sha\n  got:      $actual_sha"
  fi
  info "Checksum verified."

  # Install
  chmod +x "${tmp_dir}/${BINARY}"
  mv "${tmp_dir}/${BINARY}" "$install_path"

  echo ""
  echo -e "${GREEN}${BINARY} ${tag} installed to ${install_path}${RESET}"

  # PATH check
  if ! echo "$PATH" | tr ':' '\n' | grep -qx "$install_dir"; then
    echo ""
    warn "'${install_dir}' is not in your PATH."
    echo "  Add this to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
    echo ""
    echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo ""
  fi

  if [ "$BINARY" = "jirac" ]; then
    echo "  Run: jirac auth login"
  else
    echo "  Run: jirac-mcp serve --transport stdio"
  fi
  echo "  Docs: https://github.com/${REPO}"
  echo ""

  # Migration note for existing 'jira' users
  if command -v jira &>/dev/null; then
    warn "You have an existing 'jira' command in your PATH."
    echo "  If it is the old jira-commands binary, you can remove it:"
    echo "    rm \"\$(which jira)\""
  fi
}

main "$@"
