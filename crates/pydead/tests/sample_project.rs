//! Integration tests against fixtures/sample_project.
//!
//! Ground truth lives in EXPECTED.json — every must_be_dead entry must appear
//! in findings, and no must_be_live entry may appear.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use pydead::{analyze, apply_fixes, AnalysisOptions, Config};
use serde::Deserialize;

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/sample_project")
        .canonicalize()
        .expect("fixtures/sample_project must exist")
}

#[derive(Debug, Deserialize)]
struct Expected {
    must_be_dead: Vec<ExpectedItem>,
    must_be_live: Vec<ExpectedItem>,
}

#[derive(Debug, Deserialize, Clone)]
struct ExpectedItem {
    kind: String,
    qualname: String,
}

fn load_expected() -> Expected {
    let path = fixture_root().join("EXPECTED.json");
    let text = std::fs::read_to_string(&path).expect("EXPECTED.json");
    serde_json::from_str(&text).expect("parse EXPECTED.json")
}

fn key(kind: &str, qualname: &str) -> String {
    format!("{kind}\0{qualname}")
}

#[test]
fn sample_project_finds_expected_dead_code() {
    let root = fixture_root();
    let expected = load_expected();

    let result = analyze(&AnalysisOptions {
        root: root.clone(),
        config: Config::default(),
        min_confidence: Some(70),
    })
    .expect("analyze sample_project");

    assert!(
        result.stats.files >= 6,
        "expected multiple python files, got {}",
        result.stats.files
    );
    assert_eq!(result.stats.parse_errors, 0, "fixture should parse cleanly");

    let found: HashSet<String> = result
        .findings
        .iter()
        .map(|f| key(f.kind.as_str(), &f.qualname))
        .collect();

    let mut missing = Vec::new();
    for item in &expected.must_be_dead {
        let k = key(&item.kind, &item.qualname);
        if !found.contains(&k) {
            missing.push(format!("{} {}", item.kind, item.qualname));
        }
    }
    assert!(
        missing.is_empty(),
        "missing expected dead definitions:\n  {}\n\nactual findings:\n  {}",
        missing.join("\n  "),
        result
            .findings
            .iter()
            .map(|f| format!("{} {}", f.kind.as_str(), f.qualname))
            .collect::<Vec<_>>()
            .join("\n  ")
    );

    let mut false_dead = Vec::new();
    for item in &expected.must_be_live {
        let k = key(&item.kind, &item.qualname);
        if found.contains(&k) {
            false_dead.push(format!("{} {}", item.kind, item.qualname));
        }
    }
    assert!(
        false_dead.is_empty(),
        "live definitions incorrectly reported dead:\n  {}",
        false_dead.join("\n  ")
    );
}

#[test]
fn sample_project_fix_removes_dead_code() {
    let root = fixture_root();
    let tmp = tempfile::tempdir().expect("tempdir");
    copy_dir(&root, tmp.path()).expect("copy fixture");

    let result = analyze(&AnalysisOptions {
        root: tmp.path().to_path_buf(),
        config: Config::default(),
        min_confidence: Some(70),
    })
    .expect("analyze copy");

    assert!(!result.findings.is_empty(), "expected dead code before fix");

    let fix = apply_fixes(
        tmp.path(),
        &result.findings,
        false,
        Some(&result.file_hashes),
    )
    .expect("apply_fixes");
    assert!(fix.files_changed > 0, "expected files to change");
    assert!(!fix.removed.is_empty());

    // Greeter must still be present in api.py
    let api = std::fs::read_to_string(tmp.path().join("libs/core/api.py")).unwrap();
    assert!(
        api.contains("class Greeter"),
        "live class Greeter must remain after fix"
    );
    assert!(
        api.contains("def greet"),
        "live method greet must remain after fix"
    );
    assert!(
        !api.contains("class DeadService"),
        "DeadService should be removed"
    );
    assert!(
        !api.contains("orphan_public"),
        "orphan_public should be removed"
    );

    // Re-analyze: expected dead set from original should be gone (or mostly)
    let after = analyze(&AnalysisOptions {
        root: tmp.path().to_path_buf(),
        config: Config::default(),
        min_confidence: Some(70),
    })
    .expect("re-analyze");

    let expected = load_expected();
    let after_keys: HashSet<String> = after
        .findings
        .iter()
        .map(|f| key(f.kind.as_str(), &f.qualname))
        .collect();

    for item in &expected.must_be_dead {
        let k = key(&item.kind, &item.qualname);
        assert!(
            !after_keys.contains(&k),
            "still reported dead after fix: {} {}",
            item.kind,
            item.qualname
        );
    }

    // Live symbols still not reported
    for item in &expected.must_be_live {
        // Some live symbols may have been methods inside removed classes — skip those
        if item.qualname.contains("DeadService") {
            continue;
        }
        let k = key(&item.kind, &item.qualname);
        assert!(
            !after_keys.contains(&k),
            "live symbol reported dead after fix: {} {}",
            item.kind,
            item.qualname
        );
    }
}

fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
