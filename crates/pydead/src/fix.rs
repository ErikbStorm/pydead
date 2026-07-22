use std::collections::HashMap;
use std::path::Path;

use crate::parse::expand_removal_bytes;
use crate::path_safety::{atomic_write, is_symlink, resolve_under_root, sha256_hex};
use crate::symbols::Finding;

#[derive(Debug, Clone)]
pub struct FixResult {
    pub removed: Vec<Finding>,
    pub files_changed: usize,
    pub dry_run: bool,
    pub skipped: Vec<String>,
    /// path → new content (or preview for dry-run)
    pub previews: HashMap<String, String>,
}

/// Apply fixes for the given findings under `root`.
///
/// * Paths must stay under `root` (no `..` / absolute / symlink escape).
/// * If `expected_hashes` is set (rel_path → sha256 hex of content at analyze
///   time), the file must still match or the file is skipped.
/// * Writes are atomic and refuse symlink targets.
pub fn apply_fixes(
    root: &Path,
    findings: &[Finding],
    dry_run: bool,
    expected_hashes: Option<&HashMap<String, String>>,
) -> anyhow::Result<FixResult> {
    let mut by_file: HashMap<String, Vec<&Finding>> = HashMap::new();
    for f in findings {
        if !f.fixable {
            continue;
        }
        by_file.entry(f.path.clone()).or_default().push(f);
    }

    let mut removed = Vec::new();
    let mut previews = HashMap::new();
    let mut skipped = Vec::new();
    let mut files_changed = 0;

    for (rel, mut items) in by_file {
        let path = match resolve_under_root(root, &rel) {
            Ok(p) => p,
            Err(e) => {
                skipped.push(format!("{rel}: path rejected ({e})"));
                continue;
            }
        };

        if is_symlink(&path) {
            skipped.push(format!("{rel}: refusing symlink"));
            continue;
        }

        let source = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                skipped.push(format!("{rel}: read failed ({e})"));
                continue;
            }
        };

        if let Some(hashes) = expected_hashes {
            let current = sha256_hex(source.as_bytes());
            match hashes.get(&rel) {
                Some(expected) if expected == &current => {}
                Some(_) => {
                    skipped.push(format!(
                        "{rel}: content changed since analysis (hash mismatch); re-run find"
                    ));
                    continue;
                }
                None => {
                    // Hash map present but file missing — still allow with warning path
                    // Prefer strict: skip
                    skipped.push(format!("{rel}: no content hash from analysis"));
                    continue;
                }
            }
        }

        // Prefer outer spans: sort by start asc, length desc; skip contained
        items.sort_by(|a, b| {
            a.byte_start
                .cmp(&b.byte_start)
                .then_with(|| b.byte_end.cmp(&a.byte_end))
        });

        let mut selected: Vec<&Finding> = Vec::new();
        for item in items {
            let contained = selected
                .iter()
                .any(|s| s.byte_start <= item.byte_start && s.byte_end >= item.byte_end);
            if !contained {
                selected.push(item);
            }
        }

        selected.sort_by(|a, b| b.byte_start.cmp(&a.byte_start));

        let mut new_source = source.clone();
        let mut file_removed = Vec::new();
        for item in &selected {
            let (start, end) = expand_removal_bytes(&new_source, item.byte_start, item.byte_end);
            let start = start as usize;
            let end = (end as usize).min(new_source.len());
            if start > new_source.len() || start > end {
                skipped.push(format!(
                    "{}:{}: invalid span after edits",
                    rel, item.name
                ));
                continue;
            }
            // Soft integrity: span should still contain the definition name
            let slice = &new_source[start..end];
            if !slice.contains(&item.name) {
                skipped.push(format!(
                    "{}:{}: span no longer contains name (file may have changed)",
                    rel, item.name
                ));
                continue;
            }
            new_source.replace_range(start..end, "");
            file_removed.push((*item).clone());
        }

        new_source = collapse_blank_lines(&new_source);

        if new_source != source && !file_removed.is_empty() {
            files_changed += 1;
            previews.insert(rel.clone(), new_source.clone());
            if !dry_run {
                atomic_write(&path, new_source.as_bytes()).map_err(|e| {
                    anyhow::anyhow!("failed to write {}: {e}", path.display())
                })?;
            }
            removed.extend(file_removed);
        }
    }

    removed.sort_by(|a, b| (&a.path, &a.name).cmp(&(&b.path, &b.name)));

    Ok(FixResult {
        removed,
        files_changed,
        dry_run,
        skipped,
        previews,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path_safety::sha256_hex;
    use crate::symbols::{DefKind, Position, Range};
    use std::fs;
    use std::path::PathBuf;

    fn finding(path: &str, name: &str, start: u32, end: u32, hash: &str) -> Finding {
        Finding {
            id: "t".into(),
            kind: DefKind::Function,
            name: name.into(),
            qualname: name.into(),
            path: path.into(),
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
            byte_start: start,
            byte_end: end,
            confidence: 90,
            code: "DC001".into(),
            message: "t".into(),
            fixable: true,
            file_hash: hash.into(),
        }
    }

    #[test]
    fn rejects_path_escape() {
        let dir = tempfile::tempdir().unwrap();
        let f = finding("../etc/passwd", "x", 0, 1, "abc");
        let r = apply_fixes(dir.path(), &[f], false, None).unwrap();
        assert!(r.files_changed == 0);
        assert!(!r.skipped.is_empty());
    }

    #[test]
    fn hash_mismatch_skips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.py");
        let src = "def dead():\n    pass\n";
        fs::write(&path, src).unwrap();
        let hash = sha256_hex(src.as_bytes());
        let mut hashes = HashMap::new();
        hashes.insert("a.py".into(), hash);
        // mutate file
        fs::write(&path, "def dead():\n    return 1\n").unwrap();
        let f = finding("a.py", "dead", 0, 20, "old");
        let r = apply_fixes(dir.path(), &[f], false, Some(&hashes)).unwrap();
        assert_eq!(r.files_changed, 0);
        assert!(r.skipped.iter().any(|s| s.contains("hash mismatch")));
    }

    #[allow(dead_code)]
    fn _pb() -> PathBuf {
        PathBuf::new()
    }
}

fn collapse_blank_lines(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut blank_run = 0;
    for line in s.split_inclusive('\n') {
        let is_blank = line.trim().is_empty();
        if is_blank {
            blank_run += 1;
            if blank_run <= 2 {
                out.push_str(line);
            }
        } else {
            blank_run = 0;
            out.push_str(line);
        }
    }
    out
}
