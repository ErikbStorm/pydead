//! Data-driven entry-point patterns (EP004–EP007 built-ins).
//!
//! Parse only records definition facts. Liveness decisions live here + user config
//! (see `rules::matching_entry_rule`).

use crate::config::{glob_match, Config};
use crate::symbols::Definition;

/// One built-in entry-point rule as pure data.
#[derive(Debug, Clone, Copy)]
pub struct EntryPattern {
    pub code: &'static str,
    pub name: &'static str,
    pub explanation: &'static str,
    /// Exact definition names (methods/functions/vars).
    pub names: &'static [&'static str],
    /// Decorator attribute / bare names (`field_validator`, `route`, …).
    pub decorators: &'static [&'static str],
    /// If non-empty, definition name must end with one of these.
    pub name_suffixes: &'static [&'static str],
    /// If non-empty, decorator attr must end with one of these.
    pub decorator_suffixes: &'static [&'static str],
    /// Optional path globs (relative path). Empty = any path.
    pub path_globs: &'static [&'static str],
}

pub const EP004: EntryPattern = EntryPattern {
    code: "EP004",
    name: "azure-functions-v2",
    explanation:
        "Azure Functions Python v2 host entry points (`@app.route`, `@app.timer_trigger`, `@app.activity_trigger`, `@bp.*`, any `*_trigger`, …).",
    names: &[],
    decorators: &[
        "route",
        "function_name",
        "http_type",
        "timer_trigger",
        "schedule",
        "queue_trigger",
        "queue_output",
        "blob_trigger",
        "blob_input",
        "blob_output",
        "table_input",
        "table_output",
        "service_bus_queue_trigger",
        "service_bus_topic_trigger",
        "service_bus_queue_output",
        "service_bus_topic_output",
        "event_hub_trigger",
        "event_hub_output",
        "event_grid_trigger",
        "event_grid_output",
        "cosmos_db_trigger",
        "cosmos_db_input",
        "cosmos_db_output",
        "sql_trigger",
        "sql_input",
        "sql_output",
        "mysql_trigger",
        "mysql_input",
        "mysql_output",
        "orchestration_trigger",
        "activity_trigger",
        "entity_trigger",
        "durable_client",
        "orchestration_client",
        "generic_trigger",
        "generic_input_binding",
        "generic_output_binding",
        "warmup_trigger",
    ],
    name_suffixes: &[],
    decorator_suffixes: &["_trigger"],
    path_globs: &[],
};

pub const EP005: EntryPattern = EntryPattern {
    code: "EP005",
    name: "alembic-migrations",
    explanation:
        "Alembic revision hooks: `upgrade`/`downgrade` and metadata under migration paths.",
    names: &[
        "upgrade",
        "downgrade",
        "revision",
        "down_revision",
        "branch_labels",
        "depends_on",
    ],
    decorators: &[],
    name_suffixes: &[],
    decorator_suffixes: &[],
    path_globs: &[
        "**/versions/*.py",
        "**/versions/**/*.py",
        "**/alembic/**/*.py",
        "**/migrations/versions/*.py",
        "**/migrations/**/*.py",
    ],
};

pub const EP006: EntryPattern = EntryPattern {
    code: "EP006",
    name: "pydantic-hooks",
    explanation: "Pydantic v1/v2 hooks: validators, serializers, computed fields, schema methods.",
    names: &[
        "model_post_init",
        "__get_pydantic_core_schema__",
        "__get_pydantic_json_schema__",
        "__get_validators__",
        "__modify_schema__",
        "__pydantic_init_subclass__",
    ],
    decorators: &[
        "field_validator",
        "model_validator",
        "field_serializer",
        "model_serializer",
        "computed_field",
        "validate_call",
        "validator",
        "root_validator",
        "validate_arguments",
    ],
    name_suffixes: &[],
    decorator_suffixes: &["_validator", "_serializer"],
    path_globs: &[],
};

pub const EP007: EntryPattern = EntryPattern {
    code: "EP007",
    name: "sqlalchemy-hooks",
    explanation:
        "SQLAlchemy TypeDecorator/TypeEngine hooks, ORM `@validates` / hybrids, `@compiles`, events.",
    names: &[
        "process_bind_param",
        "process_result_value",
        "process_literal_param",
        "load_dialect_impl",
        "compare_values",
        "coerce_compared_value",
        "bind_expression",
        "column_expression",
        "bind_processor",
        "result_processor",
        "literal_processor",
        "get_col_spec",
        "get_dbapi_type",
        "as_generic",
        "copy",
        "dialect_impl",
        "python_type",
        "comparator_factory",
        "adapt",
        "__mapper_args__",
        "__table_args__",
        "__tablename__",
        "__abstract__",
    ],
    decorators: &[
        "validates",
        "reconstructor",
        "declared_attr",
        "hybrid_property",
        "hybrid_method",
        "compiles",
        "listens_for",
    ],
    name_suffixes: &[],
    decorator_suffixes: &[],
    path_globs: &[],
};

