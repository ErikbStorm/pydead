//! Cross-file Python dead-code analysis.
//!
//! Scans a folder of Python files, builds a definition/reference index across
//! packages, and reports unused functions, classes, methods, and module-level
//! variables. Designed for monorepos where Ruff's per-file checks are not enough.

mod analyze;
mod config;
mod discover;
mod entry;
mod fix;
mod ignore;
mod parse;
mod path_safety;
mod report;
mod resolve;
mod rules;
mod symbols;

pub use analyze::{analyze, AnalysisOptions, AnalysisResult};
pub use config::{Config, EntryRule};
pub use fix::{apply_fixes, FixResult};
pub use path_safety::{is_safe_relative, resolve_under_root, MAX_FILES, MAX_FILE_BYTES};
pub use report::{write_report, Format, Report};
pub use rules::{all_rules, RuleMeta};
pub use symbols::{DefKind, Finding, Position, Range, Stats};
