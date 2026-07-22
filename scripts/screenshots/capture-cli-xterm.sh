#!/usr/bin/env bash
# True terminal capture: real XTerm window under Xvfb, screenshot with ImageMagick.
# Runs inside the screenshot Docker image — not a Pillow mockup.
#
# Usage:
#   OUT_DIR=/out WORK_ROOT=/work PYDEAD_BIN=pydead ./capture-cli-xterm.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORK="${WORK_ROOT:-/work}"
OUT="${OUT_DIR:-/out}"
PYDEAD="${PYDEAD_BIN:-pydead}"
SAMPLE="${SAMPLE_PROJECT:-$WORK/fixtures/sample_project}"
DISPLAY_NUM="${DISPLAY_NUM:-99}"
export DISPLAY=":${DISPLAY_NUM}"

PNG="${OUT}/cli-find.png"
RAW="${OUT}/cli-find.raw.txt"

mkdir -p "$OUT"

if [[ ! -x "$PYDEAD" ]] && ! command -v "$PYDEAD" >/dev/null 2>&1; then
  echo "pydead binary not found: $PYDEAD" >&2
  exit 1
fi

# Always keep a text dump of real output for debugging / diffs
"$PYDEAD" find "$SAMPLE" >"$RAW"

need() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required tool: $1" >&2
    exit 1
  }
}
need Xvfb
need xterm
need xdotool
need import

# Clean previous Xvfb on this display if any
if [[ -e "/tmp/.X${DISPLAY_NUM}-lock" ]]; then
  pkill -f "Xvfb :${DISPLAY_NUM}" 2>/dev/null || true
  rm -f "/tmp/.X${DISPLAY_NUM}-lock"
fi

echo "→ CLI: true XTerm capture under Xvfb :${DISPLAY_NUM}"

Xvfb ":${DISPLAY_NUM}" -screen 0 1600x1000x24 -ac +extension RANDR -nolisten tcp >/tmp/xvfb.log 2>&1 &
XVFB_PID=$!
cleanup() {
  kill "$XTERM_PID" 2>/dev/null || true
  kill "$XVFB_PID" 2>/dev/null || true
  wait "$XTERM_PID" 2>/dev/null || true
  wait "$XVFB_PID" 2>/dev/null || true
}
trap cleanup EXIT

# Wait for X
for _ in $(seq 1 50); do
  if xdpyinfo -display "$DISPLAY" >/dev/null 2>&1; then
    break
  fi
  sleep 0.1
done
xdpyinfo -display "$DISPLAY" >/dev/null

# Dark xterm that still looks like a real terminal (not a marketing mock)
# -fs: font size; geometry: cols x rows
xterm \
  -display "$DISPLAY" \
  -geometry 115x32+48+48 \
  -fa "DejaVu Sans Mono" \
  -fs 12 \
  -bg "#11111b" \
  -fg "#cdd6f4" \
  -cr "#cdd6f4" \
  +sb \
  -title "Terminal" \
  -xrm "XTerm*faceName: DejaVu Sans Mono" \
  -xrm "XTerm*faceSize: 12" \
  -xrm "XTerm*allowBoldFonts: true" \
  -xrm "XTerm*foreground: #cdd6f4" \
  -xrm "XTerm*background: #11111b" \
  -xrm "XTerm*cursorColor: #cdd6f4" \
  -xrm "XTerm*colorBD: #ffffff" \
  -e bash -lc "
    set -e
    export TERM=xterm-256color
    cd $(printf '%q' "$WORK")
    clear
    # Show the prompt + command the user would type, then real binary output
    printf '%s\n' \"\$ pydead find fixtures/sample_project\"
    $(printf '%q' "$PYDEAD") find $(printf '%q' "$SAMPLE")
    printf '\n'
    # Hold the window open long enough for import(1)
    sleep 12
  " &
XTERM_PID=$!

# Find the XTerm window id
WIN=""
for _ in $(seq 1 80); do
  # class is usually XTerm / xterm
  WIN="$(xdotool search --onlyvisible --class XTerm 2>/dev/null | head -1 || true)"
  if [[ -z "$WIN" ]]; then
    WIN="$(xdotool search --onlyvisible --name Terminal 2>/dev/null | head -1 || true)"
  fi
  if [[ -n "$WIN" ]]; then
    break
  fi
  sleep 0.15
done

if [[ -z "$WIN" ]]; then
  echo "XTerm window not found; Xvfb log:" >&2
  cat /tmp/xvfb.log >&2 || true
  exit 1
fi

# Wait until pydead has printed (raw file already written; give UI time to paint)
sleep 2.5

# Screenshot just the terminal window (true pixels — keep full window chrome)
import -display "$DISPLAY" -window "$WIN" "$PNG"

# Slight sharpen/resize-safe copy only; do NOT -trim (that crops real decorations)
if command -v convert >/dev/null 2>&1; then
  convert "$PNG" -strip PNG32:"$PNG" || true
fi

if [[ ! -s "$PNG" ]]; then
  echo "failed to write $PNG" >&2
  exit 1
fi

echo "  wrote $PNG ($(wc -c <"$PNG") bytes) from XTerm window $WIN"
