//! The file resolver — `spec://` authority + doc-path → a file on disk
//! (PROP-035 §6, the `doc_path → file` half).
//!
//! It resolves against the **materialised** tree, as the specmap engine does:
//! the host project's authored `spec/`, and each package's `vibedeps/` slot —
//! never `packages/` (the authoring source). The lossy `PROP-NNN` / `FEAT-NNN`
//! truncation is inverted by a directory prefix-scan (the id number is unique
//! within a directory, an invariant), so `…/PROP-042` finds
//! `PROP-042-example-thing.md`.
//!
//! Version / slot selection is deliberately thin here: an explicit `@version`
//! picks the slot version, and absent one a single installed version is taken.
//! A lockfile-backed selection (kind + version from `vibe.lock`) is the layer
//! above; this resolver only needs the workspace root and the host namespace.

use std::fs;
use std::path::{Path, PathBuf};

use crate::address::{Authority, SpecAddress};

/// Resolves `spec://` addresses to files under a workspace root.
#[derive(Debug, Clone)]
pub struct FileResolver {
    ws_root: PathBuf,
    host_namespace: String,
}

/// Why an address does not resolve to a file.
#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("address targets host `{addr_host}`, but this resolver's host is `{our_host}`")]
    UnknownHost { addr_host: String, our_host: String },
    #[error("no installed vibedeps slot for package `{0}`")]
    PackageSlotNotFound(String),
    #[error("package `{0}` has several installed versions; address must pin `@version`")]
    AmbiguousVersion(String),
    #[error("document `{doc_path}` not found under `{base}`")]
    DocNotFound { doc_path: String, base: String },
    #[error("document id `{id}` is ambiguous ({count} files match) under `{dir}`")]
    AmbiguousDoc {
        id: String,
        count: usize,
        dir: String,
    },
}

impl FileResolver {
    /// A resolver rooted at `ws_root`, treating `host_namespace` (e.g.
    /// `vibevm`) as the authored host project's authority.
    pub fn new(ws_root: impl Into<PathBuf>, host_namespace: impl Into<String>) -> Self {
        Self {
            ws_root: ws_root.into(),
            host_namespace: host_namespace.into(),
        }
    }

    /// Resolve an address to the file that holds its document. Ignores the
    /// anchor / revision — those address a node *within* the returned file
    /// (see [`DocTree`](crate::DocTree)).
    pub fn resolve_file(&self, addr: &SpecAddress) -> Result<PathBuf, ResolveError> {
        let base_spec = self.spec_root(&addr.authority)?;
        resolve_doc(&base_spec, &addr.doc_path)
    }

    /// The `spec/` root an authority resolves against.
    fn spec_root(&self, authority: &Authority) -> Result<PathBuf, ResolveError> {
        match authority {
            Authority::Host(h) if *h == self.host_namespace => Ok(self.ws_root.join("spec")),
            Authority::Host(h) => Err(ResolveError::UnknownHost {
                addr_host: h.clone(),
                our_host: self.host_namespace.clone(),
            }),
            Authority::Package { name, version, .. } => {
                Ok(self.package_slot(name, version.as_deref())?.join("spec"))
            }
        }
    }

    /// Find a package's materialised slot: `vibedeps/<kind>-<name>/<version>`.
    /// The address carries no `kind`, so the slot is matched by the `-<name>`
    /// suffix (kind + name is unique).
    fn package_slot(&self, name: &str, version: Option<&str>) -> Result<PathBuf, ResolveError> {
        let vibedeps = self.ws_root.join("vibedeps");
        let suffix = format!("-{name}");
        let slot_dir = read_dir_or_empty(&vibedeps)
            .map(|e| e.path())
            .find(|p| {
                p.is_dir()
                    && p.file_name()
                        .and_then(|s| s.to_str())
                        .is_some_and(|n| n.ends_with(&suffix))
            })
            .ok_or_else(|| ResolveError::PackageSlotNotFound(name.to_string()))?;

        match version {
            Some(v) => Ok(slot_dir.join(v)),
            None => {
                let mut versions: Vec<PathBuf> = read_dir_or_empty(&slot_dir)
                    .map(|e| e.path())
                    .filter(|p| p.is_dir())
                    .collect();
                match versions.len() {
                    0 => Err(ResolveError::PackageSlotNotFound(name.to_string())),
                    1 => Ok(versions.pop().unwrap()),
                    _ => Err(ResolveError::AmbiguousVersion(name.to_string())),
                }
            }
        }
    }
}

