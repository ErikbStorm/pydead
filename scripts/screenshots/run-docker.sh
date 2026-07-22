#!/usr/bin/env bash
# Build a headless Docker image and capture real CLI + VS Code Web screenshots.
# Never opens or touches your personal desktop VS Code.
#
# Usage (from repo root):
#   ./scripts/screenshots/run-docker.sh
#
# Requires: Docker with buildx (or plain docker build)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

OUT_HOST="$ROOT/docs/images"
mkdir -p "$OUT_HOST"

IMAGE="${PYDEAD_SHOT_IMAGE:-pydead-screenshots:local}"

echo "==> Building image (Rust pydead + Playwright + VS Code Web)…"
echo "    This may take several minutes the first time."
docker build \
  -f scripts/screenshots/Dockerfile \
  -t "$IMAGE" \
  .

echo "==> Running capture (output → docs/images/*.png)…"
docker run --rm \
  -v "$OUT_HOST:/out" \
  -e OUT_DIR=/out \
  -e WORK_ROOT=/work \
  -e PYDEAD_BIN=/usr/local/bin/pydead \
  "$IMAGE"

echo ""
echo "Done:"
ls -la "$OUT_HOST"/*.png 2>/dev/null || true
echo ""
echo "Update README to point at .png if needed, then commit docs/images/*.png"
