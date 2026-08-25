//! The dependency materialisation tree — PROP-009 §2.1.
//!
//! `vibe install` writes every resolved dependency into a tree rooted at the
//! absolute workspace root, one slot per package:
//!
//! ```text
//! <workspace-root>/<live-dependency-root>/<group>.<name>/<version>/
//! ```
//!
//! The slot's payload holds the package's published tree **verbatim** beside
//! one reserved `.vibe-slot.toml` identity/footprint record. Unified resolution
//! (PROP-007 §2.4) guarantees one version per package, so a single slot serves
//! the whole workspace. The materialisation tree is committed to the repository
//! — a fresh clone is bootable with no `vibe install`, and the dependency corpus
//! stays visible and diffable.
//!
//! This module owns the **layout**, the **verbatim payload copy**, and its
//! materialisation record. It is
//! additive: it never retires the legacy `[writes]` mirror layout
//! (`VIBEVM-SPEC.md` §13.1). That retirement is the `vibe install`
//! switch-over — a later PROP-009 phase — and removing the mirror path
//! before `vibe install` is rebuilt on dependency slots would break the build.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#two-trees");

use std::fs;
use std::path::{Path, PathBuf};

use vibe_core::{ContentHash, Group, layout};

use crate::{WorkspaceError, layout_paths};

mod build_ignore;
mod derived;
mod slot_diff;
mod slot_record;

pub use build_ignore::{BUILD_OUTPUT_IGNORES, ensure_build_output_ignores};
pub(crate) use derived::materialise_with_spec_format_report;
pub use derived::{
    CONVERTER_RECIPE, DERIVED_MANIFEST_FILENAME, DerivedFile, DerivedFileDisposition,
    DerivedManifest, compute_derived_hash, format_is_current, materialise_with_spec_format,
    read_derived_manifest,
};
pub(crate) use slot_diff::MaterialiseReport;
pub use slot_record::{
    SLOT_RECORD_FILENAME, SLOT_RECORD_SCHEMA, SlotFile, SlotFileDisposition, SlotRecord,
    compute_recorded_payload_hash, read_slot_record, sha256_file, verify_recorded_files,
    write_slot_record,
};

/// Directory name of the materialisation tree's final component.
pub use vibe_core::layout::VIBEDEPS_DIR;

/// The slot path for one resolved package, relative to the workspace root
/// and forward-slashed under the live dependency root.
///
/// Root-relative and forward-slashed so it is portable across machines —
/// the same property [`WorkspaceMember::rel_path`](crate::WorkspaceMember)
/// carries.
pub fn slot_rel_path(group: &Group, name: &str, version: &semver::Version) -> String {
    layout_paths::vibedeps(format!("{group}.{name}/{version}"))
}

/// The absolute on-disk slot path — `workspace_root` joined with
/// [`slot_rel_path`]. In-memory only; never persist an absolute path.
pub fn slot_abs_path(
    workspace_root: &Path,
    group: &Group,
    name: &str,
    version: &semver::Version,
) -> PathBuf {
    let mut p = workspace_root.join(layout::current_vibedeps_root());
    p.push(format!("{group}.{name}"));
    p.push(version.to_string());
    p
}

/// `true` iff the slot for this package already exists on disk.
pub fn is_materialised(
    workspace_root: &Path,
    group: &Group,
    name: &str,
    version: &semver::Version,
) -> bool {
    slot_abs_path(workspace_root, group, name, version).is_dir()
}

/// Materialise a resolved package into its dependency slot — copy the
/// package's published content tree (`content_src`) verbatim into
/// live dependency-root path.
///
/// **Idempotent.** Re-materialisation diffs the incoming footprint against
/// `.vibe-slot.toml`: unchanged recorded files retain their inode and mtime,
/// changed files are atomically replaced, and stale recorded files are
/// removed. Unrecorded build output remains outside materialiser ownership.
/// A legacy slot with no record receives one final whole-slot replacement.
///
/// A `.git` entry in the source is skipped at every depth — a materialised
/// slot is plain content committed into the outer repository, never a
/// nested repository. Symlinks are skipped: a committed dependency tree
/// must be portable, and a published package ships plain files.
///
/// Returns the slot-relative payload paths, forward-slashed and sorted. The
/// slot record is written last from that footprint and is not returned.
pub fn materialise(
    workspace_root: &Path,
    group: &Group,
    name: &str,
    version: &semver::Version,
    content_src: &Path,
    source_hash: &ContentHash,
) -> Result<Vec<PathBuf>, WorkspaceError> {
    materialise_with(
        workspace_root,
        group,
        name,
        version,
        content_src,
        CopyMode::Copy,
        source_hash,
    )
}

