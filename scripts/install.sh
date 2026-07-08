#!/bin/sh
# gitid installer for Linux and macOS.
#
#   curl -fsSL https://raw.githubusercontent.com/dgreco-at-speer/gitid/main/scripts/install.sh | sh
#
# Environment overrides:
#   GITID_REPO         owner/repo to install from (default below)
#   GITID_VERSION      release tag to install (default: latest release)
#   GITID_INSTALL_DIR  where to put the binary (default: ~/.local/bin)
#   GH_TOKEN / GITHUB_TOKEN   token for private-repo downloads (curl fallback)
#
# Private repos: install `gh` and run `gh auth login` first — this script will
# use it automatically and handle auth. Linux ships prebuilt binaries; macOS has
# no prebuilt binary, so the script builds from source with cargo.
set -eu

REPO="${GITID_REPO:-dgreco-at-speer/gitid}"
INSTALL_DIR="${GITID_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${GITID_VERSION:-}"
TOKEN="${GH_TOKEN:-${GITHUB_TOKEN:-}}"

say() { printf '%s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }
err() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}
have() { command -v "$1" >/dev/null 2>&1; }

# --- detect target -----------------------------------------------------------
os="$(uname -s)"
arch="$(uname -m)"
case "$arch" in
  x86_64 | amd64) arch=x86_64 ;;
  aarch64 | arm64) arch=aarch64 ;;
  *) err "unsupported architecture: $arch" ;;
esac

# --- macOS: build from source (no prebuilt darwin binary) --------------------
if [ "$os" = "Darwin" ]; then
  have cargo || err "macOS has no prebuilt binary and cargo was not found; install Rust from https://rustup.rs and re-run"
  say "No prebuilt macOS binary — building from source with cargo…"
  if [ -n "$VERSION" ]; then
    cargo install --git "https://github.com/$REPO" --tag "$VERSION" gitid
  else
    cargo install --git "https://github.com/$REPO" gitid
  fi
  say "Installed gitid via cargo. Next: run 'gitid setup' to enable the shell hook."
  exit 0
fi

[ "$os" = "Linux" ] || err "unsupported OS: $os — on Windows use scripts/install.ps1"
# Prefer the statically-linked musl build (runs on any libc), falling back to
# the glibc build for older releases that only shipped gnu.
target="${arch}-unknown-linux-musl"
fallback="${arch}-unknown-linux-gnu"
ext="tar.gz"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# --- download the release archive --------------------------------------------
if have gh; then
  say "Downloading gitid (${VERSION:-latest}) for $target via gh…"
  # shellcheck disable=SC2086
  gh release download ${VERSION:+"$VERSION"} \
    --repo "$REPO" \
    --pattern "gitid-*-${target}.${ext}" \
    --dir "$tmp" 2>/dev/null \
    || {
      say "No $target asset; trying $fallback…"
      # shellcheck disable=SC2086
      gh release download ${VERSION:+"$VERSION"} \
        --repo "$REPO" \
        --pattern "gitid-*-${fallback}.${ext}" \
        --dir "$tmp"
    }
else
  have curl || err "need either gh or curl installed"
  if [ -z "$VERSION" ]; then
    say "Resolving latest release…"
    api="$(curl -fsSL ${TOKEN:+-H "Authorization: Bearer $TOKEN"} \
      "https://api.github.com/repos/$REPO/releases/latest")" \
      || err "could not query releases (private repo? install gh, or set GH_TOKEN)"
    VERSION="$(printf '%s' "$api" | grep -o '"tag_name"[ ]*:[ ]*"[^"]*"' | head -1 | sed 's/.*"\([^"]*\)"$/\1/')"
    [ -n "$VERSION" ] || err "could not determine latest version"
  fi
  asset="gitid-${VERSION}-${target}.${ext}"
  url="https://github.com/$REPO/releases/download/$VERSION/$asset"
  say "Downloading $asset…"
  if ! curl -fSL ${TOKEN:+-H "Authorization: Bearer $TOKEN"} "$url" -o "$tmp/$asset" 2>/dev/null; then
    asset="gitid-${VERSION}-${fallback}.${ext}"
    url="https://github.com/$REPO/releases/download/$VERSION/$asset"
    say "No $target asset; downloading $asset…"
    curl -fSL ${TOKEN:+-H "Authorization: Bearer $TOKEN"} "$url" -o "$tmp/$asset" \
      || err "download failed (private repo? install gh and run 'gh auth login')"
  fi
fi

# --- install -----------------------------------------------------------------
archive="$(find "$tmp" -name '*.tar.gz' -type f | head -1)"
[ -n "$archive" ] || err "no archive downloaded"
tar -xzf "$archive" -C "$tmp"
bin="$(find "$tmp" -name gitid -type f | head -1)"
[ -n "$bin" ] || err "binary not found in archive"

mkdir -p "$INSTALL_DIR"
install -m 0755 "$bin" "$INSTALL_DIR/gitid"
say "Installed gitid to $INSTALL_DIR/gitid"
"$INSTALL_DIR/gitid" --version || true

case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *) warn "$INSTALL_DIR is not on your PATH — add it, e.g. 'export PATH=\"$INSTALL_DIR:\$PATH\"'" ;;
esac

say ""
say "Next: run 'gitid setup' to install the shell hook, then 'gitid add <name>' to create a profile."
