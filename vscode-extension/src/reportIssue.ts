import * as cp from "child_process";
import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";
import { Finding, platformHint, resolvePydeadPath } from "./client";

const DEFAULT_ISSUES_REPO = "https://github.com/ErikbStorm/pydead";

/**
 * Interactive flow: collect user context, then open a pre-filled GitHub issue
 * for a false-positive unused finding.
 */
export async function reportFalsePositive(
  document: vscode.TextDocument,
  finding: Finding,
  context: vscode.ExtensionContext
): Promise<void> {
  const packageName = await vscode.window.showInputBox({
    title: "PyDead — false positive report",
    prompt:
      "Library / package / framework this relates to (e.g. sqlalchemy, pydantic, mycompany-sdk)",
    placeHolder: "e.g. sqlalchemy, celery, django, custom plugin",
    value: detectNearbyPackage(document.uri.fsPath),
    ignoreFocusOut: true,
  });
  if (packageName === undefined) {
    return; // cancelled
  }

  const expected = await vscode.window.showInputBox({
    title: "PyDead — false positive report",
    prompt: "What should happen? Why is this symbol actually used?",
    placeHolder:
      "e.g. SQLAlchemy calls bind_expression when the type is used on a column",
    ignoreFocusOut: true,
  });
  if (expected === undefined) {
    return;
  }

  const extra = await vscode.window.showInputBox({
    title: "PyDead — false positive report",
    prompt:
      "Extra context (optional): decorator, host, dynamic call path, links",
    placeHolder: "Optional — leave empty to skip",
    ignoreFocusOut: true,
  });
  if (extra === undefined) {
    return;
  }

  const snippet = extractSnippet(document, finding);
  const packageHint = packageName.trim() || detectNearbyPackage(document.uri.fsPath);
  const binary = resolvePydeadPath(context);
  const version = await tryPydeadVersion(binary);

  const title = buildTitle(finding, packageHint);
  const body = buildBody({
    finding,
    packageName: packageHint,
    expected: expected.trim(),
    extra: (extra || "").trim(),
    snippet,
    filePath: finding.path,
    version,
    platform: platformHint(),
  });

  const repoBase = (
    vscode.workspace.getConfiguration("pydead").get<string>("issueRepo") ||
    DEFAULT_ISSUES_REPO
  ).replace(/\/$/, "");

  const url = buildIssueUrl(repoBase, title, body);

  if (url.length > 7000) {
    await vscode.env.clipboard.writeText(
      `Title: ${title}\n\n${body}\n\nOpen: ${repoBase}/issues/new`
    );
    const pick = await vscode.window.showWarningMessage(
      "Issue body is large — copied to clipboard. Open GitHub Issues to paste?",
      "Open Issues page",
      "Cancel"
    );
    if (pick === "Open Issues page") {
      await vscode.env.openExternal(vscode.Uri.parse(`${repoBase}/issues/new`));
    }
    return;
  }

  await vscode.env.openExternal(vscode.Uri.parse(url));
  vscode.window.showInformationMessage(
    "Opened GitHub to create a false-positive issue. Review and submit when ready."
  );
}

function buildTitle(finding: Finding, packageName: string): string {
  const pkg = packageName ? ` (${packageName})` : "";
  return `False positive: ${finding.kind} \`${finding.name}\`${pkg}`;
}

function buildBody(opts: {
  finding: Finding;
  packageName: string;
  expected: string;
  extra: string;
  snippet: string;
  filePath: string;
  version: string;
  platform: string;
}): string {
  const {
    finding,
    packageName,
    expected,
    extra,
    snippet,
    filePath,
    version,
    platform,
  } = opts;
  return [
    "## Summary",
    "",
    "PyDead reported this definition as unused, but it **is** used (by a framework, host, or dynamic call).",
    "",
    "## Package / framework",
    "",
    packageName || "_(not specified)_",
    "",
    "## What should happen",
    "",
    expected || "_(not specified)_",
    "",
    "## Extra context",
    "",
    extra || "_(none)_",
    "",
    "## Finding details",
    "",
    `- **Rule:** \`${finding.code || "DC?"}\``,
    `- **Kind:** ${finding.kind}`,
    `- **Name:** \`${finding.name}\``,
    `- **Qualname:** \`${finding.qualname}\``,
    `- **Confidence:** ${finding.confidence}`,
    `- **Path:** \`${filePath}\``,
    `- **Range:** L${finding.range.start.line + 1}–L${finding.range.end.line + 1}`,
    `- **Message:** ${finding.message}`,
    "",
    "## Code",
    "",
    "```python",
    snippet || "# (could not extract snippet)",
    "```",
    "",
    "## Environment",
    "",
    `- **pydead:** ${version}`,
    `- **platform:** ${platform}`,
    `- **vscode:** ${vscode.version}`,
    "",
    "## Suggested fix (for maintainers)",
    "",
    "- [ ] Add an **EP** entry-point rule / decorator name / method hook",
    "- [ ] Document in `docs/rules.md`",
    "- [ ] Add a fixture case under `fixtures/`",
    "",
    "---",
    "_Filed via PyDead VS Code extension_",
  ].join("\n");
}

function buildIssueUrl(repoBase: string, title: string, body: string): string {
  const params = new URLSearchParams();
  params.set("title", title);
  params.set("body", body);
  params.set("labels", "false-positive,needs-triage");
  return `${repoBase}/issues/new?${params.toString()}`;
}

function extractSnippet(
  document: vscode.TextDocument,
  finding: Finding
): string {
  const start = Math.max(0, finding.range.start.line);
  const end = Math.min(
    document.lineCount - 1,
    Math.min(finding.range.end.line, start + 40)
  );
  const lines: string[] = [];
  for (let i = start; i <= end; i++) {
    lines.push(document.lineAt(i).text);
  }
  let text = lines.join("\n");
  if (text.length > 2500) {
    text = text.slice(0, 2500) + "\n# ... truncated ...";
  }
  return text;
}

/** Best-effort: nearest pyproject.toml name walking up. */
function detectNearbyPackage(filePath: string): string {
  let dir = path.dirname(filePath);
  for (let i = 0; i < 8; i++) {
    const pyproject = path.join(dir, "pyproject.toml");
    if (fs.existsSync(pyproject)) {
      try {
        const text = fs.readFileSync(pyproject, "utf8");
        const m =
          text.match(/^\s*name\s*=\s*["']([^"']+)["']/m) ||
          text.match(/\[project\][\s\S]*?name\s*=\s*["']([^"']+)["']/);
        if (m) {
          return m[1];
        }
      } catch {
        /* ignore */
      }
    }
    const parent = path.dirname(dir);
    if (parent === dir) {
      break;
    }
    dir = parent;
  }
  return "";
}

function tryPydeadVersion(binary: string): Promise<string> {
  return new Promise((resolve) => {
    cp.execFile(binary, ["--version"], { timeout: 5000 }, (err, stdout) => {
      if (err) {
        resolve("unknown");
        return;
      }
      resolve((stdout || "").toString().trim() || "unknown");
    });
  });
}

export function findingAtCursor(
  findings: Finding[],
  pos: vscode.Position,
  selection: vscode.Range
): Finding | undefined {
  return findings.find((f) => {
    const r = new vscode.Range(
      f.range.start.line,
      f.range.start.character,
      f.range.end.line,
      f.range.end.character
    );
    return r.contains(pos) || !!r.intersection(selection);
  });
}
