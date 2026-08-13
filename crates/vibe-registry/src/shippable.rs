//! The shippable tree (PROP-024 §2.2): identity is the source,
//! never build artifacts. The content-hash algorithm exists in two recipes
//! (PROP-044 §4.7) and is duplicated verbatim-in-intent in `vibe-index`'s
//! `content_hash` port — the two MUST stay in lockstep (PROP-005 §3.2), and a
//! parity test (`vibe-index`'s `tests/content_hash_parity.rs`) gates that.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-024#shippable-tree");

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::RegistryError;
use crate::hash_recipe::{LEGACY0_EXCLUDES, RecipeId, order_entries};

/// Prune build output from a [`WalkDir`] walk so the hash and slot cover only
/// the shippable tree — per-entry, so an excluded dir is skipped, not entered.
/// The list is recipe 0's frozen [`LEGACY0_EXCLUDES`]; recipe 1's data-driven
/// list is identical today and carries the same lockstep obligation.
fn is_shippable(entry: &walkdir::DirEntry, excludes: &[&str]) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|name| !excludes.contains(&name))
        .unwrap_or(true)
}

pub(crate) fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), RegistryError> {
    fs::create_dir_all(dst).map_err(|source| RegistryError::Io {
        path: dst.to_path_buf(),
        source,
    })?;
    for entry in WalkDir::new(src)
        .into_iter()
        .filter_entry(|e| is_shippable(e, LEGACY0_EXCLUDES))
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

/// Compute the content hash of `pkg_dir`'s shippable tree under `recipe`.
///
/// This is the **identity** half of the `(group, name, version,
/// content_hash)` tuple (PROP-002 §2.1). The bytes fed to SHA-256 per file
/// are `normalised_path || 0x00 || file_bytes || 0x00`, identical across
/// recipes; the two recipes differ only in file ORDER (recipe 1 normalises
/// separators before ordering, recipe 0 orders the platform path first) and
/// in the wire label. [`compute_content_hash`] (recipe 0, this crate's
/// default) delegates here.
///
/// Reads every file under `pkg_dir`:
///
/// ```no_run
/// use std::path::Path;
/// use vibe_registry::compute_content_hash;
///
/// let hash = compute_content_hash(Path::new("path/to/package")).unwrap();
/// assert!(hash.starts_with("sha256:"));
/// ```
pub fn compute_content_hash_with(
    recipe: RecipeId,
    pkg_dir: &Path,
) -> Result<String, RegistryError> {
    let excludes: Vec<&str> = match recipe {
        RecipeId::Legacy0 => LEGACY0_EXCLUDES.to_vec(),
        RecipeId::Tree1 => crate::hash_recipe::recipe1_excludes(),
    };

    // Walk the shippable tree, pairing each file with its raw relative path
    // string. Recipe 0 is lossy on non-UTF-8 (frozen behaviour); recipe 1
    // makes non-UTF-8 a hard error — two distinct invalid names would
    // otherwise collide to one hash (PROP-044 §4.7).
    let mut entries: Vec<(String, PathBuf)> = Vec::new();
    for entry in WalkDir::new(pkg_dir)
        .into_iter()
        .filter_entry(|e| is_shippable(e, &excludes))
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path().to_path_buf();
        let rel = path.strip_prefix(pkg_dir).unwrap_or(&path);
        let raw = match recipe {
            RecipeId::Legacy0 => rel.to_string_lossy().into_owned(),
            RecipeId::Tree1 => rel
                .to_str()
                .ok_or_else(|| RegistryError::Io {
                    path: path.clone(),
                    source: std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "relative path is not valid UTF-8; recipe 1 (sha256-tree/1) requires \
                         UTF-8 paths so two distinct non-UTF-8 names cannot collide to one hash",
                    ),
                })?
                .to_owned(),
        };
        entries.push((raw, path));
    }

    // The file rides through the ordering beside its path, so nothing has to
    // be looked back up afterwards — see `order_entries`.
    let ordered = order_entries(recipe, entries);

    let mut hasher = Sha256::new();
    for (norm, path) in &ordered {
        hasher.update(norm.as_bytes());
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
    Ok(format!("{}{hex}", recipe.label()))
}

/// Compute the content hash of `pkg_dir`'s shippable tree with this crate's
/// default recipe — recipe 0 (`sha256:`), the form every lockfile in
/// existence already carries. Thin wrapper over [`compute_content_hash_with`].
pub fn compute_content_hash(pkg_dir: &Path) -> Result<String, RegistryError> {
    compute_content_hash_with(RecipeId::Legacy0, pkg_dir)
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
