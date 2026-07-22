# Real screenshots (Docker, headless)

Captures **real PNG** images for the README **without opening your personal VS Code**.

| Output | How |
|--------|-----|
| `docs/images/cli-find.png` | **True XTerm window** under Xvfb running real `pydead find` (ImageMagick `import`) |
| `docs/images/vscode-diag.png` | **code-server** (Default Dark Modern) + real extension + fixture |
| `docs/images/vscode-quickfix.png` | Same session after **Quick Fix** (`Ctrl+.`) |

## Run (host)

```bash
# repo root — Docker only; host Code/Cursor is never launched
./scripts/screenshots/run-docker.sh
```

What it does:

1. Multi-stage Docker build: compiles Linux `pydead`, builds the extension  
2. **CLI:** starts **Xvfb + XTerm**, runs `pydead find`, screenshots the real window  
3. **VS Code:** starts **code-server** on `127.0.0.1:8080` **inside the container**  
4. **Playwright Chromium** opens the fixture, captures diagnostics / quick fix  
5. Bind-mounts results to `docs/images/` on your machine  

## Requirements

- Docker (OrbStack / Docker Desktop / Colima / engine)
- First build is large (Rust image + Playwright + code-server + X11)

## Why Docker?

Host automation with `code --extensionDevelopmentPath` steals focus and mixes with your daily editor. Everything here stays inside the container — including a headless X server for true terminal pixels (not a Pillow mockup).
