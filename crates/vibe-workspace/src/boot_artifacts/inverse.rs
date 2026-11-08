//! The inverse of one boot regeneration, held until its operation commits.
//!
//! An install or scoped update regenerates every boot lane before it
//! publishes `vibe.lock`. The transaction manager makes each per-directory
//! publication atomic and crash-recoverable, but a committed publication has
//! no inverse of its own: once its journal is gone, a stop or failure that
//! arrives before the lock is published would leave the boot lanes and the
//! managed `<vibevm>` blocks describing a world the project never committed.
//!
//! [`BootImages`] is that inverse. It records the generated files the
//! regeneration may rewrite — each boot directory's `INDEX.md` and both
//! STATIC spellings, and each node's agent instruction files — before the
//! regeneration runs, and again right after it. Undo then puts back only the
//! files this operation changed, and only while each still holds exactly the
//! bytes this operation left: a file that already matches its before-image is
//! never touched, and a file someone else changed meanwhile is preserved and
//! reported, so a foreign edit outside (or inside) a managed block is never
//! overwritten. A directory whose transaction journal is still pending is the
//! journal's to recover and is reported rather than restored, because that
//! recovery could otherwise roll the new bytes forward again.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use vibe_core::layout;

use super::redirect::REDIRECT_FILES;
use super::{INDEX_FILE, STATIC_FILE, STATIC_XML_FILE};
use crate::{WorkspaceError, safe_file};

/// One generated file's state before the regeneration, and what it left.
#[derive(Debug)]
struct Generated {
    path: PathBuf,
    before: Option<(Vec<u8>, fs::Permissions)>,
    /// `Err` when the post-regeneration read failed: undo then cannot prove
    /// a changed file is this operation's and preserves it.
    after: Result<Option<Vec<u8>>, String>,
}

/// The generated files one boot directory's lock serialises.
#[derive(Debug)]
struct Owner {
    boot_dir: PathBuf,
    existed: bool,
    files: Vec<Generated>,
}

/// Before- and after-images of every generated boot file one regeneration
/// may rewrite. Dropping it is the commit.
#[derive(Debug, Default)]
pub(crate) struct BootImages {
    owners: Vec<Owner>,
}

impl BootImages {
    /// Record the generated files under each node directory in `nodes` (its
    /// boot lane and agent instruction files) and each package unit boot
    /// directory in `units`, before a regeneration writes any of them.
    pub(crate) fn capture(
        nodes: impl IntoIterator<Item = PathBuf>,
        units: impl IntoIterator<Item = PathBuf>,
    ) -> Result<Self, WorkspaceError> {
        let lane = [INDEX_FILE, STATIC_FILE, STATIC_XML_FILE];
        let mut owners: Vec<Owner> = Vec::new();
        let mut add = |boot_dir: PathBuf, paths: Vec<PathBuf>| -> Result<(), WorkspaceError> {
            if owners.iter().any(|owner| owner.boot_dir == boot_dir) {
                return Ok(());
            }
            let files = paths
                .into_iter()
                .map(|path| {
                    let before = read_with_permissions(&path)?;
                    Ok(Generated {
                        path,
                        before,
                        after: Ok(None),
                    })
                })
                .collect::<Result<_, WorkspaceError>>()?;
            owners.push(Owner {
                existed: fs::symlink_metadata(&boot_dir).is_ok(),
                boot_dir,
                files,
            });
            Ok(())
        };
        for node in nodes {
            let boot_dir = node.join(layout::current_boot_dir());
            let mut paths: Vec<PathBuf> = lane.iter().map(|name| boot_dir.join(name)).collect();
            paths.extend(REDIRECT_FILES.iter().map(|name| node.join(name)));
            add(boot_dir, paths)?;
        }
        for boot_dir in units {
            let paths = lane.iter().map(|name| boot_dir.join(name)).collect();
            add(boot_dir, paths)?;
        }
        Ok(Self { owners })
    }

    /// Record what the regeneration left, whether it succeeded or failed.
    pub(crate) fn settle(&mut self) {
        for file in self.owners.iter_mut().flat_map(|owner| &mut owner.files) {
            file.after = safe_file::read_optional(&file.path)
                .map_err(|error| format!("{}: {error}", file.path.display()));
        }
    }

