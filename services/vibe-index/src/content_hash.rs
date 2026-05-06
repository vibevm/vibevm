//! `compute_content_hash` — byte-for-byte port of `vibe-registry`'s
//! algorithm. PROP-005 §3.2 explained the duplicate-rather-than-import
//! decision: standalone redistribution beats workspace re-use, and a
//! parity test (`tests/content_hash_parity.rs`) gates divergence at
//! CI time.
//!
//! Algorithm:
//! 1. Walk `pkg_dir` recursively, filter to regular files.
//! 2. Sort the `Vec<PathBuf>` (lexicographic by `OsStr`).
//! 3. For each path: derive the relative path inside `pkg_dir`,
//!    normalise `\` → `/`, hash `(rel || 0x00 || file_bytes || 0x00)`.
//! 4. Final SHA-256, hex-encoded, prefixed with `"sha256:"`.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::error::{Error, Result};

pub fn compute_content_hash(pkg_dir: &Path) -> Result<String> {
    let mut files: Vec<PathBuf> = WalkDir::new(pkg_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect();
    files.sort();

    let mut hasher = Sha256::new();
    for path in &files {
        let rel = path.strip_prefix(pkg_dir).unwrap_or(path);
        let rel_normalised = rel.to_string_lossy().replace('\\', "/");
        hasher.update(rel_normalised.as_bytes());
        hasher.update([0]);
        let bytes = std::fs::read(path).map_err(|e| Error::Io {
            path: path.clone(),
            message: e.to_string(),
        })?;
        hasher.update(&bytes);
        hasher.update([0]);
    }
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(&mut hex, "{b:02x}");
    }
    Ok(format!("sha256:{hex}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn empty_directory_hashes_to_known_value() {
        let dir = tempdir().unwrap();
        let h = compute_content_hash(dir.path()).unwrap();
        // SHA-256 of zero bytes — empty stream produces empty SHA-256.
        assert_eq!(
            h,
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn single_file_hash_is_stable() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"hello\n").unwrap();
        let a = compute_content_hash(dir.path()).unwrap();
        let b = compute_content_hash(dir.path()).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn order_independent_of_walk_iteration() {
        let dir1 = tempdir().unwrap();
        fs::write(dir1.path().join("a.txt"), b"AAA").unwrap();
        fs::write(dir1.path().join("b.txt"), b"BBB").unwrap();

        let dir2 = tempdir().unwrap();
        fs::write(dir2.path().join("b.txt"), b"BBB").unwrap();
        fs::write(dir2.path().join("a.txt"), b"AAA").unwrap();

        assert_eq!(
            compute_content_hash(dir1.path()).unwrap(),
            compute_content_hash(dir2.path()).unwrap()
        );
    }

    #[test]
    fn different_content_produces_different_hash() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"AAA").unwrap();
        let h1 = compute_content_hash(dir.path()).unwrap();
        fs::write(dir.path().join("a.txt"), b"BBB").unwrap();
        let h2 = compute_content_hash(dir.path()).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn renaming_a_file_changes_hash() {
        let dir1 = tempdir().unwrap();
        fs::write(dir1.path().join("a.txt"), b"shared").unwrap();
        let dir2 = tempdir().unwrap();
        fs::write(dir2.path().join("b.txt"), b"shared").unwrap();
        assert_ne!(
            compute_content_hash(dir1.path()).unwrap(),
            compute_content_hash(dir2.path()).unwrap()
        );
    }

    #[test]
    fn nested_paths_round_trip() {
        let dir = tempdir().unwrap();
        let sub = dir.path().join("a/b/c");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("d.txt"), b"nested\n").unwrap();
        let h = compute_content_hash(dir.path()).unwrap();
        assert!(h.starts_with("sha256:"));
        assert_eq!(h.len(), 7 + 64); // "sha256:" + hex(32 bytes)
    }
}
