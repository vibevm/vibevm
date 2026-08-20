//! The machine-global package store — `~/.vibe/cache/` (PROP-010
//! §2.7, owner ruling `THE-STORE-IS-DOT-VIBE-CACHE`): one accretive,
//! identity-keyed store per machine, beside `~/.vibe/registries/`
//! (the registry git clones, which keep their own separate job —
//! `CLONES-KEEP-THEIR-OWN-JOB`) and under the one settings home.
//!
//! Layout — per-identity extracted directories
//! (`LAYOUT-EXTRACTED-DIRECTORIES`):
//!
//! ```text
//! <store>/<group>/<name>/v<version>/
//! ```
//!
//! holding the `.git`-stripped shippable tree. **The layout is the
//! index; a second representation would drift** — the "local index
//! view" of PROP-010 §2.7 is exactly this directory tree walked, so
//! resolver and management queries answer from the same bytes the
//! materialiser copies, and no side-car state can fall out of
//! agreement with the store it describes.
//!
//! **Written once** (`WRITTEN-ONCE-IS-A-RULE-FOR-OUR-CODE-NOT-A-CLAIM-
//! ABOUT-THE-DISK`): our code never rewrites an entry in place — a
//! version is written when first fetched and is read-only to us
//! afterwards; verification is a command an operator runs, not a tax
//! every install pays; and a mismatch against a lockfile pin is
//! named, never swallowed. The disk itself is the operator's —
//! nothing here defends against, or apologises for, an edit made
//! outside vibevm.
//!
//! The root-less wrappers ([`insert_from`], [`lookup`],
//! [`list_versions`], [`list_all`]) resolve the store through the one
//! `vibe_core::settings` chokepoint — deliberately with **no env
//! override of its own** (owner ruling: the store moves with
//! `$VIBE_SETTINGS`, full stop). The `*_at` cores take the root
//! explicitly so the fetch chain threads one resolved root through a
//! whole plan and in-crate tests isolate by parameter rather than by
//! environment.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-010#layout");

use std::fs;
use std::path::{Path, PathBuf};

use vibe_core::Group;

use crate::copy_dir_recursive;
use crate::error::RegistryError;

/// What [`insert_from`] / the `*_at` insert core found on disk.
///
/// Both variants carry the entry's path: the caller's next step —
/// materialisation — always wants "where are the bytes", regardless
/// of whether this call or an earlier one put them there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsertOutcome {
    /// The entry was absent and this call created it atomically
    /// (copied into a sibling temp dir, then renamed into place).
    Inserted(PathBuf),
    /// The entry already existed; **nothing was rewritten** — the
    /// write-once rule keeps the first fetch's bytes authoritative.
    AlreadyPresent(PathBuf),
}

impl InsertOutcome {
    /// The entry path, whichever way this call landed.
    pub fn into_entry(self) -> PathBuf {
        match self {
            InsertOutcome::Inserted(p) | InsertOutcome::AlreadyPresent(p) => p,
        }
    }
}

/// The machine store root: `<settings-home>/cache`
/// (`THE-STORE-IS-DOT-VIBE-CACHE`). Resolved through the one
/// settings chokepoint so `$VIBE_SETTINGS` relocates the store with
/// the rest of the per-user tree; no separate override exists, by
/// owner ruling.
pub fn store_root() -> Result<PathBuf, RegistryError> {
    vibe_core::settings::settings_dir()
        .map(|home| home.join("cache"))
        .ok_or(RegistryError::NoHomeDir)
}

/// The entry path for one package identity under `root` —
/// `<root>/<group>/<name>/v<version>/`. Pure layout: no I/O, no
/// creation. The same layout every reader (fetch, materialisation,
/// the future `vibe cache` commands) shares.
pub fn entry_dir(root: &Path, group: &Group, name: &str, version: &semver::Version) -> PathBuf {
    root.join(group.as_str())
        .join(name)
        .join(format!("v{version}"))
}

/// Insert `src`'s shippable tree as the entry for
/// `(group, name, version)` in the machine store — **write-once**.
///
/// Returns [`InsertOutcome::AlreadyPresent`] untouched when the entry
/// exists (our code never rewrites an entry); otherwise copies `src`
/// — `.git` and build output stripped, exactly the tree the content
/// hash is computed over — into a temporary sibling
/// (`v<version>.tmp-<pid>`) and renames it into place atomically: an
/// interrupted or failed insert leaves no entry behind, only a temp
/// directory the next attempt reclaims.
pub fn insert_from(
    src: &Path,
    group: &Group,
    name: &str,
    version: &semver::Version,
) -> Result<InsertOutcome, RegistryError> {
    insert_at(&store_root()?, src, group, name, version)
}

/// Is there an entry for this identity in the machine store? `None`
/// also when no settings home is resolvable — an unresolvable home
/// holds no entries.
pub fn lookup(group: &Group, name: &str, version: &semver::Version) -> Option<PathBuf> {
    let root = store_root().ok()?;
    lookup_at(&root, group, name, version)
}

