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
use std::time::SystemTime;

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

mod refresh;
pub(crate) use refresh::insert_current_at;

// ---------------------------------------------------------------------------
// The integrity sidecar (PROP-010 §2.7/§2.8, boss ruling 2026-08-20):
// `vibe cache check` reports each entry "that no longer matches what
// was RECORDED" — and the record is a one-line sidecar file
// `<name>/v<version>.sha256`, a SIBLING of the version directory
// (never inside it: a file inside the entry would change the very
// tree the hash covers). This is not a second index representation
// (the layout stays the index) — it is the integrity record the spec
// demands. Written by [`insert_at`] write-once; dies with its entry;
// an entry without one is the "unrecorded" class, never an error.
// ---------------------------------------------------------------------------

/// The sidecar path for one entry — `<root>/<group>/<name>/v<version>.sha256`.
pub(crate) fn sidecar_path(
    root: &Path,
    group: &Group,
    name: &str,
    version: &semver::Version,
) -> PathBuf {
    root.join(group.as_str())
        .join(name)
        .join(format!("v{version}.sha256"))
}

/// The hash recorded for one entry, or `None` when it has no sidecar
/// (inserted before hash recording existed, or the sidecar write was
/// interrupted) — the "unrecorded" class of `vibe cache check`, not an
/// error. The line is returned as recorded (trimmed); a malformed
/// record simply never matches a computed hash and is named as a
/// mismatch by the sweep.
pub fn recorded_hash(
    group: &Group,
    name: &str,
    version: &semver::Version,
) -> Result<Option<String>, RegistryError> {
    recorded_hash_at(&store_root()?, group, name, version)
}

/// Record the integrity sidecar for one entry as `hash` (a
/// `sha256:<hex>` line) — **write-once**: an existing sidecar is never
/// rewritten (`Ok(false)`). Written through a temp sibling + rename,
/// the same atomicity discipline as the entry itself. Called by
/// [`insert_at`] for every fresh insert and by `vibe cache check
/// --repair` for unrecorded entries.
pub fn record_hash(
    group: &Group,
    name: &str,
    version: &semver::Version,
    hash: &str,
) -> Result<bool, RegistryError> {
    record_hash_at(&store_root()?, group, name, version, hash)
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

// ---------------------------------------------------------------------------
// Reclaim — the explicit operator half of the store's life (PROP-010
// §2.1 EXPLICIT-RECLAIM, §2.8 `vibe cache clean`). Reclaim never
// rewrites an entry: write-once binds our *writes*, and removal is a
// different operation the operator performs on purpose — the store is
// never auto-evicted (§2.1). Everything below is therefore shaped as
// whole-directory removal plus empty-parent pruning, so a fully
// removed name leaves no `<group>/<name>/` husk behind: a residue that
// still named the deleted package would defeat the deletion.
// ---------------------------------------------------------------------------

/// Remove `dir` when it is empty — best-effort, never fatal: after the
/// last version of a name is reclaimed the `<group>/<name>/` directory
/// (and after the last name, the `<group>/` one) must not linger as a
/// tombstone. A failure here is cosmetic, not a lost deletion.
fn prune_empty_dir(dir: &Path) {
    let is_empty = fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_none());
    if is_empty {
        let _ = fs::remove_dir(dir);
    }
}

/// Reclaim one entry — `(group, name, version)` — from the machine
/// store ([`remove_entry_at`] core). Returns `Ok(true)` iff an entry
/// existed and was removed; `Ok(false)` when the identity was not in
/// the store (nothing to reclaim is not an error — the caller decides
/// whether an absent target is). An emptied `<group>/<name>/` chain is
/// pruned so deletion leaves no residue.
pub fn remove_entry(
    group: &Group,
    name: &str,
    version: &semver::Version,
) -> Result<bool, RegistryError> {
    remove_entry_at(&store_root()?, group, name, version)
}

/// Reclaim every version of one `(group, name)` from the machine store
/// ([`remove_name_at`] core). Returns how many version entries were
/// removed — `0` when the name was not in the store.
pub fn remove_name(group: &Group, name: &str) -> Result<usize, RegistryError> {
    remove_name_at(&store_root()?, group, name)
}

/// Every entry whose store directory's mtime is strictly older than
/// `cutoff` — the `--older-than` walk for `vibe cache clean`. Age is
/// the entry directory's own mtime (when the version landed), not any
/// file inside it. An unresolvable settings home holds no entries.
pub fn list_older_than(cutoff: SystemTime) -> Vec<(Group, String, semver::Version)> {
    match store_root() {
        Ok(root) => list_older_than_at(&root, cutoff),
        Err(_) => Vec::new(),
    }
}

/// Reclaim the entire store contents ([`remove_all_at`] core) — the
/// `--all` branch of `vibe cache clean`. Returns how many version
/// entries were removed. The store **root** itself survives (empty):
/// it is `$VIBE_SETTINGS`-owned territory that the next fetch simply
/// refills, and a foreign non-directory file dropped in the root is
/// not ours to touch (walk-as-index discipline — skipped, not fatal).
pub fn remove_all() -> Result<usize, RegistryError> {
    remove_all_at(&store_root()?)
}

