/**
 * Headless capture using code-server (real VS Code UI in browser) + Playwright.
 * Runs only inside Docker — does not touch host VS Code.
 */
import { spawn, execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const WORK = process.env.WORK_ROOT || "/work";
const OUT = process.env.OUT_DIR || "/out";
const FIXTURE = process.env.FIXTURE || path.join(WORK, "fixtures/sqlalchemy_project");
const PYDEAD = process.env.PYDEAD_BIN || "pydead";
const PASSWORD = process.env.PASSWORD || "pydeadshot";
const PORT = process.env.CODE_PORT || "8080";

function ensureDir(d) {
  fs.mkdirSync(d, { recursive: true });
}

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

function runCliPng() {
  console.log("→ CLI: true XTerm capture (Xvfb)");
  ensureDir(OUT);
  const script = path.join(__dirname, "capture-cli-xterm.sh");
  execFileSync("bash", [script], {
    stdio: "inherit",
    env: {
      ...process.env,
      WORK_ROOT: WORK,
      OUT_DIR: OUT,
      PYDEAD_BIN: PYDEAD,
    },
  });
}

async function waitHttp(url, ms = 120_000) {
  const t0 = Date.now();
  while (Date.now() - t0 < ms) {
    try {
      const res = await fetch(url, { redirect: "manual" });
      if (res.status) return;
    } catch {
      /* */
    }
    await sleep(400);
  }
  throw new Error(`timeout waiting for ${url}`);
}

function writeCodeServerSettings() {
  const userDir = path.join(WORK, ".code-user", "User");
  ensureDir(userDir);
  // Avoid Restricted Mode so the extension can run + spawn pydead
  fs.writeFileSync(
    path.join(userDir, "settings.json"),
    JSON.stringify(
      {
        "security.workspace.trust.enabled": false,
        "security.workspace.trust.startupPrompt": "never",
        "security.workspace.trust.banner": "never",
        "security.workspace.trust.untrustedFiles": "open",
        "workbench.colorTheme": "Default Dark Modern",
        "workbench.preferredDarkColorTheme": "Default Dark Modern",
        "window.autoDetectColorScheme": false,
        "workbench.preferredLightColorTheme": "Default Dark Modern",
        "pydead.enable": true,
        "pydead.path": PYDEAD,
        "pydead.severity": "Warning",
        "pydead.runOnSave": true,
        "python.analysis.diagnosticSeverityOverrides": {
          reportUnusedFunction: "none",
          reportUnusedClass: "none",
          reportUnusedVariable: "none",
        },
        "editor.lightbulb.enabled": "on",
        "workbench.startupEditor": "none",
        "extensions.autoUpdate": false,
        "telemetry.telemetryLevel": "off",
      },
      null,
      2
    )
  );
}

async function captureCodeServer() {
  console.log("→ code-server (headless VS Code) …");
  writeCodeServerSettings();
  const base = `http://127.0.0.1:${PORT}`;

  // --auth none: isolated Docker network only (no host exposure of this port)
  const child = spawn(
    "code-server",
    [
      "--bind-addr",
      `127.0.0.1:${PORT}`,
      "--auth",
      "none",
      "--disable-telemetry",
      "--disable-update-check",
      "--disable-workspace-trust",
      "--user-data-dir",
      path.join(WORK, ".code-user"),
      "--extensions-dir",
      path.join(WORK, ".local/share/code-server/extensions"),
      FIXTURE,
    ],
    {
      env: {
        ...process.env,
        // Make pydead available to extension
        PATH: `/usr/local/bin:${process.env.PATH || ""}`,
      },
      stdio: ["ignore", "pipe", "pipe"],
    }
  );

  let log = "";
  child.stdout.on("data", (d) => {
    log += d.toString();
    process.stdout.write(d);
  });
  child.stderr.on("data", (d) => {
    log += d.toString();
    process.stderr.write(d);
  });

  try {
    await waitHttp(base, 120_000);
    await sleep(2000);

    const browser = await chromium.launch({
      headless: true,
      args: ["--no-sandbox", "--disable-dev-shm-usage"],
    });
    const page = await browser.newPage({
      viewport: { width: 1440, height: 900 },
      deviceScaleFactor: 2,
    });

    try {
      await page.goto(base, { waitUntil: "domcontentloaded", timeout: 120_000 });

      // Password form (if auth enabled)
      try {
        const pwd = page.locator('input[type="password"]');
        if ((await pwd.count()) > 0 && (await pwd.first().isVisible())) {
          await pwd.first().fill(PASSWORD);
          const submit = page.locator(
            'button[type="submit"], button:has-text("Submit"), input[type="submit"]'
          );
          if ((await submit.count()) > 0) {
            await submit.first().click();
          } else {
            await page.keyboard.press("Enter");
          }
          await sleep(3000);
        }
      } catch {
        /* no password gate */
      }

      // Workbench
      await page.waitForSelector(".monaco-workbench", { timeout: 180_000 });
      console.log("  workbench ready");
      await sleep(2000);

      // Belt-and-suspenders: force Default Dark Modern via command palette
      try {
        await page.keyboard.press("Control+k");
        await page.keyboard.press("Control+t");
        await sleep(800);
        await page.keyboard.type("Dark Modern", { delay: 30 });
        await sleep(600);
        await page.keyboard.press("Enter");
        await sleep(800);
        console.log("  theme: Default Dark Modern");
      } catch {
        /* settings.json already sets the theme */
      }
      await sleep(1000);

      // Dismiss Restricted Mode / trust banners if still shown
      for (const label of [
        "text=Manage",
        "text=Trust",
        "text=Yes, I trust the authors",
        'a:has-text("Trust")',
        'button:has-text("Trust")',
      ]) {
        try {
          const el = page.locator(label).first();
          if ((await el.count()) && (await el.isVisible({ timeout: 800 }))) {
            await el.click({ timeout: 2000 });
            await sleep(1000);
          }
        } catch {
          /* */
        }
      }
      try {
        await page.keyboard.press("Escape");
        await sleep(200);
      } catch {
        /* */
      }

      // Quick Open file
      await page.keyboard.press("Control+p");
      await sleep(800);
      await page.keyboard.type("geo_types.py", { delay: 40 });
      await sleep(700);
      await page.keyboard.press("Enter");
      await sleep(4000);

      // Force PyDead scan via command palette
      await page.keyboard.press("Control+Shift+p");
      await sleep(600);
      await page.keyboard.type("PyDead: Find Dead Code", { delay: 25 });
      await sleep(500);
      await page.keyboard.press("Enter");
      console.log("  ran PyDead: Find Dead Code — waiting for diagnostics");
      await sleep(15000);

      // Open Problems panel
      await page.keyboard.press("Control+Shift+m");
      await sleep(2000);

      // Go to line with unused method
      await page.keyboard.press("Control+g");
      await sleep(400);
      await page.keyboard.type("56", { delay: 30 });
      await page.keyboard.press("Enter");
      await sleep(2500);

      ensureDir(OUT);
      await page.screenshot({ path: path.join(OUT, "vscode-diag.png") });
      console.log("  wrote vscode-diag.png");

      // Quick Fix on current line
      await page.keyboard.press("Control+Period");
      await sleep(3000);
      await page.screenshot({ path: path.join(OUT, "vscode-quickfix.png") });
      console.log("  wrote vscode-quickfix.png");
    } finally {
      await browser.close();
    }
  } finally {
    child.kill("SIGTERM");
    await sleep(500);
    try {
      child.kill("SIGKILL");
    } catch {
      /* */
    }
  }
}

async function main() {
  ensureDir(OUT);
  runCliPng();
  await captureCodeServer();

  console.log("\nArtifacts:");
  for (const f of ["cli-find.png", "vscode-diag.png", "vscode-quickfix.png"]) {
    const p = path.join(OUT, f);
    console.log(
      fs.existsSync(p)
        ? `  ✓ ${f} (${fs.statSync(p).size} bytes)`
        : `  ✗ ${f} MISSING`
    );
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
