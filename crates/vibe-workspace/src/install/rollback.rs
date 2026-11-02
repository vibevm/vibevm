//! The undo log of one install apply (VIBEVM-SPEC `#install-workflow-in-detail`).
//!
//! An install changes the dependency tree before it knows whether the rest of
//! the command will succeed: slots are placed and pruned first, the boot lanes
//! are compiled from the placed tree, and `vibe.lock` is written last. A
//! failure anywhere between the first slot change and the lock write must
//! leave the project as it was, so every slot change goes through this log
//! first and can be undone:
//!
//! - a slot that did not exist is recorded as created; undo deletes it;
//! - a slot that is pruned or replaced whole is moved aside into a stash;
//!   undo moves it back;
//! - a slot that is reconciled in place is copied into the stash first and
//!   left where it is, so the incremental diff still works on the real files
//!   and a successful apply leaves their inodes and times alone; undo swaps
//!   the copy back in. A copy, not hardlinks: the boot-artifact lock files
//!   inside a slot refuse a second link, and a pre-install script may rewrite
//!   a slot file in place;
//! - a file the apply rewrites (the `.gitignore` an in-place slot is added
//!   to) is snapshotted and written back.
//!
//! The stash is one temporary directory beside the dependency root, on the
//! same volume so a move is a rename. Dropping the log without undoing it is
//! the commit: the stash goes and the changes stay, which is what every
//! caller that never asks for an undo has always had.
//!
//! A pre-install script's own effects are outside this log: a script is an
//! arbitrary process, and a failing one already removes its slot under
//! PROP-020's rule, which a hosted park depends on.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use super::io_err;
use super::slot_lifecycle::SlotLifecycle;
use crate::WorkspaceError;

#[derive(Debug)]
enum Step {
    Created(PathBuf),
    MovedAside {
        path: PathBuf,
        stashed: PathBuf,
    },
    Rewritten {
        path: PathBuf,
        before: Option<Vec<u8>>,
    },
}

/// Every reversible change one apply made, in order.
#[derive(Debug)]
pub(crate) struct InstallRollback {
    stash_parent: PathBuf,
    stash: Option<TempDir>,
    steps: Vec<Step>,
}

impl InstallRollback {
    pub(crate) fn new(workspace_root: &Path) -> Self {
        let deps = workspace_root.join(vibe_core::layout::current_vibedeps_root());
        let stash_parent = deps
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| workspace_root.to_path_buf());
        Self {
            stash_parent,
            stash: None,
            steps: Vec::new(),
        }
    }

    /// Before `slot` is reconciled in place: a missing slot is recorded as
    /// created, a present one is copied into the stash.
    pub(crate) fn before_reconcile(&mut self, slot: &Path) -> Result<(), WorkspaceError> {
        if fs::symlink_metadata(slot).is_err() {
            self.steps.push(Step::Created(slot.to_path_buf()));
            return Ok(());
        }
        let stashed = self.stash_entry()?;
        copy_tree(slot, &stashed)?;
        self.steps.push(Step::MovedAside {
            path: slot.to_path_buf(),
            stashed,
        });
        Ok(())
    }

    /// Before `slot` is replaced whole (an in-place clone moved in): a present
    /// slot is stashed, so the replacement finds nothing to delete.
    pub(crate) fn before_replace(&mut self, slot: &Path) -> Result<(), WorkspaceError> {
        self.move_aside(slot)
    }

    /// Remove `dir` by stashing it, so undo can put it back.
    pub(crate) fn remove(&mut self, dir: &Path) -> Result<(), WorkspaceError> {
        self.move_aside(dir)
    }

    /// Before `path` is rewritten: remember its bytes, or that it was absent.
    pub(crate) fn before_rewrite(&mut self, path: &Path) -> Result<(), WorkspaceError> {
        let before = match fs::read(path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(io_err(path, error)),
        };
        self.steps.push(Step::Rewritten {
            path: path.to_path_buf(),
            before,
        });
        Ok(())
    }

    fn move_aside(&mut self, path: &Path) -> Result<(), WorkspaceError> {
        if fs::symlink_metadata(path).is_err() {
            self.steps.push(Step::Created(path.to_path_buf()));
            return Ok(());
        }
        let stashed = self.stash_entry()?;
        fs::rename(path, &stashed).map_err(|error| io_err(path, error))?;
        self.steps.push(Step::MovedAside {
            path: path.to_path_buf(),
            stashed,
        });
        Ok(())
    }

    /// A fresh path inside the stash, which is created on first use so an
    /// apply that only adds slots never makes one.
    fn stash_entry(&mut self) -> Result<PathBuf, WorkspaceError> {
        let stash = match &self.stash {
            Some(stash) => stash.path().to_path_buf(),
            None => {
                fs::create_dir_all(&self.stash_parent)
                    .map_err(|error| io_err(&self.stash_parent, error))?;
                let stash = tempfile::Builder::new()
                    .prefix(".vibe-install-rollback-")
                    .tempdir_in(&self.stash_parent)
                    .map_err(|error| io_err(&self.stash_parent, error))?;
                let path = stash.path().to_path_buf();
                self.stash = Some(stash);
                path
            }
        };
        Ok(stash.join(self.steps.len().to_string()))
    }

    /// Undo every recorded change, newest first, and tell `observer` that each
    /// of `materialised` is no longer a change this apply made. Returns
    /// `primary` — the failure that asked for the undo — or, when the undo
    /// itself failed, that failure wrapped with the first undo error.
    pub(crate) fn undo(
        mut self,
        primary: WorkspaceError,
        observer: Option<&dyn SlotLifecycle>,
        materialised: &[String],
    ) -> WorkspaceError {
        match self.restore(observer, materialised) {
            Ok(()) => primary,
            Err(reason) => WorkspaceError::InstallRollback {
                primary: Box::new(primary),
                reason,
            },
        }
    }

    pub(crate) fn restore(
        &mut self,
        observer: Option<&dyn SlotLifecycle>,
        materialised: &[String],
    ) -> Result<(), String> {
        let mut first_error = None;
        for step in std::mem::take(&mut self.steps).into_iter().rev() {
            if let Err(error) = undo_step(step) {
                first_error.get_or_insert(error);
            }
        }
        if let Some(observer) = observer {
            for slot in materialised {
                observer.rolled_back(slot);
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}

fn undo_step(step: Step) -> Result<(), String> {
    match step {
        Step::Created(path) => {
            remove_any(&path)?;
            // A `<kind>-<name>` directory this apply created for the slot is
            // empty again; one that held other versions is left alone.
            if let Some(parent) = path.parent() {
                let _ = fs::remove_dir(parent);
            }
            Ok(())
        }
        Step::MovedAside { path, stashed } => {
            remove_any(&path)?;
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|error| describe(parent, error))?;
            }
            fs::rename(&stashed, &path).map_err(|error| describe(&path, error))
        }
        Step::Rewritten { path, before } => match before {
            Some(bytes) => fs::write(&path, bytes).map_err(|error| describe(&path, error)),
            None => remove_any(&path),
        },
    }
}

