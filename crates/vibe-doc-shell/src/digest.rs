//! The shell's content hash — one algorithm, one definition, one caller
//! shape (PROP-057 `##SHELL-PIN`).
//!
//! A shell is a tree of files, so its identity is a fold over the tree
//! and not over a concatenation: two shells that differ only in where a
//! byte sits must hash differently, and a fold that ignored names could
//! not tell them apart. Each file contributes its `/`-separated relative
//! path, a separator no path can contain, its length, and its bytes; the
//! files are taken in sorted path order so the hash is a property of the
//! shell rather than of the walk that found it.
//!
//! `cargo xtask embed-doc-shell` calls this when it writes the index and
//! the pin. `vibe doc shell status` calls it on the shell the running
//! binary actually carries. They are the same call, which is what makes
//! the comparison mean anything.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#SHELL-PIN");

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

/// The digest of one shell, lowercase hex.
///
/// ```
/// use std::collections::BTreeMap;
/// use vibe_doc_shell::digest;
///
/// let mut files = BTreeMap::new();
/// files.insert("page-template.html".to_string(), b"<!doctype html>".to_vec());
/// let one = digest::of(&files);
/// assert_eq!(one.len(), 64);
///
/// // The name is part of the identity: the same bytes under another
/// // name are another shell.
/// let mut renamed = BTreeMap::new();
/// renamed.insert("other.html".to_string(), b"<!doctype html>".to_vec());
/// assert_ne!(one, digest::of(&renamed));
/// ```
pub fn of(files: &BTreeMap<String, Vec<u8>>) -> String {
    let mut hasher = Sha256::new();
    for (path, bytes) in files {
        hasher.update(path.as_bytes());
        hasher.update([0u8]);
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    hex(&hasher.finalize())
}

/// Lowercase hex of a digest.
pub(crate) fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Read a directory tree into the map [`of`] hashes, with `/`-separated
/// relative paths.
///
/// Written here rather than at each caller because the PATH SPELLING is
/// part of the hash: a Windows walk that handed back `assets\a.css` would
/// produce a different digest for the same shell, and the two sides of
/// the comparison live on different machines by design.
pub fn read_tree(root: &std::path::Path) -> std::io::Result<BTreeMap<String, Vec<u8>>> {
    let mut out = BTreeMap::new();
    collect(root, root, &mut out)?;
    Ok(out)
}

fn collect(
    root: &std::path::Path,
    dir: &std::path::Path,
    out: &mut BTreeMap<String, Vec<u8>>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect(root, &path, out)?;
            continue;
        }
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        let name = relative
            .components()
            .filter_map(|part| part.as_os_str().to_str())
            .collect::<Vec<_>>()
            .join("/");
        out.insert(name, std::fs::read(&path)?);
    }
    Ok(())
}
