#!/usr/bin/env python3
"""
Generate README visuals for PyDead (SVG mockups — exact text, crisp on GitHub).

Why SVG (not screenshot bots / AI image models)?
  - Exact monospaced text, rule codes, and UI labels
  - Regeneratable from this script, no GUI tooling
  - Looks great in light/dark GitHub READMEs

Outputs (docs/images/):
  cli-find.svg         — colorful terminal: pydead find
  vscode-diag.svg      — VS Code unused highlights + problems + status
  vscode-quickfix.svg  — Quick Fix menu (keep / report / remove)

Usage:
  python3 scripts/generate_readme_assets.py
"""

from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "docs" / "images"

MONO = "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', monospace"


def esc(s: str) -> str:
    return (
        s.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace('"', "&quot;")
    )


def text(
    x: float,
    y: float,
    content: str,
    fill: str,
    size: float = 13,
    weight: str = "500",
    **attrs: str,
) -> str:
    extra = " ".join(f'{k.replace("_", "-")}="{v}"' for k, v in attrs.items())
    return (
        f'<text x="{x}" y="{y}" fill="{fill}" font-family="{MONO}" '
        f'font-size="{size}" font-weight="{weight}" {extra}>{esc(content)}</text>'
    )


def tspan(content: str, fill: str, weight: str = "500") -> str:
    return f'<tspan fill="{fill}" font-weight="{weight}">{esc(content)}</tspan>'


def write(name: str, svg: str) -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    path = OUT / name
    path.write_text(svg.strip() + "\n", encoding="utf-8")
    print(f"wrote {path.relative_to(ROOT)}")


def cli_find() -> str:
    """Catppuccin-style terminal showing cross-file dead-code findings."""
    w, h = 940, 400
    findings = [
        ("apps/greeter/unused_app.py:4:1:", "DC001", "function", "never_called"),
        ("apps/greeter/unused_app.py:8:1:", "DC001", "function", "also_never_called"),
        ("libs/core/api.py:16:5:", "DC003", "method", "unused_method"),
        ("libs/core/api.py:21:1:", "DC002", "class", "DeadService"),
        ("libs/core/api.py:28:1:", "DC001", "function", "orphan_public"),
        ("libs/core/util.py:4:1:", "DC004", "variable", "UNUSED_CONSTANT"),
        ("libs/plugins/legacy.py:4:1:", "DC002", "class", "LegacyPlugin"),
    ]

    # palette
    bg0, bg1, bar = "#1e1e2e", "#11111b", "#181825"
    green, text_c, sub = "#a6e3a1", "#cdd6f4", "#a6adc8"
    peach, sky, mauve, yellow = "#fab387", "#89dceb", "#cba6f7", "#f9e2af"

    lines = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">',
        f"<defs><linearGradient id=\"tg\" x1=\"0\" y1=\"0\" x2=\"0.2\" y2=\"1\">"
        f'<stop offset="0%" stop-color="{bg0}"/><stop offset="100%" stop-color="{bg1}"/>'
        f"</linearGradient></defs>",
        f'<rect width="{w}" height="{h}" rx="14" fill="url(#tg)"/>',
        # window chrome
        f'<rect width="{w}" height="40" rx="14" fill="{bar}"/>',
        f'<rect y="26" width="{w}" height="14" fill="{bar}"/>',
        '<circle cx="24" cy="20" r="6" fill="#f38ba8"/>',
        '<circle cx="44" cy="20" r="6" fill="#f9e2af"/>',
        '<circle cx="64" cy="20" r="6" fill="#a6e3a1"/>',
        text(92, 25, "pydead — cross-file dead code", sub, 12),
        # prompt
        text(28, 72, "$", green, 14, "700"),
        text(46, 72, "pydead find fixtures/sample_project", text_c, 14),
    ]

    y = 104
    for path_s, code, kind, name in findings:
        # one line with colored segments via nested tspans in a single text
        line = (
            f'<text x="28" y="{y}" font-family="{MONO}" font-size="12.5">'
            f"{tspan(path_s + '  ', sub)}"
            f"{tspan(code + '  ', peach, '700')}"
            f"{tspan(kind + ' ', sky)}"
            f"{tspan(chr(39) + name + chr(39), mauve, '700')}"
            f"{tspan(' is unused  ', sub)}"
            f"{tspan('(confidence 70)', sub)}"
            f"</text>"
        )
        lines.append(line)
        y += 26

    y += 6
    lines.append(
        text(
            28,
            y,
            "13 dead definition(s) in 8 file(s) (21 definitions scanned).",
            yellow,
            13,
            "600",
        )
    )
    y += 28
    lines.append(
        text(
            28,
            y,
            "✓ Cross-package Greeter kept live  ·  EP004 Azure · EP005 Alembic · EP006 Pydantic · EP007 SQLAlchemy",
            green,
            12,
        )
    )
    lines.append("</svg>")
    return "\n".join(lines)


