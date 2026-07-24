//! Inline suppressions — mark a definition as intentionally not “unused”.
//!
//! Put a comment on the **def/class/assign line**, the **line above**, or any
//! **decorator line** stacked immediately above the definition:
//!
//! ```python
//! # pydead: keep
//! def leftover_for_plugin_system():
//!     ...
//!
//! def also_ok() -> None:  # noqa: DC001
//!     ...
//!
//! @app.task
//! def celery_job():  # pydead: used
//!     ...
//!
//! def method(self):  # pydead: ignore DC003
//!     ...
//! ```
//!
//! Suppress **all findings in a file** with a whole-line pragma (anywhere, usually
//! near the top):
//!
//! ```python
//! # pydead: ignore-file
//! ```
//!
//! | Form | Effect |
//! |------|--------|
//! | `# pydead: keep` / `# pydead: used` | Keep definition (all DC codes) |
//! | `# pydead: ignore` / `# noqa` | Same — suppress all DC on this def |
//! | `# pydead: keep DC003` / `# noqa: DC003` | Suppress only that finding code |
//! | `# pydead: ignore[DC001,DC003]` | Multiple codes |
//! | `# pydead: ignore-file` | Suppress all DC findings in the file |
//! | `# pydead: ignore-file DC001` | Suppress only that code in the file |

use std::collections::HashSet;

/// File-level ignore from a `# pydead: ignore-file` (etc.) whole-line pragma.
#[derive(Debug, Clone, Default)]
pub struct FileIgnore {
    pub suppress_all: bool,
    pub codes: HashSet<String>,
}

impl FileIgnore {
    pub fn suppresses(&self, code: &str) -> bool {
        if self.suppress_all {
            return true;
        }
        self.codes.iter().any(|c| c.eq_ignore_ascii_case(code))
    }
}

/// Parse a whole-file ignore pragma from source (`# pydead: ignore-file` …).
///
/// Only **comment-only** lines count (not inline trailing comments on code).
pub fn parse_file_ignore(source: &str) -> Option<FileIgnore> {
    let mut found: Option<FileIgnore> = None;
    for line in source.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if !t.starts_with('#') {
            continue;
        }
        if let Some(dir) = extract_file_ignore_directive(t) {
            found = Some(merge_file_ignore(found, dir));
        }
    }
    found
}

fn merge_file_ignore(existing: Option<FileIgnore>, next: FileIgnore) -> FileIgnore {
    match existing {
        None => next,
        Some(mut acc) => {
            if next.suppress_all || acc.suppress_all {
                acc.suppress_all = true;
                acc.codes.clear();
            } else {
                acc.codes.extend(next.codes);
            }
            acc
        }
    }
}

/// True if `code` is suppressed for this definition starting at `line_0based`.
///
/// Also returns true when the file has a matching `# pydead: ignore-file` pragma.
///
/// The analyzer precomputes file-level ignores and calls [`is_line_suppressed`];
/// this combined helper remains for tests and embedders.
#[allow(dead_code)]
pub fn is_suppressed(source: &str, line_0based: u32, code: &str) -> bool {
    if let Some(fi) = parse_file_ignore(source) {
        if fi.suppresses(code) {
            return true;
        }
    }
    is_line_suppressed(source, line_0based, code)
}

/// Per-definition keep/noqa only (skips file-level pragma — use when precomputed).
pub fn is_line_suppressed(source: &str, line_0based: u32, code: &str) -> bool {
    let lines: Vec<&str> = source.lines().collect();
    if lines.is_empty() {
        return false;
    }
    let start = line_0based as usize;
    if start >= lines.len() {
        return false;
    }

    // Walk upward through def line, decorator stack, blank lines, pure comments
    let mut idx = start;
    loop {
        if line_suppresses(lines[idx], code) {
            return true;
        }
        if idx == 0 {
            break;
        }
        let prev = lines[idx - 1];
        if is_continuation_line(prev) {
            idx -= 1;
            continue;
        }
        // One more chance: non-continuation previous line that is only a keep comment
        if line_suppresses(prev, code) {
            return true;
        }
        break;
    }
    false
}

/// Line is part of the definition header stack (decorators / blanks / comments).
fn is_continuation_line(line: &str) -> bool {
    let t = line.trim();
    if t.is_empty() {
        return true;
    }
    if t.starts_with('@') {
        return true;
    }
    if t.starts_with('#') {
        return true;
    }
    false
}

pub fn line_suppresses(line: &str, code: &str) -> bool {
    let Some(directive) = extract_ignore_directive(line) else {
        return false;
    };
    if directive.suppress_all {
        return true;
    }
    directive.codes.iter().any(|c| c.eq_ignore_ascii_case(code))
}

#[derive(Debug, Default)]
struct IgnoreDirective {
    suppress_all: bool,
    codes: HashSet<String>,
}

