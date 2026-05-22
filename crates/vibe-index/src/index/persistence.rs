//! Low-level on-disk helpers — atomic file replace + sha256 hex.
//!
//! Reused by every per-file writer module under `index/`. Splitting
//! these out keeps the per-format code (primary.jsonl, by-name JSON,
//! repomd.json) tightly focused on its own shape.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;

use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

/// Write `bytes` to `path` atomically (tmp + fsync + rename). Creates
/// the parent directory when missing.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| io_err(parent, e))?;
    }
    let tmp = tmp_sibling(path);
    {
        let mut f = fs::File::create(&tmp).map_err(|e| io_err(&tmp, e))?;
        f.write_all(bytes).map_err(|e| io_err(&tmp, e))?;
        f.sync_all().map_err(|e| io_err(&tmp, e))?;
    }
    fs::rename(&tmp, path).map_err(|e| io_err(path, e))?;
    Ok(())
}

fn tmp_sibling(path: &Path) -> PathBuf {
    let pid = process::id();
    let mut name = path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(format!(".tmp.{pid}"));
    let parent = path.parent().unwrap_or(Path::new("."));
    parent.join(name)
}

fn io_err(path: &Path, source: std::io::Error) -> Error {
    Error::Io {
        path: path.to_path_buf(),
        message: source.to_string(),
    }
}

/// `sha256:<hex>` over the supplied bytes — same prefix scheme
/// `vibe-registry::compute_content_hash` uses on package content so
/// the two cross-check byte-for-byte at integration time.
pub fn sha256_of_bytes(bytes: &[u8]) -> String {
    format!("sha256:{}", compute_sha256_hex(bytes))
}

pub fn compute_sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut s = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(&mut s, "{b:02x}");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn atomic_write_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("a/b/c/file.json");
        atomic_write(&path, b"hello").unwrap();
        let read = fs::read_to_string(&path).unwrap();
        assert_eq!(read, "hello");
    }

    #[test]
    fn atomic_write_replaces_existing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("file.json");
        atomic_write(&path, b"first").unwrap();
        atomic_write(&path, b"second").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "second");
    }

    #[test]
    fn atomic_write_does_not_leave_tmp_behind() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("file.json");
        atomic_write(&path, b"x").unwrap();
        let entries: Vec<_> = fs::read_dir(dir.path()).unwrap().collect();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn sha256_of_bytes_carries_prefix() {
        let h = sha256_of_bytes(b"");
        assert_eq!(
            h,
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_is_deterministic() {
        let a = sha256_of_bytes(b"vibevm");
        let b = sha256_of_bytes(b"vibevm");
        assert_eq!(a, b);
    }
}
