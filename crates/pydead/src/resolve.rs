use std::collections::{HashMap, HashSet};

use crate::config::Config;
use crate::rules::matching_entry_rule;
use crate::symbols::{Definition, NameUse};

/// Whether a definition is always live regardless of references.
pub fn is_always_live(def: &Definition, config: &Config) -> bool {
    matching_entry_rule(def, config).is_some()
}

/// Compute the set of live definition qualnames via iterative reachability.
///
/// Module-level uses (container = None) and uses inside already-live
/// definitions keep marking more definitions live until a fixpoint.
pub fn compute_live_qualnames(
    definitions: &[Definition],
    uses: &[NameUse],
    all_exports_by_module: &HashMap<String, Vec<String>>,
    config: &Config,
) -> HashSet<String> {
    let mut live: HashSet<String> = HashSet::new();

    // Seed: entry-point EP rules
    for def in definitions {
        if is_always_live(def, config) {
            live.insert(def.qualname.clone());
        }
    }

    // EP003 — __all__ exports
    if config.rule_enabled("EP003") {
        for (module, names) in all_exports_by_module {
            for n in names {
                for def in definitions {
                    if &def.module == module && &def.name == n {
                        live.insert(def.qualname.clone());
                    }
                }
            }
        }
    }

    let mut by_name: HashMap<&str, Vec<&Definition>> = HashMap::new();
    for def in definitions {
        by_name.entry(def.name.as_str()).or_default().push(def);
    }

    let mut changed = true;
    while changed {
        changed = false;

        for u in uses {
            let active = match &u.container {
                None => true,
                Some(c) => live.contains(c),
            };
            if !active {
                continue;
            }

            if let Some((ref mod_name, ref sym)) = u.imported {
                for def in definitions {
                    if &def.module == mod_name
                        && &def.name == sym
                        && live.insert(def.qualname.clone())
                    {
                        changed = true;
                    }
                }
            }

            if let Some(defs) = by_name.get(u.name.as_str()) {
                for def in defs {
                    if live.insert(def.qualname.clone()) {
                        changed = true;
                    }
                }
            }
        }
    }

    live
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbols::{DefKind, Position, Range};
    use std::path::PathBuf;

    fn def(kind: DefKind, name: &str, module: &str) -> Definition {
        let qualname = format!("{module}.{name}");
        Definition {
            kind,
            name: name.to_string(),
            qualname,
            module: module.to_string(),
            path: PathBuf::from("x.py"),
            rel_path: "x.py".into(),
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
            is_private: name.starts_with('_'),
            decorator_attrs: vec![],
        }
    }

    #[test]
    fn iterative_marks_only_from_live() {
        let defs = vec![
            def(DefKind::Function, "entry", "app"),
            def(DefKind::Function, "helper", "app"),
            def(DefKind::Function, "orphan", "app"),
            def(DefKind::Class, "OnlyUsedByOrphan", "app"),
        ];
        let uses = vec![
            NameUse {
                name: "entry".into(),
                module: None,
                imported: None,
                container: None,
            },
            NameUse {
                name: "helper".into(),
                module: None,
                imported: None,
                container: Some("app.entry".into()),
            },
            NameUse {
                name: "OnlyUsedByOrphan".into(),
                module: None,
                imported: None,
                container: Some("app.orphan".into()),
            },
        ];
        let live = compute_live_qualnames(&defs, &uses, &HashMap::new(), &Config::default());
        assert!(live.contains("app.entry"));
        assert!(live.contains("app.helper"));
        assert!(!live.contains("app.orphan"));
        assert!(!live.contains("app.OnlyUsedByOrphan"));
    }
}
