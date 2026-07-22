import * as vscode from "vscode";
import {
  Finding,
  platformHint,
  resolvePydeadPath,
  runFind,
  runFix,
} from "./client";
import { DiagnosticController } from "./diagnostics";
import { buildKeepEdit, keepActionTitle } from "./keep";
import { findingAtCursor, reportFalsePositive } from "./reportIssue";

let diagnostics: DiagnosticController;
let statusBar: vscode.StatusBarItem;
let debounceTimer: NodeJS.Timeout | undefined;
let analyzing = false;

export function activate(context: vscode.ExtensionContext): void {
  diagnostics = new DiagnosticController();
  statusBar = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Left,
    50
  );
  statusBar.command = "pydead.find";
  statusBar.tooltip = "PyDead: click to re-scan workspace";
  statusBar.text = "$(search) PyDead";
  statusBar.show();

  context.subscriptions.push(
    diagnostics.collectionRef,
    statusBar,
    vscode.commands.registerCommand("pydead.find", () =>
      analyzeWorkspace(context)
    ),
    vscode.commands.registerCommand("pydead.fixAll", () =>
      fixFindings(context, diagnostics.allFindings())
    ),
    vscode.commands.registerCommand("pydead.fixFile", () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) {
        vscode.window.showInformationMessage("No active editor");
        return;
      }
      const findings = diagnostics.findingsFor(editor.document.uri);
      return fixFindings(context, findings);
    }),
    vscode.commands.registerCommand("pydead.fixOne", (f: Finding) =>
      fixFindings(context, [f])
    ),
    vscode.commands.registerCommand("pydead.afterKeep", () => {
      scheduleAnalyze(context);
    }),
    vscode.commands.registerCommand("pydead.keepSelection", () =>
      keepAtCursor(context, false)
    ),
    vscode.commands.registerCommand("pydead.keepSelectionCodeOnly", () =>
      keepAtCursor(context, true)
    ),
    vscode.commands.registerCommand("pydead.reportFalsePositive", async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor || editor.document.languageId !== "python") {
        vscode.window.showInformationMessage("PyDead: open a Python file first");
        return;
      }
      const findings = diagnostics.findingsFor(editor.document.uri);
      const hit = findingAtCursor(
        findings,
        editor.selection.active,
        editor.selection
      );
      if (!hit) {
        vscode.window.showInformationMessage(
          "PyDead: no unused finding under the cursor"
        );
        return;
      }
      await reportFalsePositive(editor.document, hit, context);
    }),
    vscode.commands.registerCommand(
      "pydead.reportFalsePositiveOne",
      async (uri: vscode.Uri, finding: Finding) => {
        const doc = await vscode.workspace.openTextDocument(uri);
        await reportFalsePositive(doc, finding, context);
      }
    ),
    vscode.languages.registerCodeActionsProvider(
      { language: "python", scheme: "file" },
      new PyDeadCodeActionProvider(),
      {
        providedCodeActionKinds: [vscode.CodeActionKind.QuickFix],
      }
    ),
    vscode.workspace.onDidSaveTextDocument((doc) => {
      if (doc.languageId !== "python") {
        return;
      }
      const cfg = vscode.workspace.getConfiguration("pydead");
      if (
        !cfg.get<boolean>("enable", true) ||
        !cfg.get<boolean>("runOnSave", true)
      ) {
        return;
      }
      scheduleAnalyze(context);
    }),
    vscode.workspace.onDidChangeConfiguration((e) => {
      if (e.affectsConfiguration("pydead")) {
        scheduleAnalyze(context);
      }
    })
  );

  if (vscode.workspace.getConfiguration("pydead").get<boolean>("enable", true)) {
    void analyzeWorkspace(context);
  }
}

export function deactivate(): void {
  if (debounceTimer) {
    clearTimeout(debounceTimer);
  }
}

function scheduleAnalyze(context: vscode.ExtensionContext): void {
  if (debounceTimer) {
    clearTimeout(debounceTimer);
  }
  debounceTimer = setTimeout(() => {
    void analyzeWorkspace(context);
  }, 400);
}

async function analyzeWorkspace(
  context: vscode.ExtensionContext
): Promise<void> {
  if (analyzing) {
    return;
  }
  const folder = vscode.workspace.workspaceFolders?.[0];
  if (!folder) {
    return;
  }
  if (
    !vscode.workspace.getConfiguration("pydead").get<boolean>("enable", true)
  ) {
    diagnostics.clear();
    statusBar.text = "$(search) PyDead off";
    return;
  }

  analyzing = true;
  statusBar.text = "$(loading~spin) PyDead…";

  const binary = resolvePydeadPath(context);
  const minConfidence =
    vscode.workspace.getConfiguration("pydead").get<number>("minConfidence") ??
    70;

  try {
    const report = await runFind(binary, folder.uri.fsPath, minConfidence);
    diagnostics.apply(report, folder.uri.fsPath);
    const n = report.findings.length;
    statusBar.text = n === 0 ? "$(check) PyDead" : `$(warning) PyDead: ${n}`;
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    statusBar.text = "$(error) PyDead";
    vscode.window.showErrorMessage(
      `PyDead failed (${platformHint()}). Is the binary available? ${message}`
    );
  } finally {
    analyzing = false;
  }
}

