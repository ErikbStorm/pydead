//! Path jail: all reads/writes must stay under the analysis root.
//!
//! Rejects absolute relative segments, `..`, and (for writes) symlinks.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};

/// Max single Python source file size (bytes). Larger files are skipped.
pub const MAX_FILE_BYTES: u64 = 5 * 1024 * 1024; // 5 MiB

/// Max number of `.py` files to analyze in one run.
pub const MAX_FILES: usize = 50_000;

/// True if `rel` is a safe relative path (no absolute, no `..`, no empty).
pub fn is_safe_relative(rel: &str) -> bool {
    if rel.is_empty() {
        return false;
    }
    let p = Path::new(rel);
    if p.is_absolute() {
        return false;
    }
    // Windows drive-ish or root
    if rel.starts_with('/') || rel.starts_with('\\') {
        return false;
    }
    for c in p.components() {
        match c {
            Component::Normal(s) => {
                let s = s.to_string_lossy();
                if s == ".." || s.contains('\0') {
                    return false;
                }
            }
            Component::CurDir => {}
            Component::ParentDir => return false,
            Component::RootDir | Component::Prefix(_) => return false,
        }
    }
    true
}

/// Resolve `root/rel` only if the final path stays under `root`.
///
/// `root` should already be canonical when possible.
/// For existence checks, uses `canonicalize` when the path exists; otherwise
/// checks the parent and normalizes.
pub fn resolve_under_root(root: &Path, rel: &str) -> io::Result<PathBuf> {
    if !is_safe_relative(rel) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("path escapes analysis root or is invalid: {rel}"),
        ));
    }

    let root = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let joined = root.join(rel);

    // If path exists, canonicalize and require prefix
    if joined.exists() {
        let canon = fs::canonicalize(&joined)?;
        if !canon.starts_with(&root) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("path resolves outside analysis root: {rel}"),
            ));
        }
        return Ok(canon);
    }

    // Non-existent: ensure no symlink escape on parents
    let mut check = root.clone();
    for c in Path::new(rel).components() {
        if let Component::Normal(part) = c {
            check = check.join(part);
            if check.is_symlink() {
                let canon = fs::canonicalize(&check).unwrap_or(check.clone());
                if !canon.starts_with(&root) {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        format!("symlink path escapes analysis root: {rel}"),
                    ));
                }
            }
        }
    }
    Ok(joined)
}

/// True if path is a symlink (or any component makes it unsafe for write).
pub fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

/// SHA-256 hex of file bytes (for fix integrity).
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex_encode(&hasher.finalize())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0xf) as usize] as char);
    }
    s
}

/// Atomically replace file contents (write temp in same dir, rename).
/// Refuses to write through a symlink at the target path.
pub fn atomic_write(path: &Path, data: &[u8]) -> io::Result<()> {
    if is_symlink(path) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("refusing to write through symlink: {}", path.display()),
        ));
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut tmp_name = path
        .file_name()
        .map(|s| s.to_os_string())
        .unwrap_or_else(|| "pydead".into());
    tmp_name.push(".pydead.tmp");
    let tmp = parent.join(&tmp_name);

    // Clean stale temp
    let _ = fs::remove_file(&tmp);
    fs::write(&tmp, data)?;
    // On Unix rename is atomic replace
    fs::rename(&tmp, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })?;
    Ok(())
}

/// Skip files that are too large or not regular files.
pub fn file_allowed_for_read(path: &Path) -> io::Result<bool> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() {
        // Do not follow / read symlink targets as project source
        return Ok(false);
    }
    if !meta.is_file() {
        return Ok(false);
    }
    if meta.len() > MAX_FILE_BYTES {
        return Ok(false);
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn rejects_dotdot() {
        assert!(!is_safe_relative("../etc/passwd"));
        assert!(!is_safe_relative("foo/../../../etc"));
        assert!(!is_safe_relative("/abs"));
        assert!(is_safe_relative("pkg/mod.py"));
        assert!(is_safe_relative("a/b/c.py"));
    }

    #[test]
    fn resolve_under_root_ok() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::write(root.join("sub/a.py"), b"x = 1\n").unwrap();
        let p = resolve_under_root(root, "sub/a.py").unwrap();
        assert!(p.ends_with("a.py"));
        assert!(p.starts_with(fs::canonicalize(root).unwrap()));
    }

    #[test]
    fn resolve_rejects_escape() {
        let dir = tempfile::tempdir().unwrap();
        let err = resolve_under_root(dir.path(), "../x.py").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn atomic_write_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.py");
        atomic_write(&path, b"hello\n").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "hello\n");
        atomic_write(&path, b"world\n").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "world\n");
    }

    #[test]
    fn atomic_write_refuses_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("real.py");
        let link = dir.path().join("link.py");
        fs::write(&target, b"real\n").unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target, &link).unwrap();
            let err = atomic_write(&link, b"hack\n").unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
            assert_eq!(fs::read_to_string(&target).unwrap(), "real\n");
        }
        #[cfg(not(unix))]
        {
            let _ = (target, link);
        }
    }

    #[test]
    fn sha_stable() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    // silence unused Write in tests on non-unix
    #[allow(dead_code)]
    fn _w(w: &mut dyn Write) {
        let _ = w;
    }
}
