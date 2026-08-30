//! The canonical directory digest — §6.2's "records a canonical directory
//! digest", specified here rather than left to a walk order.
//!
//! **The algorithm, `sha256-tree/1`, in full.**
//!
//! 1. Walk the tree. Every entry is proved NOT to be a link (symlink,
//!    junction or reparse point) before it is looked at, so nothing
//!    outside the tree can be reached, and every entry is either a
//!    directory or a regular file — anything else refuses.
//! 2. For each regular file, take its forward-slashed path relative to
//!    the tree root and the SHA-256 of its exact bytes.
//! 3. SORT those pairs by relative path, as bytes. This is the step that
//!    makes the value independent of the walk: a filesystem is free to
//!    hand entries back in any order, and two machines that disagree about
//!    it must still agree about the digest.
//! 4. Hash one manifest over the sorted pairs — the literal algorithm
//!    name, then `path\0digest\0` for each pair. NUL is the separator
//!    because a path cannot contain one, so no two different trees can
//!    render to the same manifest bytes.
//! 5. The digest of that manifest is the directory's digest.
//!
//! **What it deliberately does not cover.** An empty directory contributes
//! nothing, because it carries no content; two trees that differ only by
//! an empty directory have one digest. That is the same choice every
//! content-addressed tree format makes, and it is stated rather than
//! discovered.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#ARTIFACT-REGISTRY");

use std::path::Path;

use sha2::{Digest, Sha256};

use crate::mechanism::contain::{FileFault, digest_file, forward_slashed, prove_directory};

/// The algorithm identity, hashed in so a future revision cannot collide
/// with this one.
const ALGORITHM: &str = "sha256-tree/1";

/// One tree's canonical witness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TreeDigest {
    /// 64 lowercase hex over the canonical manifest.
    pub(crate) digest: String,
    /// How many regular files the digest covers — the census that makes a
    /// silently skipped file visible.
    pub(crate) files: usize,
    /// The total bytes those files hold.
    pub(crate) bytes: u64,
}

/// One entry the walk refused, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TreeFault {
    /// The tree-relative path, forward-slashed.
    pub(crate) path: String,
    pub(crate) reason: String,
}

/// Digest one directory tree canonically.
pub(crate) fn tree_digest(root: &Path) -> Result<TreeDigest, TreeFault> {
    prove_directory(root).map_err(|fault| TreeFault {
        path: ".".to_owned(),
        reason: fault.reason(),
    })?;
    let mut entries: Vec<(String, String)> = Vec::new();
    let mut bytes: u64 = 0;
    collect(root, root, &mut entries, &mut bytes)?;
    entries.sort_unstable();
    let mut hash = Sha256::new();
    hash.update(ALGORITHM.as_bytes());
    hash.update(b"\x00");
    for (path, digest) in &entries {
        hash.update(path.as_bytes());
        hash.update(b"\x00");
        hash.update(digest.as_bytes());
        hash.update(b"\x00");
    }
    Ok(TreeDigest {
        digest: format!("{:x}", hash.finalize()),
        files: entries.len(),
        bytes,
    })
}

/// The recursive half: one directory's own entries, then its children.
fn collect(
    root: &Path,
    directory: &Path,
    entries: &mut Vec<(String, String)>,
    bytes: &mut u64,
) -> Result<(), TreeFault> {
    let relative_of = |path: &Path| {
        crate::mechanism::contain::relative_to(path, root).unwrap_or_else(|| forward_slashed(path))
    };
    let listing = std::fs::read_dir(directory).map_err(|error| TreeFault {
        path: relative_of(directory),
        reason: error.to_string(),
    })?;
    // Read the whole listing first, then walk it in name order: a
    // recursion driven by the filesystem's own order would make the walk —
    // though not the digest — vary between machines, and a refusal should
    // name the same entry twice in a row.
    let mut children: Vec<std::path::PathBuf> = Vec::new();
    for entry in listing {
        let entry = entry.map_err(|error| TreeFault {
            path: relative_of(directory),
            reason: error.to_string(),
        })?;
        children.push(entry.path());
    }
    children.sort();
    for child in children {
        let relative = relative_of(&child);
        let metadata = std::fs::symlink_metadata(&child).map_err(|error| TreeFault {
            path: relative.clone(),
            reason: error.to_string(),
        })?;
        if metadata.file_type().is_symlink() {
            return Err(TreeFault {
                path: relative,
                reason: FileFault::Link.reason(),
            });
        }
        if metadata.is_dir() {
            collect(root, &child, entries, bytes)?;
            continue;
        }
        if !metadata.is_file() {
            return Err(TreeFault {
                path: relative,
                reason: FileFault::NotRegular.reason(),
            });
        }
        let (digest, length) = digest_file(&child).map_err(|fault| TreeFault {
            path: relative.clone(),
            reason: fault.reason(),
        })?;
        *bytes += length;
        entries.push((relative, digest));
    }
    Ok(())
}