    /// Put back every file this regeneration changed, newest owner first.
    /// Returns the first refusal; every other owner is still restored.
    pub(crate) fn restore(self) -> Result<(), String> {
        let mut first_error = None;
        for owner in self.owners.into_iter().rev() {
            if let Err(error) = owner.restore() {
                first_error.get_or_insert(error);
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}

impl Owner {
    fn restore(self) -> Result<(), String> {
        let mut changed = Vec::new();
        for file in &self.files {
            let current =
                safe_file::read_optional(&file.path).map_err(|e| describe(&file.path, e))?;
            if current.as_deref() == file.before.as_ref().map(|(bytes, _)| bytes.as_slice()) {
                continue;
            }
            match &file.after {
                Ok(after) if *after == current => changed.push(file),
                Ok(_) => {
                    return Err(format!(
                        "{}: changed after this operation regenerated it; the newer bytes \
                         are preserved and this boot lane still describes the stopped world",
                        file.path.display()
                    ));
                }
                Err(reason) => {
                    return Err(format!(
                        "{reason}; could not prove the regenerated bytes are this operation's, \
                         so they are preserved"
                    ));
                }
            }
        }
        if changed.is_empty() {
            return Ok(());
        }
        if let Some(journal) = super::transaction::pending_journal(&self.boot_dir) {
            return Err(format!(
                "{}: a boot-artifact transaction is still pending; its recovery owns this \
                 directory, so the regenerated files are preserved",
                journal.display()
            ));
        }
        // Present files first, removals last: an interrupted undo then never
        // leaves an INDEX naming a STATIC lane that is already gone.
        changed.sort_by_key(|file| file.before.is_none());
        super::transaction::with_boot_lock(&self.boot_dir, || {
            for file in &changed {
                put_back(&file.path, file.before.as_ref()).map_err(|reason| {
                    WorkspaceError::Io {
                        path: file.path.clone(),
                        reason,
                    }
                })?;
            }
            Ok(())
        })
        .map_err(|error| error.to_string())?;
        if !self.existed {
            // The lock and the directory exist only because this operation
            // wrote here; a directory holding anything else is left alone.
            let _ = fs::remove_file(self.boot_dir.join(layout::BOOT_ARTIFACTS_LOCK));
            let _ = fs::remove_dir(&self.boot_dir);
        }
        Ok(())
    }
}

fn read_with_permissions(
    path: &Path,
) -> Result<Option<(Vec<u8>, fs::Permissions)>, WorkspaceError> {
    let io = |error: std::io::Error| WorkspaceError::Io {
        path: path.to_path_buf(),
        reason: error.to_string(),
    };
    let Some(bytes) = safe_file::read_optional(path).map_err(io)? else {
        return Ok(None);
    };
    let permissions = fs::symlink_metadata(path).map_err(io)?.permissions();
    Ok(Some((bytes, permissions)))
}

/// Replace `path` with its before-image through a same-directory rename, or
/// remove it when it did not exist.
fn put_back(path: &Path, before: Option<&(Vec<u8>, fs::Permissions)>) -> Result<(), String> {
    let Some((bytes, permissions)) = before else {
        return match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.to_string()),
        };
    };
    let parent = path
        .parent()
        .ok_or_else(|| "generated file has no parent directory".to_owned())?;
    let mut staged = tempfile::Builder::new()
        .prefix(".vibe-boot-restore-")
        .tempfile_in(parent)
        .map_err(|error| error.to_string())?;
    staged.write_all(bytes).map_err(|error| error.to_string())?;
    staged
        .as_file()
        .set_permissions(permissions.clone())
        .map_err(|error| error.to_string())?;
    staged
        .as_file()
        .sync_all()
        .map_err(|error| error.to_string())?;
    staged
        .persist(path)
        .map(drop)
        .map_err(|error| error.error.to_string())
}

fn describe(path: &Path, error: std::io::Error) -> String {
    format!("{}: {error}", path.display())
}

#[cfg(test)]
#[path = "inverse_tests.rs"]
mod tests;