async function keepAtCursor(
  context: vscode.ExtensionContext,
  codeOnly: boolean
): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  if (!editor || editor.document.languageId !== "python") {
    vscode.window.showInformationMessage("PyDead: open a Python file first");
    return;
  }
  const findings = diagnostics.findingsFor(editor.document.uri);
  const hit = findingAtCursor(
    findings,
    editor.selection.active,
    editor.selection
  );
  if (!hit) {
    vscode.window.showInformationMessage(
      "PyDead: no unused finding under the cursor"
    );
    return;
  }
  const edit = buildKeepEdit(editor.document, hit, codeOnly);
  if (!edit) {
    vscode.window.showInformationMessage(
      "PyDead: keep marker already present"
    );
    return;
  }
  await vscode.workspace.applyEdit(edit);
  scheduleAnalyze(context);
}

async function fixFindings(
  context: vscode.ExtensionContext,
  findings: Finding[]
): Promise<void> {
  const folder = vscode.workspace.workspaceFolders?.[0];
  if (!folder) {
    return;
  }
  const fixable = findings.filter((f) => f.fixable);
  if (fixable.length === 0) {
    vscode.window.showInformationMessage("PyDead: nothing to fix");
    return;
  }

  const binary = resolvePydeadPath(context);
  const minConfidence =
    vscode.workspace.getConfiguration("pydead").get<number>("minConfidence") ??
    70;

  try {
    await runFix(
      binary,
      folder.uri.fsPath,
      fixable.map((f) => f.id),
      minConfidence
    );
    vscode.window.showInformationMessage(
      `PyDead: removed ${fixable.length} unused definition(s)`
    );
    await analyzeWorkspace(context);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    vscode.window.showErrorMessage(`PyDead fix failed: ${message}`);
  }
}

/**
 * Lightbulb (⌘.) only offers the two common actions:
 *   1. Keep  (`# pydead: keep`) — preferred
 *   2. Remove this definition
 *
 * Less common actions stay in the Command Palette / editor context menu:
 *   - PyDead: Keep (code only) — via keepSelection + code-only not in lightbulb
 *   - PyDead: Report False Positive
 *   - PyDead: Fix All in File / workspace
 */
class PyDeadCodeActionProvider implements vscode.CodeActionProvider {
  provideCodeActions(
    document: vscode.TextDocument,
    range: vscode.Range | vscode.Selection,
    context: vscode.CodeActionContext
  ): vscode.CodeAction[] {
    const findings = diagnostics.findingsFor(document.uri);
    const actions: vscode.CodeAction[] = [];
    const seen = new Set<string>();

    const relevant = findings.filter((f) => {
      const fRange = findingRange(f);
      return (
        !!fRange.intersection(range) ||
        fRange.contains(range.start) ||
        context.diagnostics.some(
          (d) =>
            d.source === "pydead" &&
            d.range.intersection(fRange) !== undefined
        )
      );
    });

    for (const f of relevant) {
      if (seen.has(f.id)) {
        continue;
      }
      seen.add(f.id);

      // Preferred: mark as used
      const keepEdit = buildKeepEdit(document, f, false);
      if (keepEdit) {
        const keep = new vscode.CodeAction(
          keepActionTitle(f, false),
          vscode.CodeActionKind.QuickFix
        );
        keep.edit = keepEdit;
        keep.diagnostics = pydeadDiagnosticsFor(context, f);
        keep.isPreferred = true;
        keep.command = {
          command: "pydead.afterKeep",
          title: "Rescan after keep",
        };
        actions.push(keep);
      }

      // Remove this definition only (bulk fix lives in the command palette)
      if (f.fixable) {
        const remove = new vscode.CodeAction(
          `PyDead: remove unused ${f.kind} '${f.name}'`,
          vscode.CodeActionKind.QuickFix
        );
        remove.command = {
          command: "pydead.fixOne",
          title: "Remove unused definition",
          arguments: [f],
        };
        remove.diagnostics = pydeadDiagnosticsFor(context, f);
        actions.push(remove);
      }
    }

    return actions;
  }
}

function findingRange(f: Finding): vscode.Range {
  return new vscode.Range(
    f.range.start.line,
    f.range.start.character,
    f.range.end.line,
    f.range.end.character
  );
}

function pydeadDiagnosticsFor(
  context: vscode.CodeActionContext,
  f: Finding
): vscode.Diagnostic[] {
  const fr = findingRange(f);
  return context.diagnostics.filter(
    (d) => d.source === "pydead" && d.range.intersection(fr)
  );
}