fn extract_ignore_directive(line: &str) -> Option<IgnoreDirective> {
    let hash = line.find('#')?;
    let comment = line[hash + 1..].trim();
    let lower = comment.to_ascii_lowercase();

    // File-level forms are handled separately; do not treat as per-def keep.
    if is_file_ignore_comment(&lower) {
        return None;
    }

    // # noqa ...
    if let Some(rest) = strip_prefix_ci(&lower, "noqa") {
        return Some(parse_codes_tail(rest));
    }

    // # pydead: ...
    if let Some(rest) = strip_prefix_ci(&lower, "pydead:") {
        let rest = rest.trim_start();
        for keyword in ["keep", "used", "ignore", "noqa", "allow"] {
            if let Some(rest) = strip_prefix_ci(rest, keyword) {
                // "ignore-file" starts with "ignore" — already filtered above
                return Some(parse_codes_tail(rest));
            }
        }
    }

    None
}

/// Whole-line `# pydead: ignore-file` / aliases.
fn extract_file_ignore_directive(comment_line: &str) -> Option<FileIgnore> {
    let t = comment_line.trim();
    if !t.starts_with('#') {
        return None;
    }
    let comment = t[1..].trim();
    let lower = comment.to_ascii_lowercase();

    let rest = strip_prefix_ci(&lower, "pydead:")?.trim_start();

    for keyword in ["ignore-file", "file-ignore", "noqa-file"] {
        if let Some(rest) = strip_prefix_ci(rest, keyword) {
            let dir = parse_codes_tail(rest);
            return Some(FileIgnore {
                suppress_all: dir.suppress_all,
                codes: dir.codes,
            });
        }
    }
    None
}

fn is_file_ignore_comment(lower_comment: &str) -> bool {
    let Some(rest) = strip_prefix_ci(lower_comment, "pydead:") else {
        return false;
    };
    let rest = rest.trim_start();
    for keyword in ["ignore-file", "file-ignore", "noqa-file"] {
        if strip_prefix_ci(rest, keyword).is_some() {
            return true;
        }
    }
    false
}

fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    let p = prefix.to_ascii_lowercase();
    if s.len() >= p.len() && s.as_bytes()[..p.len()].eq_ignore_ascii_case(p.as_bytes()) {
        Some(&s[p.len()..])
    } else {
        None
    }
}

fn parse_codes_tail(rest: &str) -> IgnoreDirective {
    let rest = rest.trim_start();
    if rest.is_empty() {
        return IgnoreDirective {
            suppress_all: true,
            codes: HashSet::new(),
        };
    }
    let rest = rest
        .trim_start_matches(':')
        .trim_start()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim();
    if rest.is_empty() {
        return IgnoreDirective {
            suppress_all: true,
            codes: HashSet::new(),
        };
    }
    let mut codes = HashSet::new();
    for part in rest.split(|c: char| c == ',' || c.is_whitespace()) {
        let p = part.trim().trim_matches(|c: char| c == '[' || c == ']');
        if !p.is_empty() {
            codes.insert(p.to_ascii_uppercase());
        }
    }
    if codes.is_empty() {
        IgnoreDirective {
            suppress_all: true,
            codes,
        }
    } else {
        IgnoreDirective {
            suppress_all: false,
            codes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keep_and_used() {
        assert!(line_suppresses("def f():  # pydead: keep", "DC001"));
        assert!(line_suppresses("def f():  # pydead: used", "DC003"));
        assert!(line_suppresses("def f():  # pydead: allow", "DC001"));
    }

    #[test]
    fn bare_noqa() {
        assert!(line_suppresses("    def foo():  # noqa", "DC001"));
    }

    #[test]
    fn specific_code() {
        assert!(line_suppresses("def f():  # noqa: DC003", "DC003"));
        assert!(!line_suppresses("def f():  # noqa: DC003", "DC001"));
    }

    #[test]
    fn decorator_stack() {
        let src = "\
# pydead: keep
@app.route(\"/x\")
def handler():
    pass
";
        assert!(is_suppressed(src, 2, "DC001"));
    }

    #[test]
    fn keep_on_def_with_decorators_above() {
        let src = "\
@field_validator(\"name\")
@classmethod
def check(cls, v):  # pydead: keep
    return v
";
        assert!(is_suppressed(src, 2, "DC003"));
    }

    #[test]
    fn ignore_file_suppresses_all() {
        let src = "\
# pydead: ignore-file
def orphan():
    pass
";
        assert!(is_suppressed(src, 1, "DC001"));
        let fi = parse_file_ignore(src).expect("file ignore");
        assert!(fi.suppress_all);
        assert!(fi.suppresses("DC004"));
    }

    #[test]
    fn ignore_file_specific_code() {
        let src = "\
# pydead: ignore-file DC001
def orphan():
    pass
";
        assert!(is_suppressed(src, 1, "DC001"));
        assert!(!is_suppressed(src, 1, "DC002"));
    }

    #[test]
    fn ignore_file_aliases() {
        assert!(parse_file_ignore("# pydead: file-ignore\nx=1\n").is_some());
        assert!(parse_file_ignore("# pydead: noqa-file\nx=1\n").is_some());
    }

    #[test]
    fn ignore_file_not_inline_on_code() {
        // Trailing comment on code must not count as file-level
        let src = "def f():  # pydead: ignore-file\n    pass\n";
        assert!(parse_file_ignore(src).is_none());
    }

    #[test]
    fn ignore_file_after_shebang() {
        let src = "\
#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# pydead: ignore-file

def orphan():
    pass
";
        assert!(is_suppressed(src, 4, "DC001"));
    }
}
