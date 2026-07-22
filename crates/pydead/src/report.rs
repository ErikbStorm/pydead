use std::io::Write;
use std::path::Path;

use serde::Serialize;

use crate::analyze::AnalysisResult;
use crate::symbols::Finding;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Format {
    Text,
    Json,
    Sarif,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub version: u32,
    pub root: String,
    pub findings: Vec<Finding>,
    pub stats: crate::symbols::Stats,
}

impl Report {
    pub fn from_result(result: &AnalysisResult) -> Self {
        Self {
            version: 1,
            root: result.root.display().to_string(),
            findings: result.findings.clone(),
            stats: result.stats.clone(),
        }
    }
}

pub fn write_report(
    result: &AnalysisResult,
    format: Format,
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    match format {
        Format::Text => write_text(result, out)?,
        Format::Json => {
            let report = Report::from_result(result);
            serde_json::to_writer_pretty(&mut *out, &report)?;
            writeln!(out)?;
        }
        Format::Sarif => write_sarif(result, out)?,
    }
    Ok(())
}

fn write_text(result: &AnalysisResult, out: &mut dyn Write) -> anyhow::Result<()> {
    if result.findings.is_empty() {
        writeln!(
            out,
            "No dead code found ({} files, {} definitions).",
            result.stats.files, result.stats.definitions
        )?;
        return Ok(());
    }

    for f in &result.findings {
        // text uses 1-based line/col for humans (like compilers)
        let line = f.range.start.line + 1;
        let col = f.range.start.character + 1;
        writeln!(
            out,
            "{}:{}:{}: {} {} '{}' is unused (confidence {})",
            f.path,
            line,
            col,
            f.code,
            f.kind.as_str(),
            f.name,
            f.confidence
        )?;
    }
    writeln!(
        out,
        "\n{} dead definition(s) in {} file(s) ({} definitions scanned).",
        result.stats.dead, result.stats.files, result.stats.definitions
    )?;
    if result.stats.parse_errors > 0 {
        writeln!(
            out,
            "warning: {} file(s) failed to parse and were skipped.",
            result.stats.parse_errors
        )?;
    }
    Ok(())
}

fn write_sarif(result: &AnalysisResult, out: &mut dyn Write) -> anyhow::Result<()> {
    use crate::rules::all_rules;
    let rules: Vec<_> = all_rules()
        .into_iter()
        .filter(|r| r.code.starts_with("DC"))
        .map(|r| {
            serde_json::json!({
                "id": r.code,
                "name": r.name,
                "shortDescription": { "text": r.explanation },
                "helpUri": "https://github.com/erik/deadcode/blob/main/docs/rules.md"
            })
        })
        .collect();
    // Minimal SARIF 2.1.0
    let runs = serde_json::json!({
        "version": "2.1.0",
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "pydead",
                    "informationUri": "https://github.com/erik/deadcode",
                    "rules": rules,
                }
            },
            "results": result.findings.iter().map(|f| {
                serde_json::json!({
                    "ruleId": f.code,
                    "level": "warning",
                    "message": { "text": format!("{}: {}", f.code, f.message) },
                    "locations": [{
                        "physicalLocation": {
                            "artifactLocation": {
                                "uri": f.path,
                            },
                            "region": {
                                "startLine": f.range.start.line + 1,
                                "startColumn": f.range.start.character + 1,
                                "endLine": f.range.end.line + 1,
                                "endColumn": f.range.end.character + 1,
                            }
                        }
                    }],
                    "properties": {
                        "confidence": f.confidence,
                        "qualname": f.qualname,
                        "kind": f.kind.as_str(),
                        "id": f.id,
                        "code": f.code,
                    }
                })
            }).collect::<Vec<_>>(),
        }]
    });
    serde_json::to_writer_pretty(&mut *out, &runs)?;
    writeln!(out)?;
    let _ = Path::new("."); // silence if unused in future
    Ok(())
}
