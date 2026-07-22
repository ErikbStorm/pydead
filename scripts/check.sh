#!/usr/bin/env bash
# Local / pre-commit checks aligned with .github/workflows/ci.yml
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

SKIP_EXTENSION="${SKIP_EXTENSION:-0}"
SKIP_TESTS="${SKIP_TESTS:-0}"
FIX_FMT="${FIX_FMT:-0}"

red() { printf '\033[0;31m%s\033[0m\n' "$*"; }
green() { printf '\033[0;32m%s\033[0m\n' "$*"; }
step() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }

fail() {
  red "✗ $*"
  exit 1
}

step "cargo fmt"
if [[ "$FIX_FMT" == "1" ]]; then
  cargo fmt --all
else
  cargo fmt --all -- --check || fail "Format check failed. Run: cargo fmt --all  (or FIX_FMT=1 scripts/check.sh)"
fi
green "fmt ok"

step "cargo clippy -D warnings"
cargo clippy -p pydead --all-targets -- -D warnings || fail "Clippy failed"
green "clippy ok"

if [[ "$SKIP_TESTS" != "1" ]]; then
  step "cargo test --workspace"
  cargo test --workspace || fail "Tests failed"
  green "tests ok"
else
  step "cargo test (skipped — SKIP_TESTS=1)"
fi

if [[ "$SKIP_EXTENSION" == "1" ]]; then
  step "vscode-extension compile (skipped — SKIP_EXTENSION=1)"
else
  step "vscode-extension compile"
  if ! command -v npm >/dev/null 2>&1; then
    red "npm not found; set SKIP_EXTENSION=1 to skip, or install Node.js"
    exit 1
  fi
  (
    cd vscode-extension
    if [[ ! -d node_modules ]]; then
      npm install --no-fund --no-audit
    fi
    npm run compile
  ) || fail "Extension TypeScript compile failed"
  green "extension ok"
fi

printf '\n'
green "All checks passed (same gates as CI)."
