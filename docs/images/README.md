# README visuals

**PNG screenshots** used on the project README (real CLI + real VS Code via Docker).

## Regenerate (recommended)

```bash
# Never opens your desktop VS Code
./scripts/screenshots/run-docker.sh
```

| File | Source |
|------|--------|
| `cli-find.png` | Real `pydead find` output (framed as terminal) |
| `vscode-diag.png` | code-server + real extension + fixture (Playwright) |
| `vscode-quickfix.png` | Same session, Quick Fix lightbulb |

Optional SVG mockups (`*.svg`) can still be built with `python3 scripts/generate_readme_assets.py` for offline previews without Docker.
