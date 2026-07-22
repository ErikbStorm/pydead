#!/usr/bin/env bash
# Install the latest pydead CLI binary from GitHub Releases.
# Does not require Rust or Node.
#
#   curl -fsSL https://raw.githubusercontent.com/ErikbStorm/pydead/main/scripts/install.sh | bash
#
# Options (env):
#   PYDEAD_VERSION      pin a tag without 'v', e.g. 0.1.0 (default: latest)
#   PYDEAD_INSTALL_DIR  install directory (default: ~/.local/bin)
#   PYDEAD_REPO         owner/name (default: ErikbStorm/pydead)
#   PYDEAD_VERIFY       set to 0 to skip SHA-256 check (default: 1)

set -euo pipefail

REPO="${PYDEAD_REPO:-ErikbStorm/pydead}"
INSTALL_DIR="${PYDEAD_INSTALL_DIR:-${HOME}/.local/bin}"
VERSION="${PYDEAD_VERSION:-}"
VERIFY="${PYDEAD_VERIFY:-1}"

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
  api_json="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null || true)"
  if [[ -n "$api_json" ]]; then
    VERSION="$(printf '%s' "$api_json" | sed -n 's/.*"tag_name": *"v*\([^"]*\)".*/\1/p' | head -1)"
  fi
  if [[ -z "$VERSION" ]]; then
    loc="$(curl -fsSIL "https://github.com/${REPO}/releases/latest" 2>/dev/null \
      | tr -d '\r' | sed -n 's/^[Ll]ocation: //p' | tail -1 || true)"
    VERSION="$(printf '%s' "$loc" | sed -n 's|.*/tag/v*\([^/]*\)$|\1|p')"
  fi
fi

VERSION="${VERSION#v}"

if [[ -z "$VERSION" ]]; then
  red "could not determine latest version; set PYDEAD_VERSION=0.1.0"
  exit 1
fi

if [[ "$target" == "aarch64-unknown-linux-gnu" ]]; then
  red "no prebuilt binary for ${target} yet; use cargo from source or an x86_64 host"
  exit 1
fi

asset="pydead-${VERSION}-${target}.tar.gz"
base_url="https://github.com/${REPO}/releases/download/v${VERSION}"
url="${base_url}/${asset}"
sums_url="${base_url}/SHA256SUMS"

info "installing pydead v${VERSION} (${target})"
info "from ${url}"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

if ! curl -fsSL "$url" -o "${tmpdir}/${asset}"; then
  red "download failed — is release v${VERSION} published with asset ${asset}?"
  exit 1
fi

sha256_file() {
  local f="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$f" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$f" | awk '{print $1}'
  else
    red "need sha256sum or shasum to verify download (or set PYDEAD_VERIFY=0)"
    exit 1
  fi
}

if [[ "$VERIFY" != "0" ]]; then
  info "verifying SHA-256 against ${sums_url}"
  if ! curl -fsSL "$sums_url" -o "${tmpdir}/SHA256SUMS"; then
    red "could not download SHA256SUMS for v${VERSION}"
    red "refusing to install without integrity data (set PYDEAD_VERIFY=0 to override)"
    exit 1
  fi
  # Lines look like: <hex>  <filename>   or   <hex> *<filename>
  expected="$(
    awk -v a="$asset" '
      $2 == a || $2 == ("*" a) || $2 == ("./" a) { print $1; exit }
    ' "${tmpdir}/SHA256SUMS"
  )"
  if [[ -z "$expected" ]]; then
    red "asset ${asset} not listed in SHA256SUMS"
    exit 1
  fi
  actual="$(sha256_file "${tmpdir}/${asset}")"
  if [[ "$actual" != "$expected" ]]; then
    red "SHA-256 mismatch for ${asset}"
    red "  expected: ${expected}"
    red "  actual:   ${actual}"
    exit 1
  fi
  info "checksum OK (${actual:0:12}…)"
else
  info "skipping checksum (PYDEAD_VERIFY=0)"
fi

tar -xzf "${tmpdir}/${asset}" -C "$tmpdir"
if [[ ! -f "${tmpdir}/pydead" ]]; then
  red "archive did not contain pydead binary"
  exit 1
fi
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
