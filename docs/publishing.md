# Publishing PyDead

Skip crates.io (by design). Releases go to **GitHub Releases** + the **VS Code Marketplace**.

## One-time setup

### 1. Marketplace publisher

You already created a publisher. Confirm the **publisher ID** matches `vscode-extension/package.json`:

```json
"publisher": "pydead"
```

If your Azure DevOps / Marketplace publisher is different (e.g. `ErikbStorm`), change `publisher` to match **exactly**.

Marketplace item id will be: `{publisher}.pydead`  
(e.g. `pydead.pydead`)

### 2. Create a publish token (`VSCE_PAT`)

1. Open [Azure DevOps](https://dev.azure.com) (same Microsoft account as Marketplace).
2. User settings → **Personal access tokens** → New token.
3. Organization: **All accessible organizations** (or the one linked to Marketplace).
4. Scopes: **Marketplace** → **Manage** (or Acquire + Publish).
5. Copy the token once.

### 3. Store the token as a GitHub Actions secret

```bash
# from a machine with gh auth
gh secret set VSCE_PAT --repo ErikbStorm/pydead
# paste the Azure DevOps PAT when prompted
```

Without this secret, CI still creates the GitHub Release and `.vsix`; you can publish manually:

```bash
cd vscode-extension
npx vsce publish --packagePath pydead-0.1.0.vsix -p "$VSCE_PAT"
```

### Extension icon

Marketplace listing uses `vscode-extension/icon.png` (`package.json` → `"icon": "icon.png"`). Keep it ≥128×128 PNG (we ship 512×512).

## Cut a release

1. Bump versions if needed (tag drives extension version in CI):
   - `Cargo.toml` workspace `version`
   - `vscode-extension/package.json` `version` (overwritten from the tag in CI)
2. Commit on `main`.
3. Tag and push:

```bash
git tag v0.1.0
git push origin v0.1.0
```

4. Watch **Actions → Release**. It will:
   - Build `linux-x64`, `darwin-arm64`, `darwin-x64`, `win32-x64`
   - Bundle binaries into the extension
   - Upload CLI archives + `.vsix` + **`SHA256SUMS`** to the GitHub Release
   - Publish to the Marketplace if `VSCE_PAT` is set

`scripts/install.sh` downloads `SHA256SUMS` and refuses to install on mismatch
(set `PYDEAD_VERIFY=0` only for emergency bypass).
## Install surfaces after release

| Audience | How |
|----------|-----|
| VS Code | Prefer **VSIX** from the GitHub Release until Marketplace is public |
| VS Code | Marketplace item `pydead.pydead` (only after it appears in [manage](https://marketplace.visualstudio.com/manage)) |
| CLI | `curl -fsSL …/scripts/install.sh \| bash` (checks `SHA256SUMS`) |
| Dev | `cargo install --path crates/pydead` |

### Empty publisher (no extensions listed)

If https://marketplace.visualstudio.com/manage shows publisher **`pydead`** but **no extensions**:

1. Confirm **`package.json`** has `"publisher": "pydead"` (exact id, case-sensitive).
2. Create a **new Azure DevOps PAT** while signed in as the **same** Microsoft account that owns that publisher:
   - https://dev.azure.com → User settings → Personal access tokens  
   - Organization: **All accessible organizations**  
   - Scopes: **Marketplace → Manage**
3. Update the GitHub secret:
   ```bash
   gh secret set VSCE_PAT --repo ErikbStorm/pydead
   # paste the new PAT
   ```
4. Run **Actions → Publish Marketplace** (uses release VSIX + your PAT).
5. Refresh the manage page — you should see extension **pydead**.

Public URL (after it appears in the hub):  
https://marketplace.visualstudio.com/items?itemName=pydead.pydead

Until then, install via VSIX from GitHub Releases.

### Marketplace 404 after `vsce` says “Published”

If publish logs say success but the public page 404s and the hub is still empty, the PAT almost always belongs to a **different** Microsoft account than the hub publisher. Recreate the PAT under the hub account (steps above) and re-run **Publish Marketplace**.

## Manual local VSIX (optional)

```bash
cargo build -p pydead --release
mkdir -p vscode-extension/bin/darwin-arm64   # match your platform
cp target/release/pydead vscode-extension/bin/darwin-arm64/
cd vscode-extension && npm ci && npm run package
```