/// Built-in framework/table patterns (not EP001–003 / EP010–012).
pub const BUILTIN_FRAMEWORK_PATTERNS: &[EntryPattern] = &[EP004, EP005, EP006, EP007];

impl EntryPattern {
    pub fn matches(&self, def: &Definition, path_globs_override: Option<&[String]>) -> bool {
        if !self.path_ok(def, path_globs_override) {
            return false;
        }
        if self.names.contains(&def.name.as_str()) {
            return true;
        }
        if self.name_suffixes.iter().any(|s| def.name.ends_with(s)) {
            return true;
        }
        for attr in &def.decorator_attrs {
            if self.decorators.contains(&attr.as_str()) {
                return true;
            }
            if self.decorator_suffixes.iter().any(|s| attr.ends_with(s)) {
                return true;
            }
        }
        false
    }

    fn path_ok(&self, def: &Definition, override_globs: Option<&[String]>) -> bool {
        let globs: Vec<&str> = if let Some(o) = override_globs {
            if o.is_empty() && self.path_globs.is_empty() {
                return true;
            }
            if !o.is_empty() {
                return path_matches_any(&def.rel_path, o);
            }
            self.path_globs.to_vec()
        } else if self.path_globs.is_empty() {
            return true;
        } else {
            self.path_globs.to_vec()
        };
        let owned: Vec<String> = globs.iter().map(|s| (*s).to_string()).collect();
        path_matches_any(&def.rel_path, &owned)
    }
}

/// Match relative path against glob patterns (`*`, `**` via globset).
pub fn path_matches_any(rel_path: &str, patterns: &[String]) -> bool {
    let path = rel_path.replace('\\', "/");
    for pat in patterns {
        if path_glob_match(pat, &path) {
            return true;
        }
    }
    false
}

pub fn path_glob_match(pattern: &str, path: &str) -> bool {
    let pat = pattern.replace('\\', "/");
    // globset: ** needs Glob::new; also try match against full path
    if let Ok(g) = globset::Glob::new(&pat) {
        let matcher = g.compile_matcher();
        if matcher.is_match(path) {
            return true;
        }
        // Also match if pattern is path-suffix style without leading **/
        if !pat.starts_with("**/") {
            if let Ok(g2) = globset::Glob::new(&format!("**/{pat}")) {
                if g2.compile_matcher().is_match(path) {
                    return true;
                }
            }
        }
    }
    // Fallback: simple name glob
    glob_match(&pat, path)
}

/// User config entry names (EP010).
pub fn matches_user_names(def: &Definition, config: &Config) -> bool {
    config
        .entry_names
        .iter()
        .any(|pat| pat == &def.name || glob_match(pat, &def.name))
}

/// User config entry decorators (EP011).
pub fn matches_user_decorators(def: &Definition, config: &Config) -> bool {
    for attr in &def.decorator_attrs {
        for pat in &config.entry_decorators {
            if pat == attr || glob_match(pat, attr) {
                return true;
            }
        }
    }
    false
}

/// Custom path-scoped rules (EP012 / user codes).
pub fn matches_custom_rule(def: &Definition, config: &Config) -> Option<String> {
    for rule in &config.entry_rules {
        let path_ok = rule.paths.is_empty() || path_matches_any(&def.rel_path, &rule.paths);
        if !path_ok {
            continue;
        }
        for pat in &rule.names {
            if pat == &def.name || glob_match(pat, &def.name) {
                return Some(rule.code.clone());
            }
        }
        for attr in &def.decorator_attrs {
            for pat in &rule.decorators {
                if pat == attr || glob_match(pat, attr) {
                    return Some(rule.code.clone());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbols::{DefKind, Position, Range};
    use std::path::PathBuf;

    fn def(name: &str, rel: &str, decos: &[&str]) -> Definition {
        Definition {
            kind: DefKind::Function,
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
            decorator_attrs: decos.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    #[test]
    fn azure_route() {
        assert!(EP004.matches(&def("http_hello", "function_app.py", &["route"]), None));
    }

    #[test]
    fn alembic_path_scoped() {
        assert!(EP005.matches(&def("upgrade", "alembic/versions/001.py", &[]), None));
        assert!(!EP005.matches(&def("upgrade", "app/service.py", &[]), None));
    }

    #[test]
    fn pydantic_field_validator() {
        assert!(EP006.matches(
            &def("check", "models.py", &["field_validator", "classmethod"]),
            None
        ));
    }

    #[test]
    fn sqlalchemy_type_hooks() {
        assert!(EP007.matches(&def("load_dialect_impl", "geo.py", &[]), None));
        assert!(EP007.matches(&def("bind_expression", "geo.py", &[]), None));
        assert!(EP007.matches(&def("column_expression", "geo.py", &[]), None));
    }
}
