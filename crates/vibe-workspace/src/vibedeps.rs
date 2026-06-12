//! The `vibedeps/` materialisation tree — PROP-009 §2.1.
//!
//! `vibe install` writes every resolved dependency into a tree rooted at the
//! absolute workspace root, one slot per package:
//!
//! ```text
//! <workspace-root>/vibedeps/<kind>-<name>/<version>/
//! ```
//!
//! The slot holds the package's published tree **verbatim**. Unified
//! resolution (PROP-007 §2.4) guarantees one version per package, so a
//! single slot serves the whole workspace. `vibedeps/` is committed to the
//! repository — a fresh clone is bootable with no `vibe install`, and the
//! dependency corpus stays visible and diffable.
//!
//! This module owns only the **layout** and the **verbatim copy**. It is
//! additive: it never retires the legacy `[writes]` mirror layout
//! (`VIBEVM-SPEC.md` §13.1). That retirement is the `vibe install`
//! switch-over — a later PROP-009 phase — and removing the mirror path
//! before `vibe install` is rebuilt on `vibedeps/` would break the build.

specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-009#two-trees");

use std::fs;
use std::path::{Path, PathBuf};

use vibe_core::PackageKind;

use crate::WorkspaceError;

/// Directory name of the materialisation tree, at the workspace root.
pub const VIBEDEPS_DIR: &str = "vibedeps";

/// The slot path for one resolved package, relative to the workspace root
/// and forward-slashed: `vibedeps/<kind>-<name>/<version>`.
///
/// Root-relative and forward-slashed so it is portable across machines —
/// the same property [`WorkspaceMember::rel_path`](crate::WorkspaceMember)
/// carries.
pub fn slot_rel_path(kind: PackageKind, name: &str, version: &semver::Version) -> String {
    format!("{VIBEDEPS_DIR}/{kind}-{name}/{version}")
}

/// The absolute on-disk slot path — `workspace_root` joined with
/// [`slot_rel_path`]. In-memory only; never persist an absolute path.
pub fn slot_abs_path(
    workspace_root: &Path,
    kind: PackageKind,
    name: &str,
    version: &semver::Version,
) -> PathBuf {
    let mut p = workspace_root.join(VIBEDEPS_DIR);
    p.push(format!("{kind}-{name}"));
    p.push(version.to_string());
    p
}

/// `true` iff the slot for this package already exists on disk.
pub fn is_materialised(
    workspace_root: &Path,
    kind: PackageKind,
    name: &str,
    version: &semver::Version,
) -> bool {
    slot_abs_path(workspace_root, kind, name, version).is_dir()
}

/// Materialise a resolved package into its `vibedeps/` slot — copy the
/// package's published content tree (`content_src`) verbatim into
/// `vibedeps/<kind>-<name>/<version>/`.
///
/// **Idempotent.** An existing slot is cleared first, so re-materialising
/// the same package yields a byte-identical slot and stale files from an
/// earlier content revision never linger.
///
/// A `.git` entry in the source is skipped at every depth — a materialised
/// slot is plain content committed into the outer repository, never a
/// nested repository. Symlinks are skipped: a committed dependency tree
/// must be portable, and a published package ships plain files.
///
/// Returns the slot-relative paths of every file written, forward-slashed
/// and sorted, so the caller can report — and, in a later phase, record —
/// the materialised footprint.
pub fn materialise(
    workspace_root: &Path,
    kind: PackageKind,
    name: &str,
    version: &semver::Version,
    content_src: &Path,
) -> Result<Vec<PathBuf>, WorkspaceError> {
    let slot = slot_abs_path(workspace_root, kind, name, version);
    let slot_label = slot_rel_path(kind, name, version);

    if !content_src.is_dir() {
        return Err(WorkspaceError::Io {
            path: content_src.to_path_buf(),
            reason: format!(
                "source content tree for `{slot_label}` does not exist or is not a directory"
            ),
        });
    }

    // Idempotent: clear an existing slot so the result is exactly the
    // source — no leftovers from an earlier content revision.
    if slot.exists() {
        fs::remove_dir_all(&slot).map_err(|e| io_err(&slot, e))?;
    }
    fs::create_dir_all(&slot).map_err(|e| io_err(&slot, e))?;

    let mut written: Vec<PathBuf> = Vec::new();
    copy_tree(content_src, content_src, &slot, &mut written)?;
    written.sort();
    Ok(written)
}