def vscode_diag() -> str:
    """VS Code-like editor with unused-code highlights and PyDead status."""
    w, h = 980, 540
    editor = "#1e1e1e"
    sidebar = "#252526"
    activity = "#333333"
    tab = "#2d2d2d"
    status = "#007acc"
    line_num = "#858585"
    kw, cd, st, cm = "#569cd6", "#d4d4d4", "#ce9178", "#6a9955"
    unused = "#f48771"

    # (line_no, list of (color, text), unused?)
    rows: list[tuple[int, list[tuple[str, str]], bool]] = [
        (1, [(kw, "from"), (cd, " sqlalchemy.types "), (kw, "import"), (cd, " TypeDecorator")], False),
        (2, [], False),
        (3, [(kw, "class"), (cd, " BytesGeometry(TypeDecorator):")], False),
        (4, [(cd, "    impl = Geometry")], False),
        (5, [(cd, "    cache_ok = True")], False),
        (6, [], False),
        (7, [(cd, "    "), (kw, "def"), (cd, " load_dialect_impl(self, dialect):")], False),
        (8, [(cd, "        "), (kw, "return"), (cd, " Geometry()  "), (cm, "# EP007 · live")], False),
        (9, [], False),
        (10, [(cd, "    "), (kw, "def"), (cd, " bind_expression(self, bindvalue):")], False),
        (11, [(cd, "        "), (kw, "return"), (cd, " …  "), (cm, "# EP007 · live")], False),
        (12, [], False),
        (13, [(cd, "    "), (kw, "def"), (cd, " never_used_method(self):")], True),
        (14, [(cd, "        "), (kw, "return"), (st, ' "dead"')], True),
        (15, [], False),
        (16, [(kw, "def"), (cd, " leftover_helper():")], True),
        (17, [(cd, "    "), (kw, "pass"), (cm, "  # DC001")], True),
    ]

    out = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">',
        f'<rect width="{w}" height="{h}" rx="10" fill="{editor}"/>',
        f'<rect width="48" height="{h}" fill="{activity}"/>',
        f'<rect x="48" width="210" height="{h}" fill="{sidebar}"/>',
        text(64, 34, "EXPLORER", "#bbbbbb", 11, "700"),
        text(64, 62, "▼  geo", "#cccccc", 12),
        text(80, 86, "geo_types.py", "#ffffff", 12, "600"),
        text(80, 110, "models.py", "#cccccc", 12),
        f'<rect x="258" width="{w - 258}" height="36" fill="{tab}"/>',
        text(278, 24, "geo_types.py", "#ffffff", 12),
        f'<rect x="258" y="36" width="{w - 258}" height="{h - 64}" fill="{editor}"/>',
        f'<rect x="258" y="36" width="48" height="{h - 64}" fill="{editor}"/>',
    ]

    y0 = 60
    for i, (lnum, segs, is_unused) in enumerate(rows):
        y = y0 + i * 22
        if is_unused:
            out.append(
                f'<rect x="306" y="{y - 15}" width="{w - 320}" height="20" '
                f'fill="rgba(244,135,113,0.14)"/>'
            )
        out.append(text(270, y, f"{lnum:>2}", line_num, 12))
        x = 314.0
        for fill, t in segs:
            deco = ' text-decoration="line-through"' if is_unused and fill != cm else ""
            col = unused if is_unused and fill != cm else fill
            out.append(
                f'<text x="{x}" y="{y}" fill="{col}" font-family="{MONO}" font-size="13"{deco}>'
                f"{esc(t)}</text>"
            )
            x += len(t) * 7.85
        if is_unused and segs:
            # error squiggle
            out.append(
                f'<path d="M314,{y + 4} q2.5,2.5 5,0 t5,0 t5,0 t5,0 t5,0 t5,0 t5,0 t5,0 t5,0 t5,0 t5,0 t5,0 t5,0 t5,0 t5,0 t5,0 t5,0" '
                f'stroke="{unused}" fill="none" stroke-width="1.15"/>'
            )

    # problems strip
    out.append(f'<rect x="258" y="{h - 92}" width="{w - 258}" height="64" fill="#252526"/>')
    out.append(text(274, h - 70, "PROBLEMS", "#bbbbbb", 11, "700"))
    out.append(
        text(
            274,
            h - 48,
            "⚠  DC003  Method 'never_used_method' is never referenced in the workspace",
            unused,
            12,
        )
    )
    out.append(
        text(
            274,
            h - 30,
            "⚠  DC001  Function 'leftover_helper' is never referenced in the workspace",
            unused,
            12,
        )
    )

    out.append(f'<rect y="{h - 28}" width="{w}" height="28" fill="{status}"/>')
    out.append(text(60, h - 10, "Python  3.12.0", "#ffffff", 11))
    out.append(text(w - 210, h - 10, "⚠ PyDead: 2", "#ffffff", 11, "600"))
    out.append(text(w - 90, h - 10, "Ln 13", "#ffffff", 11))
    out.append("</svg>")
    return "\n".join(out)


