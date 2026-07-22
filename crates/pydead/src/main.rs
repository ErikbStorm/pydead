use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use pydead::{all_rules, analyze, apply_fixes, write_report, AnalysisOptions, Config, Format};

#[derive(Parser, Debug)]
#[command(
    name = "pydead",
    version,
    about = "Find and fix dead Python code across a whole folder (cross-file)."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Find unused functions, classes, methods, and module-level variables.
    Find {
        /// Root folder to analyze (default: current directory).
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long, value_enum, default_value = "text")]
        format: OutputFormat,
        #[arg(long)]
        min_confidence: Option<u8>,
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// List rule codes (EP entry-points, DC findings) — like Ruff's rule catalog.
    Rules,
    /// Remove dead definitions (run `find` first or pass --ids).
    Fix {
        /// Root folder to analyze and fix.
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Show changes without writing files.
        #[arg(long)]
        dry_run: bool,
        /// Apply all fixable findings without prompting.
        #[arg(long)]
        yes: bool,
        /// Comma-separated finding ids (from JSON output). If omitted, fixes all.
        #[arg(long, value_delimiter = ',')]
        ids: Vec<String>,
        #[arg(long)]
        min_confidence: Option<u8>,
        #[arg(long, value_enum, default_value = "text")]
        format: OutputFormat,
        #[arg(long)]
        config: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
    Sarif,
}

impl From<OutputFormat> for Format {
    fn from(v: OutputFormat) -> Self {
        match v {
            OutputFormat::Text => Format::Text,
            OutputFormat::Json => Format::Json,
            OutputFormat::Sarif => Format::Sarif,
        }
    }
}

fn main() -> ExitCode {
    if let Err(e) = run() {
        eprintln!("error: {e:#}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Rules => {
            println!("PyDead rules (see docs/rules.md)\n");
            println!("Entry-point exemptions (keep definitions live):\n");
            for r in all_rules() {
                if r.code.starts_with("EP") {
                    println!("  {}  {:24}  {}", r.code, r.name, r.explanation);
                }
            }
            println!("\nDead-code findings (reported diagnostics):\n");
            for r in all_rules() {
                if r.code.starts_with("DC") {
                    println!("  {}  {:24}  {}", r.code, r.name, r.explanation);
                }
            }
            println!(
                "\nConfigure: ignore = [\"EP005\"], entry_names = [\"main\"], entry_decorators = [\"task\"]"
            );
            println!("Custom path rules: [[tool.pydead.entry_rules]]  (docs/rules.md)");
            Ok(())
        }
        Commands::Find {
            path,
            format,
            min_confidence,
            config,
        } => {
            let cfg = Config::load(&path, config.as_deref())?;
            let result = analyze(&AnalysisOptions {
                root: path,
                config: cfg,
                min_confidence,
            })?;
            let mut stdout = std::io::stdout().lock();
            write_report(&result, format.into(), &mut stdout)?;
            Ok(())
        }
        Commands::Fix {
            path,
            dry_run,
            yes,
            ids,
            min_confidence,
            format,
            config,
        } => {
            if !yes && !dry_run {
                anyhow::bail!("refusing to modify files without --yes (or pass --dry-run)");
            }
            let cfg = Config::load(&path, config.as_deref())?;
            let result = analyze(&AnalysisOptions {
                root: path.clone(),
                config: cfg,
                min_confidence,
            })?;

            let findings: Vec<_> = if ids.is_empty() {
                result.findings.clone()
            } else {
                let set: std::collections::HashSet<_> = ids.iter().cloned().collect();
                result
                    .findings
                    .iter()
                    .filter(|f| set.contains(&f.id))
                    .cloned()
                    .collect()
            };

            if findings.is_empty() {
                eprintln!("Nothing to fix.");
                return Ok(());
            }

            let root = result.root.clone();
            let fix = apply_fixes(&root, &findings, dry_run, Some(&result.file_hashes))?;

            match format {
                OutputFormat::Json => {
                    let payload = serde_json::json!({
                        "dry_run": fix.dry_run,
                        "files_changed": fix.files_changed,
                        "removed": fix.removed,
                        "skipped": fix.skipped,
                    });
                    println!("{}", serde_json::to_string_pretty(&payload)?);
                }
                _ => {
                    let action = if dry_run { "Would remove" } else { "Removed" };
                    for f in &fix.removed {
                        println!("{action} {} '{}' in {}", f.kind.as_str(), f.name, f.path);
                    }
                    for s in &fix.skipped {
                        eprintln!("skipped: {s}");
                    }
                    println!(
                        "\n{action} {} definition(s) across {} file(s).",
                        fix.removed.len(),
                        fix.files_changed
                    );
                    if dry_run {
                        println!("(dry-run: no files were written)");
                    } else {
                        println!("Tip: run Ruff to clean up any leftover unused imports.");
                    }
                }
            }
            Ok(())
        }
    }
}
