#!/usr/bin/env python3
"""Render plain CLI text into a dark terminal-style PNG (real pydead output)."""

from __future__ import annotations

import re
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

# Catppuccin Mocha-ish
BG = (30, 30, 46)
BAR = (24, 24, 37)
TEXT = (205, 214, 244)
SUB = (166, 173, 200)
GREEN = (166, 227, 161)
PEACH = (250, 179, 135)
SKY = (137, 220, 235)
MAUVE = (203, 166, 247)
YELLOW = (249, 226, 175)
RED = (243, 139, 168)


def font(size: int) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    candidates = [
        "/System/Library/Fonts/Menlo.ttc",
        "/System/Library/Fonts/Monaco.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
    ]
    for p in candidates:
        if Path(p).exists():
            try:
                return ImageFont.truetype(p, size=size)
            except OSError:
                continue
    return ImageFont.load_default()


def colorize_line(line: str) -> list[tuple[str, tuple[int, int, int]]]:
    """Split a pydead text line into (fragment, color) runs."""
    # path:line:col: CODE kind 'name' is unused (confidence N)
    m = re.match(
        r"^(?P<path>.+?:\d+:\d+:)\s*"
        r"(?P<code>DC\d{3})\s+"
        r"(?P<kind>\w+)\s+"
        r"'(?P<name>[^']+)'\s+"
        r"(?P<rest>is unused.*)$",
        line,
    )
    if m:
        return [
            (m.group("path") + " ", SUB),
            (m.group("code") + " ", PEACH),
            (m.group("kind") + " ", SKY),
            (f"'{m.group('name')}'", MAUVE),
            (" " + m.group("rest"), SUB),
        ]
    if line.startswith("$") or line.strip().startswith("pydead "):
        return [(line, TEXT)]
    if "dead definition" in line or "file(s)" in line:
        return [(line, YELLOW)]
    if line.strip().startswith("✓") or "No dead code" in line:
        return [(line, GREEN)]
    return [(line, TEXT)]


def main() -> None:
    if len(sys.argv) < 3:
        print("usage: render_terminal.py input.txt output.png", file=sys.stderr)
        sys.exit(2)
    raw = Path(sys.argv[1]).read_text(encoding="utf-8", errors="replace")
    out_path = Path(sys.argv[2])

    body_lines = [ln.rstrip("\n") for ln in raw.splitlines()]
    # frame with prompt
    lines = ["$ pydead find fixtures/sample_project", ""] + body_lines + [""]

    fnt = font(15)
    fnt_sm = font(13)
    pad_x, pad_y = 28, 56
    line_h = 22
    # measure width
    try:
        tmp = Image.new("RGB", (10, 10))
        dr = ImageDraw.Draw(tmp)
        max_w = 0
        for ln in lines:
            bbox = dr.textbbox((0, 0), ln or " ", font=fnt)
            max_w = max(max_w, bbox[2] - bbox[0])
    except Exception:
        max_w = 900

    w = max(920, max_w + pad_x * 2)
    h = pad_y + 24 + len(lines) * line_h + 36

    img = Image.new("RGB", (w, h), BG)
    draw = ImageDraw.Draw(img)

    # title bar
    draw.rounded_rectangle((0, 0, w, 40), radius=12, fill=BAR)
    draw.rectangle((0, 20, w, 40), fill=BAR)
    for i, c in enumerate([RED, YELLOW, GREEN]):
        draw.ellipse((18 + i * 20, 12, 30 + i * 20, 24), fill=c)
    draw.text((90, 12), "pydead — real CLI output", fill=SUB, font=fnt_sm)

    y = pad_y
    for i, ln in enumerate(lines):
        if i == 0:
            draw.text((pad_x, y), "$", fill=GREEN, font=fnt)
            draw.text((pad_x + 16, y), ln[1:].lstrip() if ln.startswith("$") else ln, fill=TEXT, font=fnt)
            # fix: line already has $
            if ln.startswith("$"):
                draw.rectangle((pad_x, y, w - pad_x, y + line_h), fill=BG)  # clear
                draw.text((pad_x, y), "$", fill=GREEN, font=fnt)
                draw.text((pad_x + 18, y), ln[1:].lstrip(), fill=TEXT, font=fnt)
            y += line_h
            continue
        x = pad_x
        for frag, col in colorize_line(ln):
            draw.text((x, y), frag, fill=col, font=fnt)
            bbox = draw.textbbox((x, y), frag, font=fnt)
            x = bbox[2]
        y += line_h

    # footer
    draw.text(
        (pad_x, h - 28),
        "Captured from real pydead binary  ·  scripts/screenshots",
        fill=SUB,
        font=fnt_sm,
    )

    # rounded mask-ish border
    img.save(out_path, "PNG")
    print(f"wrote {out_path}")


if __name__ == "__main__":
    main()
