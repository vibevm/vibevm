//! `compute_content_hash` — the index side of the content-hash pair.
//!
//! The algorithm exists in two recipes (PROP-044 §4.7 `##M-BREAK-WINDOW`):
//! recipe 0 (the pre-2026-08 behaviour, frozen verbatim) and recipe 1 (the
//! live tree recipe, parameters carried as data in
//! `formats/hash_recipes/1.toml`). This crate computes with recipe 1 by
//! default; `vibe-registry` computes with recipe 0. Both implementations
//! support BOTH recipes, so the two stay byte-identical for any given
//! recipe — PROP-005 §3.2's duplicate-rather-than-import port, where a
//! parity test (`tests/content_hash_parity.rs`) gates divergence at CI time.
//!
//! Algorithm (D-C): walk → filter shippable → per file derive a relative
//! path string (recipe 0: `to_string_lossy`; recipe 1: `to_str`, a hard
//! error on non-UTF-8) → order by recipe via [`order_entries`] (which
//! normalises `\` → `/`) → hash `(norm_path || 0x00 || file_bytes || 0x00)`
//! → label with the recipe's wire prefix.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#deps");

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::error::{Error, Result};
use crate::hash_recipe::{LEGACY0_EXCLUDES, RecipeId, order_entries};

/// Prune build output from the walk so the hash covers only the shippable
/// tree. Per-entry, so an excluded directory is skipped without descending.
/// The exclude set is the recipe's: recipe 0's frozen [`LEGACY0_EXCLUDES`],
/// recipe 1's data-driven list from the recipe file — the two are identical
/// today, and MUST stay in lockstep (PROP-005 §3.2), or a package indexed
/// here and materialised there would hash differently.
fn is_shippable(entry: &walkdir::DirEntry, excludes: &[&str]) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|name| !excludes.contains(&name))
        .unwrap_or(true)
}

/// Compute the content hash of `pkg_dir`'s shippable tree under `recipe`.
///
/// `compute_content_hash` (recipe 1, this crate's default) delegates here.
/// The bytes fed to SHA-256 for each file are
/// `normalised_path || 0x00 || file_bytes || 0x00`, identical across recipes;
/// the two recipes differ only in the ORDER of those files (recipe 1
/// normalises separators before ordering, recipe 0 orders the platform path
/// first) and in the wire label stamped on the final hex.
pub fn compute_content_hash_with(recipe: RecipeId, pkg_dir: &Path) -> Result<String> {
    let excludes: Vec<&str> = match recipe {
        RecipeId::Legacy0 => LEGACY0_EXCLUDES.to_vec(),
        RecipeId::Tree1 => crate::hash_recipe::recipe1_excludes(),
    };

    // Walk the shippable tree, pairing each file with its raw relative path
    // string. Recipe 0 is lossy on non-UTF-8 (frozen behaviour); recipe 1
    // makes non-UTF-8 a hard error — two distinct invalid names would
    // otherwise collide to one hash, so an unverifiable answer would look
    // valid (PROP-044 §4.7: every derived value says how it was made).
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
                .ok_or_else(|| Error::Io {
                    path: path.clone(),
                    message: "relative path is not valid UTF-8; recipe 1 (sha256-tree/1) requires \
                     UTF-8 paths so two distinct non-UTF-8 names cannot collide to one hash"
                        .to_string(),
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
    Ok(format!("{}{hex}", recipe.label()))
}

/// Compute the content hash of `pkg_dir`'s shippable tree with this crate's
/// default recipe — recipe 1 (`sha256-tree/1:`). Thin wrapper over
/// [`compute_content_hash_with`].
pub fn compute_content_hash(pkg_dir: &Path) -> Result<String> {
    compute_content_hash_with(RecipeId::Tree1, pkg_dir)
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
        // SHA-256 of zero bytes — empty stream produces empty SHA-256. Recipe 1
        // (the index default) labels it `sha256-tree/1:`.
        assert_eq!(
            h,
            "sha256-tree/1:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
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
        assert!(h.starts_with("sha256-tree/1:"));
        assert_eq!(h.len(), "sha256-tree/1:".len() + 64); // label + hex(32 bytes)
    }

    #[test]
    fn build_output_is_excluded_from_the_hash() {
        // A package with build output hashes identically to the same package
        // without it — identity is the shippable source (PROP-024 §2.2).
        let clean = tempdir().unwrap();
        fs::write(clean.path().join("vibe.toml"), b"name = 'x'\n").unwrap();

        let dirty = tempdir().unwrap();
        fs::write(dirty.path().join("vibe.toml"), b"name = 'x'\n").unwrap();
        fs::create_dir_all(dirty.path().join("target/debug")).unwrap();
        fs::write(dirty.path().join("target/debug/x.bin"), b"ARTIFACT").unwrap();
        fs::create_dir_all(dirty.path().join(".git")).unwrap();
        fs::write(dirty.path().join(".git/HEAD"), b"ref: refs/heads/main\n").unwrap();

        assert_eq!(
            compute_content_hash(clean.path()).unwrap(),
            compute_content_hash(dirty.path()).unwrap(),
            "build output must not affect the content hash"
        );
    }

    #[test]
    fn recipe_label_selects_the_wire_form() {
        // The two recipes share the empty-tree hex but differ in their label —
        // the label is the value's recipe, readable off the value itself.
        let dir = tempdir().unwrap();
        let legacy = compute_content_hash_with(RecipeId::Legacy0, dir.path()).unwrap();
        let tree = compute_content_hash_with(RecipeId::Tree1, dir.path()).unwrap();
        assert!(legacy.starts_with("sha256:"));
        assert!(tree.starts_with("sha256-tree/1:"));
        assert_eq!(
            legacy["sha256:".len()..],
            tree["sha256-tree/1:".len()..],
            "the two recipes share the empty-tree digest; only the label differs"
        );
    }
}