/// Every version present for `(group, name)` in the machine store,
/// ascending. The "local index view" (PROP-010 §2.7) — a directory
/// walk, not a second store: an unparseable `v…` directory is skipped
/// rather than fatal, so a foreign file dropped in the store cannot
/// break listing.
pub fn list_versions(group: &Group, name: &str) -> Vec<semver::Version> {
    match store_root() {
        Ok(root) => list_versions_at(&root, group, name),
        Err(_) => Vec::new(),
    }
}

/// Every `(group, name, version)` present in the machine store — the
/// offline-resolvable inventory. Same walk-as-index discipline as
/// [`list_versions`]: unparseable segments are skipped, never fatal.
pub fn list_all() -> Vec<(Group, String, semver::Version)> {
    match store_root() {
        Ok(root) => list_all_at(&root),
        Err(_) => Vec::new(),
    }
}

/// The root-taking core of [`insert_from`]: insert into `root` laid
/// out per [`entry_dir`]. `pub(crate)` so the fetch chain threads one
/// resolved store root through a whole plan and the in-crate tests
/// isolate the store by parameter.
pub(crate) fn insert_at(
    root: &Path,
    src: &Path,
    group: &Group,
    name: &str,
    version: &semver::Version,
) -> Result<InsertOutcome, RegistryError> {
    let entry = entry_dir(root, group, name, version);
    if entry.is_dir() {
        return Ok(InsertOutcome::AlreadyPresent(entry));
    }
    if !src.is_dir() {
        return Err(RegistryError::Io {
            path: src.to_path_buf(),
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "store insert source does not exist or is not a directory",
            ),
        });
    }
    let parent = entry
        .parent()
        .ok_or_else(|| RegistryError::Io {
            path: entry.clone(),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "store entry path has no parent directory",
            ),
        })?
        .to_path_buf();
    fs::create_dir_all(&parent).map_err(|source| RegistryError::Io {
        path: parent.clone(),
        source,
    })?;
    // Copy beside the final name, then rename: same volume, so the
    // rename is atomic and an interrupted insert can only ever leave
    // the temp dir — never a half-written entry.
    let tmp = parent.join(format!("v{version}.tmp-{}", std::process::id()));
    // A temp left by an interrupted earlier attempt is not an entry
    // and carries no write-once protection — reclaim it.
    if tmp.exists() {
        let _ = fs::remove_dir_all(&tmp);
    }
    let copied = copy_dir_recursive(src, &tmp);
    if let Err(e) = copied {
        let _ = fs::remove_dir_all(&tmp);
        return Err(e);
    }
    match fs::rename(&tmp, &entry) {
        Ok(()) => Ok(InsertOutcome::Inserted(entry)),
        Err(source) => {
            let _ = fs::remove_dir_all(&tmp);
            if entry.is_dir() {
                // Another writer landed the same entry between our
                // check and the rename — write-once means theirs
                // stands, ours never existed.
                Ok(InsertOutcome::AlreadyPresent(entry))
            } else {
                Err(RegistryError::Io {
                    path: entry,
                    source,
                })
            }
        }
    }
}

/// The root-taking core of [`lookup`].
pub(crate) fn lookup_at(
    root: &Path,
    group: &Group,
    name: &str,
    version: &semver::Version,
) -> Option<PathBuf> {
    let entry = entry_dir(root, group, name, version);
    entry.is_dir().then_some(entry)
}

/// The root-taking core of [`list_versions`].
pub(crate) fn list_versions_at(root: &Path, group: &Group, name: &str) -> Vec<semver::Version> {
    let mut versions: Vec<semver::Version> = fs::read_dir(root.join(group.as_str()).join(name))
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| {
            let raw = e.file_name().into_string().ok()?;
            let stripped = raw.strip_prefix('v')?;
            semver::Version::parse(stripped).ok()
        })
        .collect();
    versions.sort();
    versions
}

/// The root-taking core of [`list_all`].
pub(crate) fn list_all_at(root: &Path) -> Vec<(Group, String, semver::Version)> {
    let mut out: Vec<(Group, String, semver::Version)> = Vec::new();
    let Ok(groups) = fs::read_dir(root) else {
        return out;
    };
    for g in groups.flatten() {
        let Ok(group_raw) = g.file_name().into_string() else {
            continue;
        };
        if !g.file_type().is_ok_and(|t| t.is_dir()) {
            continue;
        }
        let Ok(group) = Group::parse(&group_raw) else {
            continue;
        };
        let Ok(names) = fs::read_dir(g.path()) else {
            continue;
        };
        for n in names.flatten() {
            if !n.file_type().is_ok_and(|t| t.is_dir()) {
                continue;
            }
            let Ok(name) = n.file_name().into_string() else {
                continue;
            };
            out.extend(
                list_versions_at(root, &group, &name)
                    .into_iter()
                    .map(|version| (group.clone(), name.clone(), version)),
            );
        }
    }
    out.sort_by(|a, b| (a.0.as_str(), a.1.as_str(), &a.2).cmp(&(b.0.as_str(), b.1.as_str(), &b.2)));
    out
}
