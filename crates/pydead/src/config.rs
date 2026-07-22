use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::entry::EP005;

/// Custom path-scoped entry-point rule (EP012 / user codes).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct EntryRule {
    /// Rule code shown in docs (e.g. `EP100`).
    pub code: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub names: Vec<String>,
    #[serde(default)]
    pub decorators: Vec<String>,
    #[serde(default)]
    pub paths: Vec<String>,
}

/// Runtime configuration for analysis and fix.
#[derive(Debug, Clone)]
pub struct Config {
    pub min_confidence: u8,
    pub exclude: Vec<String>,
    pub ignore_names: Vec<String>,
    pub entry_modules: Vec<String>,
    pub keep_public: bool,
    /// Rule codes to disable (EP* or DC*).
    pub ignore: Vec<String>,
    /// Extra entry-point function name patterns (EP010).
    pub entry_names: Vec<String>,
    /// Extra decorator attribute patterns (EP011).
    pub entry_decorators: Vec<String>,
    /// Path-scoped custom entry rules (EP012).
    pub entry_rules: Vec<EntryRule>,
    /// Path globs for Alembic upgrade/downgrade (EP005).
    pub alembic_paths: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            min_confidence: 70,
            exclude: Vec::new(),
            ignore_names: Vec::new(),
            entry_modules: vec!["**/cli.py".into(), "**/__main__.py".into()],
            keep_public: false,
            ignore: Vec::new(),
            entry_names: Vec::new(),
            entry_decorators: Vec::new(),
            entry_rules: Vec::new(),
            alembic_paths: EP005.path_globs.iter().map(|s| (*s).to_string()).collect(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    #[serde(default)]
    tool: ToolSection,
    #[serde(default)]
    min_confidence: Option<u8>,
    #[serde(default)]
    exclude: Option<Vec<String>>,
    #[serde(default)]
    ignore_names: Option<Vec<String>>,
    #[serde(default)]
    entry_modules: Option<Vec<String>>,
    #[serde(default)]
    keep_public: Option<bool>,
    #[serde(default)]
    ignore: Option<Vec<String>>,
    #[serde(default)]
    entry_names: Option<Vec<String>>,
    #[serde(default)]
    entry_decorators: Option<Vec<String>>,
    #[serde(default)]
    entry_rules: Option<Vec<EntryRule>>,
    #[serde(default)]
    alembic_paths: Option<Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
struct ToolSection {
    #[serde(default)]
    pydead: Option<PydeadSection>,
}

#[derive(Debug, Default, Deserialize)]
struct PydeadSection {
    min_confidence: Option<u8>,
    exclude: Option<Vec<String>>,
    ignore_names: Option<Vec<String>>,
    entry_modules: Option<Vec<String>>,
    keep_public: Option<bool>,
    ignore: Option<Vec<String>>,
    entry_names: Option<Vec<String>>,
    entry_decorators: Option<Vec<String>>,
    entry_rules: Option<Vec<EntryRule>>,
    alembic_paths: Option<Vec<String>>,
}

impl Config {
    /// Load config from an explicit path, or discover `pydead.toml` /
    /// `pyproject.toml` under `root`.
    pub fn load(root: &Path, explicit: Option<&Path>) -> anyhow::Result<Self> {
        let mut cfg = Self::default();

        let path = if let Some(p) = explicit {
            Some(p.to_path_buf())
        } else {
            discover_config(root)
        };

        if let Some(path) = path {
            let text = std::fs::read_to_string(&path)?;
            let file: FileConfig = toml::from_str(&text)?;
            if let Some(section) = file.tool.pydead {
                apply_section(&mut cfg, section);
            } else {
                apply_section(
                    &mut cfg,
                    PydeadSection {
                        min_confidence: file.min_confidence,
                        exclude: file.exclude,
                        ignore_names: file.ignore_names,
                        entry_modules: file.entry_modules,
                        keep_public: file.keep_public,
                        ignore: file.ignore,
                        entry_names: file.entry_names,
                        entry_decorators: file.entry_decorators,
                        entry_rules: file.entry_rules,
                        alembic_paths: file.alembic_paths,
                    },
                );
            }
        }

        Ok(cfg)
    }

    pub fn name_ignored(&self, name: &str) -> bool {
        self.ignore_names.iter().any(|pat| glob_match(pat, name))
    }

    /// Whether a rule code is active (not listed in `ignore`).
    pub fn rule_enabled(&self, code: &str) -> bool {
        !self
            .ignore
            .iter()
            .any(|c| c.eq_ignore_ascii_case(code))
    }
}

fn apply_section(cfg: &mut Config, section: PydeadSection) {
    if let Some(v) = section.min_confidence {
        cfg.min_confidence = v;
    }
    if let Some(v) = section.exclude {
        cfg.exclude = v;
    }
    if let Some(v) = section.ignore_names {
        cfg.ignore_names = v;
    }
    if let Some(v) = section.entry_modules {
        cfg.entry_modules = v;
    }
    if let Some(v) = section.keep_public {
        cfg.keep_public = v;
    }
    if let Some(v) = section.ignore {
        cfg.ignore = v;
    }
    if let Some(v) = section.entry_names {
        cfg.entry_names = v;
    }
    if let Some(v) = section.entry_decorators {
        cfg.entry_decorators = v;
    }
    if let Some(v) = section.entry_rules {
        cfg.entry_rules = v;
    }
    if let Some(v) = section.alembic_paths {
        cfg.alembic_paths = v;
    }
}

fn discover_config(root: &Path) -> Option<PathBuf> {
    let pydead = root.join("pydead.toml");
    if pydead.is_file() {
        return Some(pydead);
    }
    let pyproject = root.join("pyproject.toml");
    if pyproject.is_file() {
        if let Ok(text) = std::fs::read_to_string(&pyproject) {
            if text.contains("[tool.pydead]") {
                return Some(pyproject);
            }
        }
    }
    None
}

/// Minimal glob: `*` matches any sequence; other chars are literal.
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let mut p = pattern.chars().peekable();
    let mut t = text.chars().peekable();
    let mut star: Option<(
        std::iter::Peekable<std::str::Chars>,
        std::iter::Peekable<std::str::Chars>,
    )> = None;

    loop {
        match (p.peek().copied(), t.peek().copied()) {
            (Some('*'), _) => {
                p.next();
                star = Some((p.clone(), t.clone()));
            }
            (Some(pc), Some(tc)) if pc == tc => {
                p.next();
                t.next();
            }
            (None, None) => return true,
            (_, Some(_)) if star.is_some() => {
                let (sp, mut st) = star.clone().unwrap();
                st.next();
                p = sp;
                t = st;
                star = Some((p.clone(), t.clone()));
            }
            _ => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_basics() {
        assert!(glob_match("visit_*", "visit_Name"));
        assert!(glob_match("test_*", "test_foo"));
        assert!(!glob_match("test_*", "foo_test"));
        assert!(glob_match("*", "anything"));
        assert!(glob_match("exact", "exact"));
        assert!(!glob_match("exact", "exact2"));
    }

    #[test]
    fn rule_enabled_respects_ignore() {
        let mut c = Config::default();
        assert!(c.rule_enabled("EP005"));
        c.ignore.push("EP005".into());
        assert!(!c.rule_enabled("EP005"));
        assert!(!c.rule_enabled("ep005"));
    }
}
