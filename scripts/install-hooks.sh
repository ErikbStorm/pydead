#!/usr/bin/env bash
# Point this clone at .githooks/ so pre-commit runs local CI gates.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

chmod +x "$ROOT/scripts/check.sh" "$ROOT/.githooks/pre-commit" "$ROOT/scripts/install-hooks.sh"

git config core.hooksPath .githooks

echo "Installed git hooks (core.hooksPath=.githooks)"
echo "  pre-commit → scripts/check.sh (fmt, clippy, tests; extension if TS staged)"
echo ""
echo "Run checks anytime:  ./scripts/check.sh"
echo "Auto-fix fmt:       FIX_FMT=1 ./scripts/check.sh"
echo "Skip tests once:     SKIP_TESTS=1 git commit ..."
echo "Skip all hooks:      SKIP_HOOKS=1 git commit ..."
