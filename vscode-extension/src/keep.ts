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

const FILE_IGNORE_MARKER = "# pydead: ignore-file";
const FILE_IGNORE_RE =
  /^\s*#\s*pydead\s*:\s*(ignore-file|file-ignore|noqa-file)\b/i;

/** True if the document already has a whole-file ignore pragma. */
export function documentHasFileIgnore(document: vscode.TextDocument): boolean {
  for (let i = 0; i < document.lineCount; i++) {
    if (FILE_IGNORE_RE.test(document.lineAt(i).text)) {
      return true;
    }
  }
  return false;
}

/**
 * Insert `# pydead: ignore-file` near the top of the file (after shebang /
 * encoding cookie). Returns undefined if already present.
 */
export function buildIgnoreFileEdit(
  document: vscode.TextDocument
): vscode.WorkspaceEdit | undefined {
  if (documentHasFileIgnore(document)) {
    return undefined;
  }

  const eol = document.eol === vscode.EndOfLine.CRLF ? "\r\n" : "\n";
  const insertLine = fileIgnoreInsertLine(document);
  const edit = new vscode.WorkspaceEdit();

  if (document.lineCount === 0 || document.getText().length === 0) {
    edit.insert(
      document.uri,
      new vscode.Position(0, 0),
      `${FILE_IGNORE_MARKER}${eol}`
    );
    return edit;
  }

  if (insertLine >= document.lineCount) {
    const last = document.lineAt(document.lineCount - 1);
    const prefix = last.text.length > 0 ? eol : "";
    edit.insert(
      document.uri,
      last.range.end,
      `${prefix}${FILE_IGNORE_MARKER}${eol}`
    );
  } else {
    edit.insert(
      document.uri,
      new vscode.Position(insertLine, 0),
      `${FILE_IGNORE_MARKER}${eol}`
    );
  }
  return edit;
}

/** Insert after shebang and coding cookie comments; otherwise at line 0. */
function fileIgnoreInsertLine(document: vscode.TextDocument): number {
  let line = 0;
  if (document.lineCount === 0) {
    return 0;
  }
  if (document.lineAt(0).text.startsWith("#!")) {
    line = 1;
  }
  if (line < document.lineCount) {
    const t = document.lineAt(line).text;
    if (
      /^#.*coding[:=]\s*[-_.a-zA-Z0-9]+/.test(t) ||
      /^#\s*-\*-.*coding[:=].*-\*-/.test(t)
    ) {
      line += 1;
    }
  }
  return Math.min(line, document.lineCount);
}
