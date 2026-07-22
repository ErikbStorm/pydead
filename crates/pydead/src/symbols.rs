use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Kind of definition we track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefKind {
    Function,
    Class,
    Variable,
    Method,
}

impl DefKind {
    pub fn as_str(self) -> &'static str {
        match self {
            DefKind::Function => "function",
            DefKind::Class => "class",
            DefKind::Variable => "variable",
            DefKind::Method => "method",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    /// 0-based line (LSP).
    pub line: u32,
    /// 0-based character/column (LSP).
    pub character: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// A single dead-code finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub kind: DefKind,
    pub name: String,
    pub qualname: String,
    /// Path relative to analysis root, using `/` separators.
    pub path: String,
    pub range: Range,
    /// Inclusive byte offsets into the file (for fix).
    #[serde(skip_serializing, default)]
    pub byte_start: u32,
    #[serde(skip_serializing, default)]
    pub byte_end: u32,
    pub confidence: u8,
    /// Rule code (e.g. `DC001`) — Ruff-style.
    pub code: String,
    pub message: String,
    pub fixable: bool,
    /// SHA-256 of the whole file at analysis time (not serialized to JSON by default).
    #[serde(default, skip_serializing)]
    pub file_hash: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Stats {
    pub files: usize,
    pub definitions: usize,
    pub dead: usize,
    pub parse_errors: usize,
}

/// Internal definition collected during indexing.
#[derive(Debug, Clone)]
pub struct Definition {
    pub kind: DefKind,
    pub name: String,
    pub qualname: String,
    pub module: String,
    #[allow(dead_code)]
    pub path: PathBuf,
    pub rel_path: String,
    pub byte_start: u32,
    pub byte_end: u32,
    pub range: Range,
    pub is_private: bool,
    /// Decorator attribute / bare names (`route`, `field_validator`, …).
    pub decorator_attrs: Vec<String>,
}

/// A name that appears as a use (load / attribute / import target / __all__).
#[derive(Debug, Clone)]
pub struct NameUse {
    pub name: String,
    /// Optional module this use was resolved to (import target).
    #[allow(dead_code)]
    pub module: Option<String>,
    /// If this is an import of a specific symbol: (module, name)
    pub imported: Option<(String, String)>,
    /// Qualname of the enclosing function/method/class body, or `None` if the
    /// use appears at module level (always counts as a live root edge).
    pub container: Option<String>,
}
