use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rayon::prelude::*;
use sha2::{Digest, Sha256};

use crate::config::Config;
use crate::discover::discover_python_files;
use crate::ignore;
use crate::parse::{extract, parse_file};
use crate::path_safety::sha256_hex;
use crate::resolve::compute_live_qualnames;
use crate::rules::finding_code;
use crate::symbols::{Definition, Finding, Stats};

#[derive(Debug, Clone)]
pub struct AnalysisOptions {
    pub root: PathBuf,
    pub config: Config,
    /// Override min_confidence from config when set.
    pub min_confidence: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub root: PathBuf,
    pub findings: Vec<Finding>,
    pub stats: Stats,
    /// SHA-256 hex of each analyzed file's content at analysis time (rel_path → hash).
    /// Used by `apply_fixes` to refuse writes if the file changed.
    pub file_hashes: HashMap<String, String>,
}

pub fn analyze(opts: &AnalysisOptions) -> anyhow::Result<AnalysisResult> {
    let root = opts
        .root
        .canonicalize()
        .unwrap_or_else(|_| opts.root.clone());
    let min_conf = opts.min_confidence.unwrap_or(opts.config.min_confidence);

    let files = discover_python_files(&root, &opts.config)?;
    let mut stats = Stats {
        files: files.len(),
        ..Default::default()
    };

    // Parse in parallel
    let parsed: Vec<_> = files
        .par_iter()
        .map(|path| parse_file(&root, path).map_err(|e| (path.clone(), e)))
        .collect();

    let mut modules = Vec::new();
    for item in parsed {
        match item {
            Ok(m) => modules.push(m),
            Err((_path, _e)) => {
                stats.parse_errors += 1;
            }
        }
    }

    let mut all_definitions: Vec<Definition> = Vec::new();
    let mut all_uses = Vec::new();
    let mut all_exports_by_module: HashMap<String, Vec<String>> = HashMap::new();
    // rel_path → source (for noqa / pydead: ignore)
    let mut sources: HashMap<String, String> = HashMap::new();

    let mut file_hashes: HashMap<String, String> = HashMap::new();
    for module in &modules {
        sources.insert(module.rel_path.clone(), module.source.clone());
        file_hashes.insert(
            module.rel_path.clone(),
            sha256_hex(module.source.as_bytes()),
        );
        let extracted = extract(module);
        all_exports_by_module.insert(module.module.clone(), extracted.all_exports);
        all_definitions.extend(extracted.definitions);
        all_uses.extend(extracted.uses);
    }

    stats.definitions = all_definitions.len();
    let live = compute_live_qualnames(
        &all_definitions,
        &all_uses,
        &all_exports_by_module,
        &opts.config,
    );

    let mut findings = Vec::new();
    for def in &all_definitions {
        if live.contains(&def.qualname) {
            continue;
        }
        if opts.config.name_ignored(&def.name) {
            continue;
        }
        if opts.config.keep_public && !def.is_private {
            continue;
        }

        let rule = finding_code(def.kind);
        if !opts.config.rule_enabled(rule.code) {
            continue;
        }

        if let Some(src) = sources.get(&def.rel_path) {
            if ignore::is_suppressed(src, def.range.start.line, rule.code) {
                continue;
            }
        }

        let confidence = if def.is_private { 90 } else { 70 };
        if confidence < min_conf {
            continue;
        }

        let file_hash = file_hashes.get(&def.rel_path).cloned().unwrap_or_default();
        findings.push(definition_to_finding(def, confidence, file_hash));
    }

    // Stable order: path, then line, then name
    findings.sort_by(|a, b| {
        (&a.path, a.range.start.line, &a.name).cmp(&(&b.path, b.range.start.line, &b.name))
    });
    stats.dead = findings.len();

    Ok(AnalysisResult {
        root,
        findings,
        stats,
        file_hashes,
    })
}

fn definition_to_finding(def: &Definition, confidence: u8, file_hash: String) -> Finding {
    let id = finding_id(def);
    let kind_label = def.kind.as_str();
    let rule = finding_code(def.kind);
    Finding {
        id,
        kind: def.kind,
        name: def.name.clone(),
        qualname: def.qualname.clone(),
        path: def.rel_path.clone(),
        range: def.range,
        byte_start: def.byte_start,
        byte_end: def.byte_end,
        confidence,
        code: rule.code.to_string(),
        message: format!(
            "{} '{}' is never referenced in the workspace",
            capitalize(kind_label),
            def.name
        ),
        fixable: true,
        file_hash,
    }
}

fn finding_id(def: &Definition) -> String {
    let mut hasher = Sha256::new();
    hasher.update(def.rel_path.as_bytes());
    hasher.update(b"\0");
    hasher.update(def.kind.as_str().as_bytes());
    hasher.update(b"\0");
    hasher.update(def.qualname.as_bytes());
    hasher.update(b"\0");
    hasher.update(def.byte_start.to_le_bytes());
    hasher.update(def.byte_end.to_le_bytes());
    let digest = hasher.finalize();
    encode_hex(&digest[..16])
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0xf) as usize] as char);
    }
    s
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

/// Re-analyze a single root path with defaults (helper for tests / embedding).
#[allow(dead_code)]
pub fn analyze_path(root: impl AsRef<Path>) -> anyhow::Result<AnalysisResult> {
    analyze(&AnalysisOptions {
        root: root.as_ref().to_path_buf(),
        config: Config::default(),
        min_confidence: None,
    })
}
