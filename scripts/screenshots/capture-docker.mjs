/**
 * Headless capture inside Docker — never touches host VS Code.
 *
 * - CLI PNG from real pydead output
 * - VS Code Web via @vscode/test-web + Playwright Chromium
 *
 * Outputs written to /out (bind-mount to docs/images on the host).
 */
import { spawn, execFileSync } from "node:child_process";
import { createServer } from "node:http";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const WORK = process.env.WORK_ROOT || path.resolve(__dirname, "../..");
const OUT = process.env.OUT_DIR || "/out";
const FIXTURE =
  process.env.FIXTURE || path.join(WORK, "fixtures/sqlalchemy_project");
const EXT = process.env.EXT_PATH || path.join(WORK, "vscode-extension");
const PYDEAD = process.env.PYDEAD_BIN || "pydead";

function ensureDir(d) {
  fs.mkdirSync(d, { recursive: true });
}

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

function freePort() {
  return new Promise((resolve, reject) => {
    const s = createServer();
    s.listen(0, "127.0.0.1", () => {
      const { port } = s.address();
      s.close(() => resolve(port));
    });
    s.on("error", reject);
  });
}

function runCliPng() {
  console.log("→ CLI: real pydead find");
  ensureDir(OUT);
  const sample = path.join(WORK, "fixtures/sample_project");
  const out = execFileSync(PYDEAD, ["find", sample], { encoding: "utf8" });
  const dump = path.join(OUT, "cli-find.raw.txt");
  fs.writeFileSync(dump, out);
  execFileSync(
    "python3",
    [path.join(__dirname, "render_terminal.py"), dump, path.join(OUT, "cli-find.png")],
    { stdio: "inherit" }
  );
}

async function waitHttp(url, child, ms = 180_000) {
  const t0 = Date.now();
  while (Date.now() - t0 < ms) {
    if (child.exitCode !== null) {
      throw new Error(`vscode-test-web exited: ${child.exitCode}`);
    }
    try {
      const res = await fetch(url, { redirect: "manual" });
      if (res.status) return;
    } catch {
      /* not ready */
    }
    await sleep(400);
  }
  throw new Error(`timeout waiting for ${url}`);
}

async function captureVscodeWeb() {
  console.log("→ VS Code Web (headless, Docker)");
  if (!fs.existsSync(path.join(EXT, "out/extension.js"))) {
    throw new Error("extension not compiled");
  }

  const testWeb = path.join(__dirname, "node_modules", ".bin", "vscode-test-web");
  if (!fs.existsSync(testWeb)) {
    throw new Error("vscode-test-web missing — npm install in scripts/screenshots");
  }

  const port = await freePort();
  const base = `http://127.0.0.1:${port}`;
  console.log(`  server port ${port}`);

  const child = spawn(
    testWeb,
    [
      "--browserType=none",
      "--host=127.0.0.1",
      `--port=${port}`,
      `--extensionDevelopmentPath=${EXT}`,
      `--folder-uri=file://${FIXTURE}`,
      "--quality=stable",
    ],
    { cwd: __dirname, stdio: ["ignore", "pipe", "pipe"], env: process.env }
  );

  let log = "";
  const onData = (d) => {
    const s = d.toString();
    log += s;
    process.stdout.write(s);
  };
  child.stdout.on("data", onData);
  child.stderr.on("data", onData);

  try {
    await waitHttp(base, child, 240_000);
    // settle after listen
    await sleep(2500);

    const browser = await chromium.launch({
      headless: true,
      args: ["--no-sandbox", "--disable-dev-shm-usage"],
    });
    const page = await browser.newPage({
      viewport: { width: 1440, height: 900 },
      deviceScaleFactor: 2,
    });

    try {
      await page.goto(base, { waitUntil: "domcontentloaded", timeout: 180_000 });
      await page.waitForSelector(".monaco-workbench", { timeout: 180_000 });
      console.log("  workbench ready — waiting for extension host");
      await sleep(12000);

      // Open file via Quick Open
      await page.keyboard.press("Control+p");
      await sleep(700);
      await page.keyboard.type("geo_types.py", { delay: 45 });
      await sleep(600);
      await page.keyboard.press("Enter");
      await sleep(10000);

      // Jump near unused method
      await page.keyboard.press("Control+g");
      await sleep(400);
      await page.keyboard.type("13", { delay: 40 });
      await page.keyboard.press("Enter");
      await sleep(2000);

      ensureDir(OUT);
      await page.screenshot({
        path: path.join(OUT, "vscode-diag.png"),
        type: "png",
      });
      console.log("  wrote vscode-diag.png");

      // Quick Fix
      await page.keyboard.press("Control+Period");
      await sleep(2500);
      await page.screenshot({
        path: path.join(OUT, "vscode-quickfix.png"),
        type: "png",
      });
      console.log("  wrote vscode-quickfix.png");
    } finally {
      await browser.close();
    }
  } finally {
    child.kill("SIGTERM");
    await sleep(800);
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
  await captureVscodeWeb();

  console.log("\nArtifacts in", OUT);
  for (const f of ["cli-find.png", "vscode-diag.png", "vscode-quickfix.png"]) {
    const p = path.join(OUT, f);
    if (fs.existsSync(p)) {
      console.log(`  ✓ ${f} (${fs.statSync(p).size} bytes)`);
    } else {
      console.error(`  ✗ ${f} missing`);
      process.exitCode = 1;
    }
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
