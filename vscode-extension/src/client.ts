import * as cp from "child_process";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import * as vscode from "vscode";

export interface Position {
  line: number;
  character: number;
}

export interface Range {
  start: Position;
  end: Position;
}

export interface Finding {
  id: string;
  kind: string;
  name: string;
  qualname: string;
  path: string;
  range: Range;
  confidence: number;
  code: string;
  message: string;
  fixable: boolean;
}

export interface Report {
  version: number;
  root: string;
  findings: Finding[];
  stats: {
    files: number;
    definitions: number;
    dead: number;
    parse_errors: number;
  };
}

/** Resolve the pydead binary: setting → bundled → PATH. */
export function resolvePydeadPath(context: vscode.ExtensionContext): string {
  const configured = vscode.workspace
    .getConfiguration("pydead")
    .get<string>("path")
    ?.trim();
  if (configured) {
    return configured;
  }

  const bundled = bundledBinaryPath(context);
  if (bundled && fs.existsSync(bundled)) {
    return bundled;
  }

  return "pydead";
}

function bundledBinaryPath(context: vscode.ExtensionContext): string | undefined {
  const platform = process.platform;
  const arch = process.arch;
  let platformDir: string;
  if (platform === "darwin" && arch === "arm64") {
    platformDir = "darwin-arm64";
  } else if (platform === "darwin") {
    platformDir = "darwin-x64";
  } else if (platform === "linux") {
    platformDir = "linux-x64";
  } else if (platform === "win32") {
    platformDir = "win32-x64";
  } else {
    return undefined;
  }
  const name = platform === "win32" ? "pydead.exe" : "pydead";
  return path.join(context.extensionPath, "bin", platformDir, name);
}

export async function runFind(
  binary: string,
  workspaceRoot: string,
  minConfidence: number
): Promise<Report> {
  const args = [
    "find",
    workspaceRoot,
    "--format",
    "json",
    "--min-confidence",
    String(minConfidence),
  ];
  const { stdout } = await execFileAsync(binary, args, workspaceRoot);
  return JSON.parse(stdout) as Report;
}

export async function runFix(
  binary: string,
  workspaceRoot: string,
  ids: string[],
  minConfidence: number
): Promise<void> {
  const args = [
    "fix",
    workspaceRoot,
    "--yes",
    "--min-confidence",
    String(minConfidence),
  ];
  if (ids.length > 0) {
    args.push("--ids", ids.join(","));
  }
  await execFileAsync(binary, args, workspaceRoot);
}

function execFileAsync(
  binary: string,
  args: string[],
  cwd: string
): Promise<{ stdout: string; stderr: string }> {
  return new Promise((resolve, reject) => {
    cp.execFile(
      binary,
      args,
      {
        cwd,
        maxBuffer: 20 * 1024 * 1024,
        env: { ...process.env },
        timeout: 120_000,
      },
      (err, stdout, stderr) => {
        if (err) {
          const msg =
            stderr?.trim() ||
            err.message ||
            `Failed to run ${binary}`;
          reject(new Error(msg));
          return;
        }
        resolve({ stdout: stdout.toString(), stderr: stderr.toString() });
      }
    );
  });
}

export function platformHint(): string {
  return `${os.platform()}-${os.arch()}`;
}
