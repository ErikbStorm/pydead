import * as vscode from "vscode";
import { Finding } from "./client";

/**
 * Insert `# pydead: keep` (or keep CODE) on the definition's first line so the
 * analyzer treats it as intentionally used.
 */
export function buildKeepEdit(
  document: vscode.TextDocument,
  finding: Finding,
  codeOnly: boolean
): vscode.WorkspaceEdit | undefined {
  const lineNum = finding.range.start.line;
  if (lineNum < 0 || lineNum >= document.lineCount) {
    return undefined;
  }

  const line = document.lineAt(lineNum);
  const text = line.text;

  // Already has a keep / ignore / noqa marker
  if (/(#\s*(pydead\s*:|noqa))/i.test(text)) {
    // If bare noqa/keep already present, nothing to do for full keep
    if (!codeOnly) {
      return undefined;
    }
  }

  const marker = codeOnly && finding.code
    ? `  # pydead: keep ${finding.code}`
    : `  # pydead: keep`;

  // Avoid double-space if line ends with spaces
  const trimmedRight = text.replace(/\s+$/, "");
  const newText = alreadyHasKeep(trimmedRight, finding.code, codeOnly)
    ? null
    : appendMarker(trimmedRight, marker, finding.code, codeOnly);

  if (newText === null) {
    return undefined;
  }

  const edit = new vscode.WorkspaceEdit();
  edit.replace(document.uri, line.range, newText);
  return edit;
}

function alreadyHasKeep(
  line: string,
  code: string | undefined,
  codeOnly: boolean
): boolean {
  const m = line.match(/#\s*pydead\s*:\s*(keep|used|allow|ignore|noqa)\b(.*)$/i);
  if (!m) {
    const noqa = line.match(/#\s*noqa\b(.*)$/i);
    if (!noqa) {
      return false;
    }
    const rest = (noqa[1] || "").trim();
    if (!rest || rest === ":") {
      return !codeOnly; // bare noqa covers all
    }
    if (codeOnly && code) {
      return rest.toUpperCase().includes(code.toUpperCase());
    }
    return true;
  }
  const rest = (m[2] || "").trim();
  if (!rest || rest.startsWith(":") && rest.replace(/^:\s*/, "") === "") {
    return !codeOnly || true; // bare keep covers all codes
  }
  if (codeOnly && code) {
    return rest.toUpperCase().includes(code.toUpperCase());
  }
  return true;
}

function appendMarker(
  trimmedRight: string,
  marker: string,
  code: string | undefined,
  codeOnly: boolean
): string {
  // If line already ends with # comment, append keep into that comment carefully
  const hash = trimmedRight.lastIndexOf("#");
  if (hash >= 0) {
    const before = trimmedRight.slice(0, hash).replace(/\s+$/, "");
    const comment = trimmedRight.slice(hash);
    // Existing noqa without our code — add pydead keep after
    if (codeOnly && code && /#\s*noqa\b/i.test(comment)) {
      return `${before}${comment}, ${code}`;
    }
    if (/#\s*pydead\s*:/i.test(comment)) {
      return `${before}${comment}`; // leave as-is (alreadyHasKeep should catch)
    }
    return `${before}${marker}`;
  }
  return `${trimmedRight}${marker}`;
}

export function keepActionTitle(finding: Finding, codeOnly: boolean): string {
  if (codeOnly && finding.code) {
    return `PyDead: keep (${finding.code} only)`;
  }
  return `PyDead: keep '${finding.name}' (mark as used)`;
}
