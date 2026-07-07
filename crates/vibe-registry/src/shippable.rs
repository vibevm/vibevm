//! The shippable tree (PROP-024 §2.2): identity is the source,
//! never build artifacts. The exclusion list MUST stay in lockstep
//! with `vibe-index`'s content_hash port (PROP-005 §3.2).

specmark::scope!("spec://vibevm/common/PROP-024#shippable-tree");

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::RegistryError;

/// Build-output dir/file names a package's shippable tree excludes (PROP-024
/// §2.2): identity is the source, not artifacts. MUST stay in lockstep with
/// the identical list in `vibe-index`'s content_hash port (PROP-005 §3.2).
const SHIPPABLE_EXCLUDES: &[&str] = &[".git", ".vibe", "target", "node_modules", ".vibeignore"];

/// Prune build output from a [`WalkDir`] walk so the hash and slot cover only
/// the shippable tree — per-entry, so an excluded dir is skipped, not entered.
fn is_shippable(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|name| !SHIPPABLE_EXCLUDES.contains(&name))
        .unwrap_or(true)
}

pub(crate) fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), RegistryError> {
    fs::create_dir_all(dst).map_err(|source| RegistryError::Io {
        path: dst.to_path_buf(),
        source,
    })?;
    for entry in WalkDir::new(src)
        .into_iter()
        .filter_entry(is_shippable)
        .filter_map(|e| e.ok())
    {
        let rel = entry.path().strip_prefix(src).unwrap_or(entry.path());
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).map_err(|source| RegistryError::Io {
                path: target.clone(),
                source,
            })?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|source| RegistryError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            fs::copy(entry.path(), &target).map_err(|source| RegistryError::Io {
                path: target.clone(),
                source,
            })?;
        }
    }
    Ok(())
}

/// sha256 of concatenated (rel_path_bytes || 0x00 || file_bytes || 0x00) for
/// every file in the package, traversed in sorted order for determinism.
///
/// This is the **identity** half of the `(group, name, version,
/// content_hash)` tuple (PROP-002 §2.1). Reads every file under
/// `pkg_dir`:
///
/// ```no_run
/// use std::path::{Path, PathBuf};
/// use vibe_registry::compute_content_hash;
///
/// let hash = compute_content_hash(Path::new("path/to/package")).unwrap();
/// assert!(hash.starts_with("sha256:"));
/// ```
pub fn compute_content_hash(pkg_dir: &Path) -> Result<String, RegistryError> {
    let mut files: Vec<PathBuf> = WalkDir::new(pkg_dir)
        .into_iter()
        .filter_entry(is_shippable)
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect();
    files.sort();

    let mut hasher = Sha256::new();
    for path in &files {
        let rel = path.strip_prefix(pkg_dir).unwrap_or(path);
        let rel_normalized = rel.to_string_lossy().replace('\\', "/");
        hasher.update(rel_normalized.as_bytes());
        hasher.update([0]);
        let bytes = fs::read(path).map_err(|source| RegistryError::Io {
            path: path.clone(),
            source,
        })?;
        hasher.update(&bytes);
        hasher.update([0]);
    }
    let digest = hasher.finalize();
    let hex = digest.iter().fold(String::new(), |mut s, b| {
        use std::fmt::Write;
        let _ = write!(&mut s, "{b:02x}");
        s
    });
    Ok(format!("sha256:{hex}"))
}

#[cfg(test)]
mod shippable_tree_tests {
    use super::*;
    use tempfile::tempdir;

    /// `copy_dir_recursive` and `compute_content_hash` both skip build output,
    /// so neither the materialised slot nor the content hash carries `target/`
    /// & friends (PROP-024 §2.2). Inline because `copy_dir_recursive` is
    /// `pub(crate)` — a `tests/` target could not reach it.
    #[test]
    fn copy_and_hash_exclude_build_output() {
        let src = tempdir().unwrap();
        fs::write(src.path().join("vibe.toml"), b"x").unwrap();
        fs::create_dir_all(src.path().join("target/debug")).unwrap();
        fs::write(src.path().join("target/debug/x.bin"), b"ARTIFACT").unwrap();

        let dst = tempdir().unwrap();
        copy_dir_recursive(src.path(), dst.path()).unwrap();
        assert!(dst.path().join("vibe.toml").exists());
        assert!(
            !dst.path().join("target").exists(),
            "target/ must not be copied"
        );

        let clean = tempdir().unwrap();
        fs::write(clean.path().join("vibe.toml"), b"x").unwrap();
        assert_eq!(
            compute_content_hash(src.path()).unwrap(),
            compute_content_hash(clean.path()).unwrap(),
            "build output must not affect the content hash"
        );
    }
}