/// The root-taking core of [`remove_entry`].
pub(crate) fn remove_entry_at(
    root: &Path,
    group: &Group,
    name: &str,
    version: &semver::Version,
) -> Result<bool, RegistryError> {
    let entry = entry_dir(root, group, name, version);
    if !entry.is_dir() {
        return Ok(false);
    }
    fs::remove_dir_all(&entry).map_err(|source| RegistryError::Io {
        path: entry.clone(),
        source,
    })?;
    // The sidecar dies with its entry — best-effort (the entry is
    // already gone; a sidecar that somehow outlives it is inert: the
    // walks only ever read directories).
    let sidecar = sidecar_path(root, group, name, version);
    if sidecar.exists() {
        let _ = fs::remove_file(&sidecar);
    }
    // The name directory (and then the group directory) must not
    // survive as an empty husk naming the deleted package.
    if let Some(name_dir) = entry.parent() {
        prune_empty_dir(name_dir);
        if let Some(group_dir) = name_dir.parent() {
            prune_empty_dir(group_dir);
        }
    }
    Ok(true)
}

/// The root-taking core of [`remove_name`].
pub(crate) fn remove_name_at(
    root: &Path,
    group: &Group,
    name: &str,
) -> Result<usize, RegistryError> {
    let name_dir = root.join(group.as_str()).join(name);
    if !name_dir.is_dir() {
        return Ok(0);
    }
    let removed = list_versions_at(root, group, name).len();
    fs::remove_dir_all(&name_dir).map_err(|source| RegistryError::Io {
        path: name_dir.clone(),
        source,
    })?;
    prune_empty_dir(&root.join(group.as_str()));
    Ok(removed)
}

/// The root-taking core of [`list_older_than`].
pub(crate) fn list_older_than_at(
    root: &Path,
    cutoff: SystemTime,
) -> Vec<(Group, String, semver::Version)> {
    list_all_at(root)
        .into_iter()
        .filter(|(group, name, version)| {
            fs::metadata(entry_dir(root, group, name, version))
                .and_then(|meta| meta.modified())
                .is_ok_and(|mtime| mtime < cutoff)
        })
        .collect()
}

/// The root-taking core of [`remove_all`].
pub(crate) fn remove_all_at(root: &Path) -> Result<usize, RegistryError> {
    let removed = list_all_at(root).len();
    if let Ok(children) = fs::read_dir(root) {
        for child in children.flatten() {
            if !child.file_type().is_ok_and(|t| t.is_dir()) {
                continue;
            }
            let path = child.path();
            fs::remove_dir_all(&path).map_err(|source| RegistryError::Io {
                path: path.clone(),
                source,
            })?;
        }
    }
    Ok(removed)
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
        Ok(()) => {
            // The integrity record lands beside the entry the moment
            // the entry itself is authoritative (boss ruling: rename
            // entry → record sidecar via tmp+rename). Best-effort: a
            // hash or write that fails leaves the entry whole but
            // unrecorded — `vibe cache check` names that class, and
            // its `--repair` records it; failing an insert that
            // landed would only strand an AlreadyPresent entry with
            // no record forever.
            if let Ok(hash) = crate::shippable::compute_content_hash(&entry) {
                let _ = record_hash_at(root, group, name, version, &hash);
            }
            Ok(InsertOutcome::Inserted(entry))
        }
        Err(source) => {
            let _ = fs::remove_dir_all(&tmp);
            if entry.is_dir() {
                // Another writer landed the same entry between our
                // check and the rename — write-once means theirs
                // stands, ours never existed. Their insert recorded
                // their sidecar; ours touches nothing.
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

/// The root-taking core of [`recorded_hash`].
pub(crate) fn recorded_hash_at(
    root: &Path,
    group: &Group,
    name: &str,
    version: &semver::Version,
) -> Result<Option<String>, RegistryError> {
    let sidecar = sidecar_path(root, group, name, version);
    match fs::read_to_string(&sidecar) {
        Ok(body) => {
            let line = body.trim().to_string();
            Ok((!line.is_empty()).then_some(line))
        }
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(RegistryError::Io {
            path: sidecar,
            source,
        }),
    }
}

/// The root-taking core of [`record_hash`].
pub(crate) fn record_hash_at(
    root: &Path,
    group: &Group,
    name: &str,
    version: &semver::Version,
    hash: &str,
) -> Result<bool, RegistryError> {
    let sidecar = sidecar_path(root, group, name, version);
    if sidecar.exists() {
        // Write-once: an existing record for a living entry is never
        // rewritten — it is the authority the sweep compares against.
        return Ok(false);
    }
    let Some(parent) = sidecar.parent() else {
        return Err(RegistryError::Io {
            path: sidecar.clone(),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "sidecar path has no parent directory",
            ),
        });
    };
    fs::create_dir_all(parent).map_err(|source| RegistryError::Io {
        path: parent.to_path_buf(),
        source,
    })?;
    let tmp = parent.join(format!("v{version}.sha256.tmp-{}", std::process::id()));
    fs::write(&tmp, format!("{hash}\n")).map_err(|source| RegistryError::Io {
        path: tmp.clone(),
        source,
    })?;
    match fs::rename(&tmp, &sidecar) {
        Ok(()) => Ok(true),
        Err(source) => {
            let _ = fs::remove_file(&tmp);
            if sidecar.exists() {
                Ok(false)
            } else {
                Err(RegistryError::Io {
                    path: sidecar,
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

#[cfg(test)]
#[path = "store/tests.rs"]
mod tests;
