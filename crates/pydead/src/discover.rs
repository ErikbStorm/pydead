use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use crate::config::Config;
use crate::path_safety::{file_allowed_for_read, MAX_FILES};

const DEFAULT_SKIP_DIRS: &[&str] = &[
    "venv",
    ".venv",
    "env",
    ".env",
    "__pycache__",
    ".git",
    "node_modules",
    "dist",
    "build",
    ".tox",
    ".mypy_cache",
    ".ruff_cache",
    "site-packages",
    ".pytest_cache",
    "target",
];

/// Discover Python source files under `root`.
///
/// Does **not** follow symlinks. Skips oversized files and caps total count.
pub fn discover_python_files(root: &Path, config: &Config) -> anyhow::Result<Vec<PathBuf>> {
    let mut builder = WalkBuilder::new(root);
    builder.hidden(false);
    builder.git_ignore(true);
    builder.git_global(false);
    builder.parents(true);
    builder.follow_links(false);
    // Don't traverse into symlink dirs either
    builder.same_file_system(false);

    let mut files = Vec::new();
    let mut skipped_large = 0u32;
    let mut skipped_symlink = 0u32;

    for entry in builder.build() {
        let entry = entry?;
        let path = entry.path();
        if entry.path_is_symlink() {
            skipped_symlink += 1;
            continue;
        }
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("py") {
            continue;
        }
        if should_skip(path, root, config) {
            continue;
        }
        match file_allowed_for_read(path) {
            Ok(true) => {}
            Ok(false) => {
                // symlink or too large
                if path
                    .symlink_metadata()
                    .map(|m| m.file_type().is_symlink())
                    .unwrap_or(false)
                {
                    skipped_symlink += 1;
                } else {
                    skipped_large += 1;
                }
                continue;
            }
            Err(_) => continue,
        }
        files.push(path.to_path_buf());
        if files.len() >= MAX_FILES {
            eprintln!("warning: reached max file limit ({MAX_FILES}); remaining files skipped");
            break;
        }
    }
    if skipped_large > 0 {
        eprintln!(
            "warning: skipped {skipped_large} oversized Python file(s) (>{limit} bytes)",
            limit = crate::path_safety::MAX_FILE_BYTES
        );
    }
    if skipped_symlink > 0 {
        eprintln!("warning: skipped {skipped_symlink} symlink path(s)");
    }
    files.sort();
    Ok(files)
}

fn should_skip(path: &Path, root: &Path, config: &Config) -> bool {
    let rel = path.strip_prefix(root).unwrap_or(path);
    let rel_str = rel.to_string_lossy();

    for part in rel.components() {
        if let std::path::Component::Normal(name) = part {
            let name = name.to_string_lossy();
            if DEFAULT_SKIP_DIRS.iter().any(|d| *d == name) {
                return true;
            }
        }
    }

    for pat in &config.exclude {
        if crate::config::glob_match(pat, &rel_str)
            || crate::config::glob_match(pat, &path.to_string_lossy())
        {
            return true;
        }
        if let Some(rest) = pat.strip_prefix("**/") {
            if rel_str.contains(rest.trim_end_matches("/**").trim_end_matches("/**/"))
                || path_matches_glob_path(rel, rest)
            {
                return true;
            }
        }
    }

    false
}

fn path_matches_glob_path(rel: &Path, pattern: &str) -> bool {
    let pat = pattern.trim_end_matches('/').trim_end_matches("**");
    let pat = pat.trim_end_matches('/');
    if pat.is_empty() {
        return false;
    }
    rel.to_string_lossy().contains(pat)
}

/// Map a file path under root to a dotted module name.
pub fn path_to_module(root: &Path, file: &Path) -> String {
    let rel = file.strip_prefix(root).unwrap_or(file);
    let mut parts: Vec<String> = rel
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();

    if let Some(last) = parts.last_mut() {
        if let Some(stem) = last.strip_suffix(".py") {
            *last = stem.to_string();
        }
    }
    if parts.last().map(|s| s.as_str()) == Some("__init__") {
        parts.pop();
    }
    parts.join(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_names() {
        let root = Path::new("/proj");
        assert_eq!(
            path_to_module(root, Path::new("/proj/libs/core/api.py")),
            "libs.core.api"
        );
        assert_eq!(
            path_to_module(root, Path::new("/proj/libs/core/__init__.py")),
            "libs.core"
        );
        assert_eq!(path_to_module(root, Path::new("/proj/main.py")), "main");
    }
}
