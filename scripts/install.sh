#!/usr/bin/env bash
# Install the latest pydead CLI binary from GitHub Releases.
# Does not require Rust or Node.
#
#   curl -fsSL https://raw.githubusercontent.com/ErikbStorm/pydead/main/scripts/install.sh | bash
#
# Options (env):
#   PYDEAD_VERSION   pin a tag without 'v', e.g. 0.1.0 (default: latest)
#   PYDEAD_INSTALL_DIR  install directory (default: ~/.local/bin)
#   PYDEAD_REPO      owner/name (default: ErikbStorm/pydead)

set -euo pipefail

REPO="${PYDEAD_REPO:-ErikbStorm/pydead}"
INSTALL_DIR="${PYDEAD_INSTALL_DIR:-${HOME}/.local/bin}"
VERSION="${PYDEAD_VERSION:-}"

red() { printf '\033[0;31m%s\033[0m\n' "$*" >&2; }
info() { printf '→ %s\n' "$*" >&2; }

need() {
  command -v "$1" >/dev/null 2>&1 || {
    red "missing required command: $1"
    exit 1
  }
}

need curl
need tar
need uname

os="$(uname -s | tr '[:upper:]' '[:lower:]')"
arch="$(uname -m)"

case "$os" in
  linux)  rust_os=unknown-linux-gnu ;;
  darwin) rust_os=apple-darwin ;;
  msys*|cygwin*|mingw*) red "use the Windows .zip from GitHub Releases"; exit 1 ;;
  *) red "unsupported OS: $os"; exit 1 ;;
esac

case "$arch" in
  x86_64|amd64) rust_arch=x86_64 ;;
  aarch64|arm64) rust_arch=aarch64 ;;
  *) red "unsupported arch: $arch"; exit 1 ;;
esac

target="${rust_arch}-${rust_os}"

if [[ -z "$VERSION" ]]; then
  info "resolving latest release from ${REPO}…"
  # Prefer the API; fall back to redirect for unauthenticated use
  VERSION="$(
    curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
      | sed -n 's/.*"tag_name": "v\?\([^"]*\)".*/\1/p' \
      | head -1
  )"
  if [[ -z "$VERSION" ]]; then
    # redirect: …/releases/latest → …/releases/tag/vX.Y.Z
    loc="$(curl -fsSIL "https://github.com/${REPO}/releases/latest" 2>/dev/null \
      | tr -d '\r' | sed -n 's/^[Ll]ocation: //p' | tail -1)"
    VERSION="$(printf '%s' "$loc" | sed -n 's|.*/tag/v\?\([^/]*\)$|\1|p')"
  fi
fi

if [[ -z "$VERSION" ]]; then
  red "could not determine latest version; set PYDEAD_VERSION=0.1.0"
  exit 1
fi

asset="pydead-${VERSION}-${target}.tar.gz"
url="https://github.com/${REPO}/releases/download/v${VERSION}/${asset}"

info "installing pydead v${VERSION} (${target})"
info "from ${url}"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

if ! curl -fsSL "$url" -o "${tmpdir}/${asset}"; then
  red "download failed — is release v${VERSION} published with asset ${asset}?"
  exit 1
fi

tar -xzf "${tmpdir}/${asset}" -C "$tmpdir"
mkdir -p "$INSTALL_DIR"
install -m 755 "${tmpdir}/pydead" "${INSTALL_DIR}/pydead"

info "installed ${INSTALL_DIR}/pydead"
if ! command -v pydead >/dev/null 2>&1; then
  printf '\n' >&2
  info "add to PATH, e.g.:"
  printf '  export PATH="%s:$PATH"\n' "$INSTALL_DIR" >&2
fi

"${INSTALL_DIR}/pydead" --help 2>/dev/null | head -3 || true
printf '\n✓ pydead v%s ready\n' "$VERSION" >&2
