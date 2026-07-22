# Real screenshots (Docker, headless)

Captures **real PNG** images for the README **without opening your personal VS Code**.

| Output | How |
|--------|-----|
| `docs/images/cli-find.png` | Real `pydead find` output, framed as a terminal |
| `docs/images/vscode-diag.png` | **code-server** (Default Dark Modern) + real extension + fixture |
| `docs/images/vscode-quickfix.png` | Same session after **Quick Fix** (`Ctrl+.`) |

## Run (host)

```bash
# repo root — Docker only; host Code/Cursor is never launched
./scripts/screenshots/run-docker.sh
```

What it does:

1. Multi-stage Docker build: compiles Linux `pydead`, builds the extension  
2. Starts **code-server** on `127.0.0.1:8080` **inside the container**  
3. **Playwright Chromium** (also in the container) logs in, opens `geo_types.py`, screenshots  
4. Bind-mounts results to `docs/images/` on your machine  

## Requirements

- Docker (OrbStack / Docker Desktop / Colima / engine)
- First build is large (Rust image + Playwright + code-server)

## Optional: CLI-only on host (no Docker)

```bash
cargo build -p pydead --release
./target/release/pydead find fixtures/sample_project > /tmp/out.txt
python3 scripts/screenshots/render_terminal.py /tmp/out.txt docs/images/cli-find.png
```

## Why Docker?

Host automation with `code --extensionDevelopmentPath` steals focus and mixes with your daily editor. Everything here stays inside the container.