/// How [`materialise_with`] places each file into a slot (PROP-022 §2.2/§2.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyMode {
    /// Full byte copy — the default `copy` materialisation (PROP-022 §2.2).
    Copy,
    /// Hardlink each file from the source, falling back to a copy when the
    /// filesystem refuses (cross-volume / unsupported) — the `hardlink`
    /// materialisation for packages big in bytes but modest in file count
    /// (PROP-022 §2.3).
    Hardlink,
}

/// Like [`materialise`] but selects how each file is placed (PROP-022
/// §2.2/§2.3). The slot still presents a full tree and the returned
/// footprint is identical — only the on-disk byte-sharing differs.
pub fn materialise_with(
    workspace_root: &Path,
    group: &Group,
    name: &str,
    version: &semver::Version,
    content_src: &Path,
    mode: CopyMode,
    source_hash: &ContentHash,
) -> Result<Vec<PathBuf>, WorkspaceError> {
    materialise_with_report(
        workspace_root,
        group,
        name,
        version,
        content_src,
        mode,
        source_hash,
    )
    .map(MaterialiseReport::into_footprint)
}

pub(crate) fn materialise_with_report(
    workspace_root: &Path,
    group: &Group,
    name: &str,
    version: &semver::Version,
    content_src: &Path,
    mode: CopyMode,
    source_hash: &ContentHash,
) -> Result<MaterialiseReport, WorkspaceError> {
    let slot = slot_abs_path(workspace_root, group, name, version);
    let slot_label = slot_rel_path(group, name, version);

    if !content_src.is_dir() {
        return Err(WorkspaceError::Io {
            path: content_src.to_path_buf(),
            reason: format!(
                "source content tree for `{slot_label}` does not exist or is not a directory"
            ),
        });
    }
    refuse_reserved_source_record(content_src)?;

    let incoming = slot_diff::prepare_source_tree(content_src, mode)?;
    let files = incoming
        .iter()
        .map(|file| SlotFile {
            path: file.path_wire(),
            sha256: file.sha256().to_string(),
            source: None,
            disposition: None,
        })
        .collect();
    let record = SlotRecord {
        schema: SLOT_RECORD_SCHEMA,
        source_hash: source_hash.clone(),
        spec_format: vibe_core::manifest::SpecFormat::Mixed,
        converter_recipe: None,
        derived_hash: None,
        overlay_hash: None,
        files,
    };
    slot_diff::reconcile_slot(&slot, &incoming, &record)
}