fn remove_any(path: &Path) -> Result<(), String> {
    let removed = match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(path),
        Ok(_) => fs::remove_file(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => Err(error),
    };
    removed.map_err(|error| describe(path, error))
}

fn describe(path: &Path, error: std::io::Error) -> String {
    format!("{}: {error}", path.display())
}

/// Copy `from` to `to`: regular files byte for byte, with their modification
/// time where the platform allows, and symbolic links recreated.
fn copy_tree(from: &Path, to: &Path) -> Result<(), WorkspaceError> {
    fs::create_dir_all(to).map_err(|error| io_err(to, error))?;
    for entry in fs::read_dir(from).map_err(|error| io_err(from, error))? {
        let entry = entry.map_err(|error| io_err(from, error))?;
        let source = entry.path();
        let target = to.join(entry.file_name());
        let kind = entry.file_type().map_err(|error| io_err(&source, error))?;
        if kind.is_dir() {
            copy_tree(&source, &target)?;
        } else if kind.is_symlink() {
            copy_symlink(&source, &target)?;
        } else {
            fs::copy(&source, &target).map_err(|error| io_err(&target, error))?;
            // Best effort: the bytes are what an undo must restore, and a
            // read-only copy may refuse a new time on some platforms.
            if let Ok(modified) = fs::metadata(&source).and_then(|meta| meta.modified()) {
                let _ = fs::File::open(&target).and_then(|file| file.set_modified(modified));
            }
        }
    }
    Ok(())
}

#[cfg(unix)]
fn copy_symlink(source: &Path, target: &Path) -> Result<(), WorkspaceError> {
    let link = fs::read_link(source).map_err(|error| io_err(source, error))?;
    std::os::unix::fs::symlink(link, target).map_err(|error| io_err(target, error))
}

#[cfg(not(unix))]
fn copy_symlink(source: &Path, target: &Path) -> Result<(), WorkspaceError> {
    // Creating a symbolic link needs a privilege Windows rarely grants; the
    // pointed-at bytes are the closest faithful copy.
    fs::copy(source, target)
        .map(drop)
        .map_err(|error| io_err(target, error))
}

#[cfg(test)]
#[path = "rollback_tests.rs"]
mod tests;