/// Remove a package's `vibedeps/` slot, if it exists. Returns `true` when a
/// slot was present and deleted, `false` when there was nothing to remove.
pub fn remove_slot(
    workspace_root: &Path,
    kind: PackageKind,
    name: &str,
    version: &semver::Version,
) -> Result<bool, WorkspaceError> {
    let slot = slot_abs_path(workspace_root, kind, name, version);
    if !slot.exists() {
        return Ok(false);
    }
    fs::remove_dir_all(&slot).map_err(|e| io_err(&slot, e))?;
    Ok(true)
}

/// Recursively copy the contents of `dir` into the slot at `dest_root`.
/// `src_root` is the materialisation source root; every copied file's path
/// relative to it (forward-slashed) is pushed to `written`.
fn copy_tree(
    dir: &Path,
    src_root: &Path,
    dest_root: &Path,
    written: &mut Vec<PathBuf>,
) -> Result<(), WorkspaceError> {
    for entry in fs::read_dir(dir).map_err(|e| io_err(dir, e))? {
        let entry = entry.map_err(|e| io_err(dir, e))?;
        // `.git` is never materialised — a slot is plain committed content,
        // not a repository (whether `.git` is a directory or a gitlink file).
        if entry.file_name() == ".git" {
            continue;
        }
        let path = entry.path();
        let file_type = entry.file_type().map_err(|e| io_err(&path, e))?;
        if file_type.is_dir() {
            copy_tree(&path, src_root, dest_root, written)?;
        } else if file_type.is_file() {
            let rel = path
                .strip_prefix(src_root)
                .map_err(|_| WorkspaceError::Io {
                    path: path.clone(),
                    reason: format!("walked path escaped its copy root `{}`", src_root.display()),
                })?;
            let dest = dest_root.join(rel);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).map_err(|e| io_err(parent, e))?;
            }
            fs::copy(&path, &dest).map_err(|e| io_err(&dest, e))?;
            written.push(PathBuf::from(crate::path_to_slash(rel)));
        }
        // A symlink is neither a dir nor a file via the non-following
        // `file_type` — it falls through and is skipped (see the docs).
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
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn version(s: &str) -> semver::Version {
        semver::Version::parse(s).unwrap()
    }

    /// Write `body` to `dir/rel`, creating parent directories.
    fn write(dir: &Path, rel: &str, body: &str) {
        let path = dir.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    #[test]
    fn slot_rel_path_is_kind_name_version() {
        let rel = slot_rel_path(PackageKind::Flow, "wal", &version("0.3.0"));
        assert_eq!(rel, "vibedeps/flow-wal/0.3.0");
    }

    #[test]
    fn slot_abs_path_joins_under_workspace_root() {
        let root = Path::new("ws-root");
        let abs = slot_abs_path(root, PackageKind::Stack, "rust", &version("2.1.0"));
        assert!(abs.starts_with(root));
        assert!(abs.ends_with(Path::new("vibedeps/stack-rust/2.1.0")));
    }

    #[test]
    fn materialise_copies_the_tree_verbatim() {
        let ws = TempDir::new().unwrap();
        let src = TempDir::new().unwrap();
        write(
            src.path(),
            "vibe.toml",
            "[package]\ngroup = \"org.vibevm\"\nname = \"wal\"\n",
        );
        write(src.path(), "boot/10-flow-wal.md", "# boot");
        write(src.path(), "spec/flows/wal/WAL.md", "# protocol");

        let written = materialise(
            ws.path(),
            PackageKind::Flow,
            "wal",
            &version("0.3.0"),
            src.path(),
        )
        .unwrap();

        let slot = ws.path().join("vibedeps/flow-wal/0.3.0");
        assert_eq!(
            fs::read_to_string(slot.join("vibe.toml")).unwrap(),
            "[package]\ngroup = \"org.vibevm\"\nname = \"wal\"\n"
        );
        assert_eq!(
            fs::read_to_string(slot.join("boot/10-flow-wal.md")).unwrap(),
            "# boot"
        );
        assert_eq!(
            fs::read_to_string(slot.join("spec/flows/wal/WAL.md")).unwrap(),
            "# protocol"
        );
        // The returned footprint is slot-relative, forward-slashed, sorted.
        assert_eq!(
            written,
            vec![
                PathBuf::from("boot/10-flow-wal.md"),
                PathBuf::from("spec/flows/wal/WAL.md"),
                PathBuf::from("vibe.toml"),
            ]
        );
    }

    #[test]
    fn materialise_skips_dot_git() {
        let ws = TempDir::new().unwrap();
        let src = TempDir::new().unwrap();
        write(src.path(), "vibe.toml", "x");
        write(src.path(), ".git/config", "[core]");
        write(src.path(), ".git/objects/ab/cdef", "blob");
        // A `.git` nested deeper than the root is skipped too.
        write(src.path(), "boot/.git/HEAD", "ref: refs/heads/main");
        write(src.path(), "boot/snippet.md", "# snippet");

        let written = materialise(
            ws.path(),
            PackageKind::Flow,
            "w",
            &version("1.0.0"),
            src.path(),
        )
        .unwrap();

        let slot = ws.path().join("vibedeps/flow-w/1.0.0");
        assert!(slot.join("vibe.toml").is_file());
        assert!(slot.join("boot/snippet.md").is_file());
        assert!(!slot.join(".git").exists());
        assert!(!slot.join("boot/.git").exists());
        assert_eq!(
            written,
            vec![PathBuf::from("boot/snippet.md"), PathBuf::from("vibe.toml")]
        );
    }

    #[test]
    fn materialise_is_idempotent_and_clears_stale_files() {
        let ws = TempDir::new().unwrap();
        let src1 = TempDir::new().unwrap();
        write(src1.path(), "vibe.toml", "v1");
        write(src1.path(), "stale.md", "remove me");
        materialise(
            ws.path(),
            PackageKind::Feat,
            "auth",
            &version("0.1.0"),
            src1.path(),
        )
        .unwrap();

        // Re-materialise from a source that no longer carries `stale.md`.
        let src2 = TempDir::new().unwrap();
        write(src2.path(), "vibe.toml", "v2");
        let written = materialise(
            ws.path(),
            PackageKind::Feat,
            "auth",
            &version("0.1.0"),
            src2.path(),
        )
        .unwrap();

        let slot = ws.path().join("vibedeps/feat-auth/0.1.0");
        assert_eq!(fs::read_to_string(slot.join("vibe.toml")).unwrap(), "v2");
        assert!(
            !slot.join("stale.md").exists(),
            "stale file must be cleared"
        );
        assert_eq!(written, vec![PathBuf::from("vibe.toml")]);
    }

    #[test]
    fn materialise_errors_when_source_missing() {
        let ws = TempDir::new().unwrap();
        let missing = ws.path().join("no-such-source");
        let err = materialise(
            ws.path(),
            PackageKind::Flow,
            "ghost",
            &version("0.1.0"),
            &missing,
        )
        .unwrap_err();
        assert!(matches!(err, WorkspaceError::Io { .. }), "{err}");
    }

    #[test]
    fn is_materialised_reflects_slot_presence() {
        let ws = TempDir::new().unwrap();
        let src = TempDir::new().unwrap();
        write(src.path(), "vibe.toml", "x");
        assert!(!is_materialised(
            ws.path(),
            PackageKind::Tool,
            "fmt",
            &version("1.0.0")
        ));
        materialise(
            ws.path(),
            PackageKind::Tool,
            "fmt",
            &version("1.0.0"),
            src.path(),
        )
        .unwrap();
        assert!(is_materialised(
            ws.path(),
            PackageKind::Tool,
            "fmt",
            &version("1.0.0")
        ));
    }

    #[test]
    fn remove_slot_deletes_and_reports() {
        let ws = TempDir::new().unwrap();
        let src = TempDir::new().unwrap();
        write(src.path(), "vibe.toml", "x");
        materialise(
            ws.path(),
            PackageKind::Flow,
            "wal",
            &version("0.3.0"),
            src.path(),
        )
        .unwrap();

        assert!(remove_slot(ws.path(), PackageKind::Flow, "wal", &version("0.3.0")).unwrap());
        assert!(!is_materialised(
            ws.path(),
            PackageKind::Flow,
            "wal",
            &version("0.3.0")
        ));
        // A second removal finds nothing to do.
        assert!(!remove_slot(ws.path(), PackageKind::Flow, "wal", &version("0.3.0")).unwrap());
    }
}
