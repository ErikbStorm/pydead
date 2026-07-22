//! Rule catalog (EP / DC codes) and entry-point matching.

use crate::config::Config;
use crate::entry::{
    matches_custom_rule, matches_user_decorators, matches_user_names, BUILTIN_FRAMEWORK_PATTERNS,
};
use crate::parse::is_dunder;
use crate::symbols::{DefKind, Definition};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleMeta {
    pub code: &'static str,
    pub name: &'static str,
    pub explanation: &'static str,
}

pub const DC001: RuleMeta = RuleMeta {
    code: "DC001",
    name: "unused-function",
    explanation: "Function is never referenced in the analysis root.",
};
pub const DC002: RuleMeta = RuleMeta {
    code: "DC002",
    name: "unused-class",
    explanation: "Class is never referenced in the analysis root.",
};
pub const DC003: RuleMeta = RuleMeta {
    code: "DC003",
    name: "unused-method",
    explanation: "Method is never referenced in the analysis root.",
};
pub const DC004: RuleMeta = RuleMeta {
    code: "DC004",
    name: "unused-variable",
    explanation: "Module-level variable is never referenced in the analysis root.",
};

pub const EP001: RuleMeta = RuleMeta {
    code: "EP001",
    name: "dunder-names",
    explanation: "Dunder names like `__init__` / `__str__` are protocol hooks.",
};
pub const EP002: RuleMeta = RuleMeta {
    code: "EP002",
    name: "test-discovery",
    explanation: "Pytest/unittest discovery names: `test_*`, `Test*`, `pytest_*`.",
};
pub const EP003: RuleMeta = RuleMeta {
    code: "EP003",
    name: "dunder-all-exports",
    explanation: "Names listed in module `__all__` are part of the public API.",
};
pub const EP010: RuleMeta = RuleMeta {
    code: "EP010",
    name: "user-entry-names",
    explanation: "Names / globs listed in config `entry_names`.",
};
pub const EP011: RuleMeta = RuleMeta {
    code: "EP011",
    name: "user-entry-decorators",
    explanation: "Decorator attribute names listed in config `entry_decorators`.",
};
pub const EP012: RuleMeta = RuleMeta {
    code: "EP012",
    name: "user-entry-rules",
    explanation: "Custom path-scoped name rules from `[[tool.pydead.entry_rules]]`.",
};

/// Re-export framework pattern metadata as RuleMeta for `pydead rules`.
fn framework_meta(p: &crate::entry::EntryPattern) -> RuleMeta {
    RuleMeta {
        code: p.code,
        name: p.name,
        explanation: p.explanation,
    }
}

pub fn all_rules() -> Vec<RuleMeta> {
    let mut v = vec![EP001, EP002, EP003];
    for p in BUILTIN_FRAMEWORK_PATTERNS {
        v.push(framework_meta(p));
    }
    v.extend([EP010, EP011, EP012, DC001, DC002, DC003, DC004]);
    v
}

pub fn finding_code(kind: DefKind) -> RuleMeta {
    match kind {
        DefKind::Function => DC001,
        DefKind::Class => DC002,
        DefKind::Method => DC003,
        DefKind::Variable => DC004,
    }
}

/// First matching entry-point rule code, or None.
pub fn matching_entry_rule(def: &Definition, config: &Config) -> Option<String> {
    if config.rule_enabled("EP001") && is_dunder(&def.name) {
        return Some(EP001.code.into());
    }

    if config.rule_enabled("EP002")
        && (def.name.starts_with("test_")
            || def.name.starts_with("pytest_")
            || (def.name.starts_with("Test") && matches!(def.kind, DefKind::Class)))
    {
        return Some(EP002.code.into());
    }

    // Framework table (EP004–EP007)
    for pat in BUILTIN_FRAMEWORK_PATTERNS {
        if !config.rule_enabled(pat.code) {
            continue;
        }
        let path_override = if pat.code == "EP005" {
            Some(config.alembic_paths.as_slice())
        } else {
            None
        };
        if pat.matches(def, path_override) {
            return Some(pat.code.into());
        }
    }

    if config.rule_enabled("EP010") && matches_user_names(def, config) {
        return Some(EP010.code.into());
    }
    if config.rule_enabled("EP011") && matches_user_decorators(def, config) {
        return Some(EP011.code.into());
    }
    if config.rule_enabled("EP012") {
        if let Some(code) = matches_custom_rule(def, config) {
            return Some(code);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbols::{Position, Range};
    use std::path::PathBuf;

    fn def(name: &str, rel: &str, kind: DefKind) -> Definition {
        Definition {
            kind,
            name: name.into(),
            qualname: format!("m.{name}"),
            module: "m".into(),
            path: PathBuf::from(rel),
            rel_path: rel.into(),
            byte_start: 0,
            byte_end: 1,
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 1,
                },
            },
            is_private: false,
            decorator_attrs: vec![],
        }
    }

    // entry tests cover framework patterns; keep forced_live gone
    #[test]
    fn sqlalchemy_via_table() {
        let mut d = def("load_dialect_impl", "t.py", DefKind::Method);
        d.decorator_attrs = vec![];
        assert_eq!(
            matching_entry_rule(&d, &Config::default()).as_deref(),
            Some("EP007")
        );
    }

    #[test]
    fn alembic_upgrade_matches() {
        let cfg = Config::default();
        let d = def(
            "upgrade",
            "alembic/versions/001_initial.py",
            DefKind::Function,
        );
        assert_eq!(matching_entry_rule(&d, &cfg).as_deref(), Some("EP005"));
        let d2 = def("upgrade", "app/service.py", DefKind::Function);
        assert_eq!(matching_entry_rule(&d2, &cfg), None);
    }

    #[test]
    fn user_entry_names() {
        let cfg = Config {
            entry_names: vec!["main".into(), "run_*".into()],
            ..Default::default()
        };
        assert_eq!(
            matching_entry_rule(&def("main", "cli.py", DefKind::Function), &cfg).as_deref(),
            Some("EP010")
        );
        assert_eq!(
            matching_entry_rule(&def("run_job", "jobs.py", DefKind::Function), &cfg).as_deref(),
            Some("EP010")
        );
    }

    #[test]
    fn ignore_disables_ep005() {
        let cfg = Config {
            ignore: vec!["EP005".into()],
            ..Default::default()
        };
        let d = def(
            "upgrade",
            "alembic/versions/001_initial.py",
            DefKind::Function,
        );
        assert_eq!(matching_entry_rule(&d, &cfg), None);
    }
}