fn refuse_reserved_source_record(content_src: &Path) -> Result<(), WorkspaceError> {
    let reserved = content_src.join(SLOT_RECORD_FILENAME);
    match fs::symlink_metadata(&reserved) {
        Ok(_) => Err(WorkspaceError::SpecMaterialization {
            path: reserved,
            reason: format!(
                "authored root `{SLOT_RECORD_FILENAME}` is reserved for materialisation metadata"
            ),
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_err(&reserved, error)),
    }
}

/// Remove a package's dependency slot, if it exists. Returns `true` when a
/// slot was present and deleted, `false` when there was nothing to remove.
pub fn remove_slot(
    workspace_root: &Path,
    group: &Group,
    name: &str,
    version: &semver::Version,
) -> Result<bool, WorkspaceError> {
    let slot = slot_abs_path(workspace_root, group, name, version);
    if !slot.exists() {
        return Ok(false);
    }
    fs::remove_dir_all(&slot).map_err(|e| io_err(&slot, e))?;
    Ok(true)
}

// --- in-place materialization (PROP-022 §2.4) ---------------------------
//
// An `in-place` package is placed as a project-local git working tree in an
// **unversioned** slot under the live dependency root with no `/<version>/` —
// keeping its `.git` so git manages it in place. The slot is moved into
// position from a fetched clone (no per-file `copy`) and `.gitignore`d
// (not vendored, §2.7).

/// The unversioned slot path for an `in-place` package, relative to the
/// workspace root and forward-slashed under the live dependency root (PROP-022
/// §2.4 — one working clone whose version is the current git ref, so the
/// path carries no `/<version>/`).
pub fn in_place_slot_rel_path(group: &Group, name: &str) -> String {
    layout_paths::vibedeps(format!("{group}.{name}"))
}

/// The absolute on-disk path of an `in-place` slot — `workspace_root` joined
/// with [`in_place_slot_rel_path`]. In-memory only; never persisted.
pub fn in_place_slot_abs_path(workspace_root: &Path, group: &Group, name: &str) -> PathBuf {
    workspace_root
        .join(layout::current_vibedeps_root())
        .join(format!("{group}.{name}"))
}

/// `true` iff an `in-place` slot is materialised for this package — the
/// unversioned slot directory exists and is a git working tree (carries
/// `.git`). The `.git` presence is what distinguishes an in-place slot from
/// a `<group>.<name>/` directory that merely groups versioned `copy` slots,
/// so [`prune_stale_slots`](crate::install) leaves it untouched.
pub fn is_in_place_slot(workspace_root: &Path, group: &Group, name: &str) -> bool {
    in_place_slot_abs_path(workspace_root, group, name)
        .join(".git")
        .exists()
}

/// Materialise an `in-place` package by **moving** a fetched git clone
/// (`clone_src`, a working tree WITH its `.git`) into the unversioned slot
/// (PROP-022 §2.4). A move — `rename` when source and slot share a volume, a
/// recursive copy-then-remove across volumes — so a giant repo is placed
/// without the per-file `copy` materialisation the mode exists to avoid. The
/// `.git` is
/// preserved (unlike [`materialise`], which strips it) so the slot stays a
/// git working tree manageable in place.
pub fn materialise_in_place(
    workspace_root: &Path,
    group: &Group,
    name: &str,
    clone_src: &Path,
) -> Result<(), WorkspaceError> {
    let slot = in_place_slot_abs_path(workspace_root, group, name);
    if !clone_src.is_dir() {
        return Err(WorkspaceError::Io {
            path: clone_src.to_path_buf(),
            reason: format!(
                "in-place clone source for `{}` does not exist or is not a directory",
                in_place_slot_rel_path(group, name)
            ),
        });
    }
    // Replace any existing slot so the result is exactly the fetched clone.
    if slot.exists() {
        fs::remove_dir_all(&slot).map_err(|e| io_err(&slot, e))?;
    }
    if let Some(parent) = slot.parent() {
        fs::create_dir_all(parent).map_err(|e| io_err(parent, e))?;
    }
    move_dir(clone_src, &slot)
}

/// Remove an `in-place` slot if present. Returns `true` when one was deleted.
pub fn remove_in_place_slot(
    workspace_root: &Path,
    group: &Group,
    name: &str,
) -> Result<bool, WorkspaceError> {
    let slot = in_place_slot_abs_path(workspace_root, group, name);
    if !slot.exists() {
        return Ok(false);
    }
    fs::remove_dir_all(&slot).map_err(|e| io_err(&slot, e))?;
    Ok(true)
}

/// Ensure `entry` (a forward-slashed, workspace-root-relative path) is listed
/// in the workspace's top-level `.gitignore`, appending it if absent
/// (PROP-022 §2.7 — an in-place slot is not vendored). Idempotent; creates
/// `.gitignore` when missing. The entry is written with a trailing slash so
/// git treats it as a directory ignore.
pub fn ensure_gitignored(workspace_root: &Path, entry: &str) -> Result<(), WorkspaceError> {
    let path = workspace_root.join(".gitignore");
    let existing = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(io_err(&path, e)),
    };
    let want = entry.trim_end_matches('/');
    if existing
        .lines()
        .any(|l| l.trim() == want || l.trim() == format!("{want}/"))
    {
        return Ok(());
    }
    let mut out = existing;
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&format!("{entry}/\n"));
    fs::write(&path, out).map_err(|e| io_err(&path, e))
}

/// Move `src` to `dest`: a fast `rename` when they share a volume, else a
/// recursive copy (including `.git`) followed by removing `src`. The
/// same-volume `rename` is what makes an in-place placement O(1) rather than
/// a per-file copy.
fn move_dir(src: &Path, dest: &Path) -> Result<(), WorkspaceError> {
    if fs::rename(src, dest).is_ok() {
        return Ok(());
    }
    // Cross-volume (or rename otherwise refused): recursively copy every
    // entry, `.git` included, then drop the source.
    copy_all(src, dest)?;
    fs::remove_dir_all(src).map_err(|e| io_err(src, e))?;
    Ok(())
}

/// Recursively copy `src` into `dest`, **including** `.git` (unlike
/// [`copy_tree`], which strips it) — the cross-volume fallback for
/// [`move_dir`]. Symlinks are skipped (best-effort fallback path).
fn copy_all(src: &Path, dest: &Path) -> Result<(), WorkspaceError> {
    fs::create_dir_all(dest).map_err(|e| io_err(dest, e))?;
    for entry in fs::read_dir(src).map_err(|e| io_err(src, e))? {
        let entry = entry.map_err(|e| io_err(src, e))?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        let ft = entry.file_type().map_err(|e| io_err(&from, e))?;
        if ft.is_dir() {
            copy_all(&from, &to)?;
        } else if ft.is_file() {
            fs::copy(&from, &to).map_err(|e| io_err(&to, e))?;
        }
    }
    Ok(())
}

/// Build a [`WorkspaceError::Io`] from a `std::io::Error` and the path it
/// failed on.
fn io_err(path: &Path, e: std::io::Error) -> WorkspaceError {
    WorkspaceError::Io {
        path: path.to_path_buf(),
        reason: e.to_string(),
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tests_slot_record;

#[cfg(test)]
mod tests_diff;

#[cfg(test)]
mod tests_report;

#[cfg(test)]
mod tests_transformed;
