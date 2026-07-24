import * as path from "path";
import * as vscode from "vscode";
import { Finding, Report } from "./client";

export class DiagnosticController {
  private readonly collection: vscode.DiagnosticCollection;
  private findingsByUri = new Map<string, Finding[]>();

  constructor() {
    this.collection = vscode.languages.createDiagnosticCollection("pydead");
  }

  dispose(): void {
    this.collection.dispose();
  }

  clear(): void {
    this.collection.clear();
    this.findingsByUri.clear();
  }

  apply(report: Report, workspaceRoot: string): void {
    this.clear();

    const byPath = new Map<string, Finding[]>();
    for (const f of report.findings) {
      const abs = path.normalize(path.join(workspaceRoot, f.path));
      const list = byPath.get(abs) ?? [];
      list.push(f);
      byPath.set(abs, list);
    }

    const severity = severityFromConfig();

    for (const [abs, findings] of byPath) {
      const uri = vscode.Uri.file(abs);
      const diags: vscode.Diagnostic[] = findings.map((f) => {
        const range = new vscode.Range(
          f.range.start.line,
          f.range.start.character,
          f.range.end.line,
          f.range.end.character
        );
        const msg = f.code
          ? `${f.code}: ${f.message}`
          : f.message;
        const d = new vscode.Diagnostic(range, msg, severity);
        d.source = "pydead";
        d.code = f.code || f.kind;
        d.tags = [vscode.DiagnosticTag.Unnecessary];
        return d;
      });
      this.collection.set(uri, diags);
      this.findingsByUri.set(uri.toString(), findings);
    }
  }

  findingsFor(uri: vscode.Uri): Finding[] {
    return this.findingsByUri.get(uri.toString()) ?? [];
  }

  allFindings(): Finding[] {
    const out: Finding[] = [];
    for (const list of this.findingsByUri.values()) {
      out.push(...list);
    }
    return out;
  }

  get collectionRef(): vscode.DiagnosticCollection {
    return this.collection;
  }
}

function severityFromConfig(): vscode.DiagnosticSeverity {
  const s = vscode.workspace
    .getConfiguration("pydead")
    .get<string>("severity", "Warning");
  switch (s) {
    case "Error":
      return vscode.DiagnosticSeverity.Error;
    case "Information":
      return vscode.DiagnosticSeverity.Information;
    case "Hint":
      return vscode.DiagnosticSeverity.Hint;
    case "Warning":
    default:
      return vscode.DiagnosticSeverity.Warning;
  }
}