/// Resolve a doc-path (relative to a `spec/` root) to a `.md` file, inverting
/// the `PROP-NNN` / `FEAT-NNN` truncation by a prefix-scan.
fn resolve_doc(base_spec: &Path, doc_path: &str) -> Result<PathBuf, ResolveError> {
    let (dir, last) = match doc_path.rsplit_once('/') {
        Some((d, l)) => (base_spec.join(d), l),
        None => (base_spec.to_path_buf(), doc_path),
    };

    if is_id_stem(last) {
        let mut matches: Vec<PathBuf> = read_dir_or_empty(&dir)
            .map(|e| e.path())
            .filter(|p| id_file_matches(p, last))
            .collect();
        match matches.len() {
            0 => Err(ResolveError::DocNotFound {
                doc_path: doc_path.to_string(),
                base: base_spec.display().to_string(),
            }),
            1 => Ok(matches.pop().unwrap()),
            n => Err(ResolveError::AmbiguousDoc {
                id: last.to_string(),
                count: n,
                dir: dir.display().to_string(),
            }),
        }
    } else {
        let candidate = base_spec.join(format!("{doc_path}.md"));
        if candidate.is_file() {
            Ok(candidate)
        } else {
            Err(ResolveError::DocNotFound {
                doc_path: doc_path.to_string(),
                base: base_spec.display().to_string(),
            })
        }
    }
}

/// Does a file stem equal `id` or start with `id-` (the descriptive-slug form)?
fn id_file_matches(path: &Path, id: &str) -> bool {
    let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
        return false;
    };
    let Some(stem) = name.strip_suffix(".md") else {
        return false;
    };
    stem == id
        || stem
            .strip_prefix(id)
            .is_some_and(|rest| rest.starts_with('-'))
}

/// A `PROP-NNN` / `FEAT-NNN` id stem (the truncated doc-path tail).
fn is_id_stem(s: &str) -> bool {
    let Some((kind, num)) = s.split_once('-') else {
        return false;
    };
    (kind == "PROP" || kind == "FEAT") && !num.is_empty() && num.bytes().all(|b| b.is_ascii_digit())
}

/// Iterate a directory's entries, yielding nothing if it is unreadable or
/// absent (the resolver degrades to "not found", never panics).
fn read_dir_or_empty(dir: &Path) -> impl Iterator<Item = fs::DirEntry> {
    fs::read_dir(dir).into_iter().flatten().flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_stem_recognition() {
        assert!(is_id_stem("PROP-042"));
        assert!(is_id_stem("FEAT-7"));
        assert!(!is_id_stem("PROP"));
        assert!(!is_id_stem("PROP-"));
        assert!(!is_id_stem("README"));
        assert!(!is_id_stem("PROP-00x"));
        assert!(!is_id_stem("DESIGN-1")); // only PROP / FEAT truncate
    }

    #[test]
    fn id_file_match() {
        assert!(id_file_matches(
            Path::new("PROP-042-example-thing.md"),
            "PROP-042"
        ));
        assert!(id_file_matches(Path::new("PROP-042.md"), "PROP-042"));
        // A different number sharing a prefix does not match.
        assert!(!id_file_matches(Path::new("PROP-0420-x.md"), "PROP-042"));
        assert!(!id_file_matches(
            Path::new("PROP-042-example.txt"),
            "PROP-042"
        ));
    }
}