def vscode_quickfix() -> str:
    """Quick Fix palette for keep / report / remove."""
    w, h = 700, 360
    bg, menu, border, accent = "#1e1e1e", "#252526", "#454545", "#0e639c"
    text_c, dim, green, unused = "#cccccc", "#858585", "#89d185", "#f48771"

    return "\n".join(
        [
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">',
            f'<rect width="{w}" height="{h}" rx="10" fill="{bg}"/>',
            text(24, 32, "Quick Fix  ·  ⌘.  /  Ctrl+.", dim, 12),
            text(24, 68, "def leftover_helper():", "#d4d4d4", 15),
            f'<path d="M24,74 q3,2.5 6,0 t6,0 t6,0 t6,0 t6,0 t6,0 t6,0 t6,0 t6,0 t6,0 t6,0 t6,0 t6,0 t6,0 t6,0" '
            f'stroke="{unused}" fill="none" stroke-width="1.4"/>',
            text(24, 100, "    pass", "#d4d4d4", 15),
            text(24, 138, "💡  PyDead actions", dim, 13),
            f'<rect x="40" y="152" width="620" height="176" rx="6" fill="{menu}" stroke="{border}"/>',
            f'<rect x="44" y="160" width="612" height="34" rx="4" fill="{accent}"/>',
            text(58, 182, "PyDead: keep 'leftover_helper' (mark as used)", "#ffffff", 13, "600"),
            text(58, 220, "PyDead: keep (DC001 only)", text_c, 13),
            text(58, 252, "PyDead: report false positive for 'leftover_helper'…", text_c, 13),
            text(58, 284, "PyDead: remove unused function 'leftover_helper'", text_c, 13),
            text(58, 316, "→ inserts  # pydead: keep   ·   opens GitHub issue prefilled", green, 12),
            "</svg>",
        ]
    )


def main() -> None:
    write("cli-find.svg", cli_find())
    write("vscode-diag.svg", vscode_diag())
    write("vscode-quickfix.svg", vscode_quickfix())
    print("Done → docs/images/")


if __name__ == "__main__":
    main()
