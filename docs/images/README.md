# README visuals

**PNG assets** used for branding and README screenshots.

## Icons

| File | Description |
|------|-------------|
| `icon.png` | Primary logo (soft shading) on dark tile, 2048×2048 |
| `icon-flat.png` | Flatter solid-color variant on dark tile, 2048×2048 |
| `icon-transparent.png` | Primary logo, **true transparent PNG** (no plate) |
| `icon-flat-transparent.png` | Flat logo, **true transparent PNG** (no plate) |

Size variants for the extension live under `vscode-extension/media/`:

- `icon-{128,1024,2048}.png`, `icon-flat-{128,1024,2048}.png`
- `icon-transparent-{128,1024,2048}.png`, `icon-flat-transparent-{128,1024,2048}.png`

**License:** icon artwork is **CC BY 4.0** — see [`LICENSE`](LICENSE).  
(Source code remains MIT; see the repository root `LICENSE`.)

Marketplace / VSIX uses `vscode-extension/icon.png` (primary). The flat variant is available for docs, dark UIs, or alternate branding.

## Screenshots

| File | Source |
|------|--------|
| `cli-find.png` | True XTerm window (Xvfb) running real `pydead find` |
| `vscode-diag.png` | code-server (Default Dark Modern) + real extension + fixture |
| `vscode-quickfix.png` | Same session, Quick Fix lightbulb |

### Regenerate screenshots

```bash
# Never opens your desktop VS Code — fully isolated Docker capture
./scripts/screenshots/run-docker.sh
```
