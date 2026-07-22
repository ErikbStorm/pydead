//! Shared EXPECTED.json assertions for integration fixtures.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use pydead::{analyze, AnalysisOptions, Config};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Expected {
    pub must_be_dead: Vec<ExpectedItem>,
    pub must_be_live: Vec<ExpectedItem>,
}

#[derive(Debug, Deserialize)]
pub struct ExpectedItem {
    pub kind: String,
    pub qualname: String,
}

pub fn fixture_root(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
        .canonicalize()
        .unwrap_or_else(|_| panic!("fixtures/{name} must exist"))
}

pub fn load_expected(root: &Path) -> Expected {
    let text = std::fs::read_to_string(root.join("EXPECTED.json")).expect("EXPECTED.json");
    serde_json::from_str(&text).expect("parse EXPECTED.json")
}

pub fn assert_expected(fixture: &str) {
    let root = fixture_root(fixture);
    let expected = load_expected(&root);
    let result = analyze(&AnalysisOptions {
        root,
        config: Config::default(),
        min_confidence: Some(70),
    })
    .unwrap_or_else(|e| panic!("analyze {fixture}: {e:#}"));

    assert_eq!(
        result.stats.parse_errors, 0,
        "{fixture}: parse errors"
    );

    let found: HashSet<String> = result
        .findings
        .iter()
        .map(|f| format!("{}\0{}", f.kind.as_str(), f.qualname))
        .collect();

    let mut false_dead = Vec::new();
    for item in &expected.must_be_live {
        let k = format!("{}\0{}", item.kind, item.qualname);
        if found.contains(&k) {
            false_dead.push(format!("{} {}", item.kind, item.qualname));
        }
    }
    assert!(
        false_dead.is_empty(),
        "{fixture}: incorrectly dead:\n  {}\n\nfindings:\n  {}",
        false_dead.join("\n  "),
        result
            .findings
            .iter()
            .map(|f| format!("{} {} {}", f.code, f.kind.as_str(), f.qualname))
            .collect::<Vec<_>>()
            .join("\n  ")
    );

    let mut missing = Vec::new();
    for item in &expected.must_be_dead {
        let k = format!("{}\0{}", item.kind, item.qualname);
        if !found.contains(&k) {
            missing.push(format!("{} {}", item.kind, item.qualname));
        }
    }
    assert!(
        missing.is_empty(),
        "{fixture}: missing dead:\n  {}\n\nfindings:\n  {}",
        missing.join("\n  "),
        result
            .findings
            .iter()
            .map(|f| format!("{} {} {}", f.code, f.kind.as_str(), f.qualname))
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}
