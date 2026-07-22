# PyDead — VS Code extension

Highlights **unused Python functions, classes, methods, and module-level variables** across the **whole workspace folder** (cross-file), using the `pydead` Rust analyzer.

## Features

- Diagnostics (Hint by default) on dead definitions
- Status bar count
- **Quick Fix (lightbulb / `Cmd+.`)** — only the common pair:
  - **Keep** — inserts `# pydead: keep` (preferred)
  - **Remove** — deletes this definition
- **Command Palette / right-click** for everything else:
  - **PyDead: Keep (this rule code only)** — `# pydead: keep DCxxx`
  - **PyDead: Report False Positive (GitHub Issue)**
  - **PyDead: Fix All in File** / **Fix All (workspace)**
  - **PyDead: Find Dead Code**

Setting `pydead.issueRepo` (default `https://github.com/ErikbStorm/pydead`) controls where issues are filed.

## Install

**→ [VS Code Marketplace: PyDead](https://marketplace.visualstudio.com/items?itemName=pydead.pydead)**

```bash
code --install-extension pydead.pydead
```

### From a VSIX (GitHub Release)

- **Direct download:** [pydead-0.1.0.vsix](https://github.com/ErikbStorm/pydead/releases/download/v0.1.0/pydead-0.1.0.vsix)
- Or open [Releases](https://github.com/ErikbStorm/pydead/releases) and grab the latest `.vsix`

```bash
code --install-extension pydead-0.1.0.vsix
# or: Extensions → ⋯ → Install from VSIX…
```

Reload, open a Python workspace, run **PyDead: Find Dead Code**.  
The VSIX includes platform binaries under `bin/` — no separate CLI install required.

Release pipeline: tag `vX.Y.Z` → CI builds binaries, packs this extension, publishes to Marketplace. See [`docs/publishing.md`](../docs/publishing.md).

## Binary resolution

1. Setting `pydead.path` if set
2. Bundled binary under `bin/<platform>/pydead` (shipped in Marketplace builds)
3. `pydead` on your `PATH`

### Local development

```bash
# build the CLI
cargo build -p pydead --release

# copy into extension bundle dir for F5
mkdir -p vscode-extension/bin/darwin-arm64   # or linux-x64 / darwin-x64 / win32-x64
cp target/release/pydead vscode-extension/bin/darwin-arm64/
```

```bash
cd vscode-extension
npm install
npm run compile
# F5 in VS Code to launch Extension Development Host
```

## Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `pydead.enable` | `true` | Turn diagnostics on/off |
| `pydead.path` | `""` | Override binary path |
| `pydead.minConfidence` | `70` | Minimum confidence |
| `pydead.severity` | `Hint` | Diagnostic severity |
| `pydead.runOnSave` | `true` | Re-scan after saving `.py` files |
