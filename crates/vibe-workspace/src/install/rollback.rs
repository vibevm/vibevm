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
//!   to) is snapshotted and written back;
//! - a boot regeneration run through [`InstallRollback::regenerate_boot`]
//!   records the generated boot files it may rewrite before and after it
//!   runs; undo puts back only the ones it changed, and only while they still
//!   hold the bytes it wrote, so the boot lanes and managed blocks describe
//!   the restored tree, never the stopped one.
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
    CheckedPlacement {
        path: PathBuf,
        stashed: Option<PathBuf>,
        after: super::checked_placement::Tree,
        /// Ancestors this apply created for the placement, deepest first.
        ancestors: Vec<PathBuf>,
    },
    CheckedIgnore(super::checked_placement::Ignore),
    Created {
        path: PathBuf,
        /// Ancestors missing when the creation was recorded, deepest first.
        ancestors: Vec<PathBuf>,
    },
    MovedAside {
        path: PathBuf,
        stashed: PathBuf,
    },
    Rewritten {
        path: PathBuf,
        before: Option<Vec<u8>>,
    },
    BootArtifacts(crate::boot_artifacts::BootImages),
}

/// Every reversible change one apply made, in order.
///
/// Public so a scoped `vibe update`, which prunes superseded slots itself
/// before its subtree materialisation, records that prune in the same log
/// and can put the tree back when the update stops or fails before its lock
/// is published. Dropping it is the commit.
#[derive(Debug)]
pub struct InstallRollback {
    stash_parent: PathBuf,
    stash: Option<TempDir>,
    steps: Vec<Step>,
    /// The operation whose completed step effects (an agent step's published
    /// outputs) join this log: undone with it when a stop rolls the operation
    /// back — see [`Self::with_cancellation`].
    cancellation: Option<vibe_core::cancellation::CancellationToken>,
}

impl InstallRollback {
    /// Copy into this owner's same-volume stash before moving aside a caller
    /// slot. The source may live on another filesystem; only owned paths rename.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION-LAW"
    )]
    pub(crate) fn place_checked(
        &mut self,
        slot: &Path,
        source: &Path,
        validate: impl FnOnce() -> Result<(), String>,
    ) -> Result<(), WorkspaceError> {
        let expected = super::checked_placement::Tree::read(source)?;
        let candidate = self.stash_entry()?.with_extension("candidate");
        copy_tree(source, &candidate)?;
        let after = super::checked_placement::Tree::read(&candidate)?;
        if !expected.same_content(&after) {
            return Err(WorkspaceError::SpecMaterialization {
                path: slot.into(),
                reason: "prepared copy differs from retained source".into(),
            });
        }
        let ancestors = missing_ancestors(slot);
        if let Some(parent) = slot.parent() {
            fs::create_dir_all(parent).map_err(|e| io_err(parent, e))?;
        }
        validate().map_err(|reason| WorkspaceError::SpecMaterialization {
            path: slot.into(),
            reason,
        })?;
        let stashed = match fs::symlink_metadata(slot) {
            Ok(_) => {
                let stash = self.stash_entry()?;
                fs::rename(slot, &stash).map_err(|e| io_err(slot, e))?;
                Some(stash)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(io_err(slot, e)),
        };
        self.steps.push(Step::CheckedPlacement {
            path: slot.into(),
            stashed,
            after,
            ancestors,
        });
        fs::rename(&candidate, slot).map_err(|e| io_err(slot, e))
    }
    pub(crate) fn ignore_checked(
        &mut self,
        root: &Path,
        entry: &str,
    ) -> Result<(), WorkspaceError> {
        if let Some(ignore) = self.steps.iter_mut().find_map(|step| match step {
            Step::CheckedIgnore(ignore) => Some(ignore),
            _ => None,
        }) {
            return ignore
                .publish(entry)
                .map_err(|reason| WorkspaceError::SpecMaterialization {
                    path: root.join(".gitignore"),
                    reason,
                });
        }
        let mut ignore = super::checked_placement::Ignore::capture(root)?;
        if let Some(bytes) = ignore.before_bytes() {
            let stash = self.stash_entry()?.with_extension("gitignore");
            fs::write(&stash, bytes).map_err(|e| io_err(&stash, e))?;
        }
        let result = ignore.publish(entry);
        self.steps.push(Step::CheckedIgnore(ignore));
        result.map_err(|reason| WorkspaceError::SpecMaterialization {
            path: root.join(".gitignore"),
            reason,
        })
    }
    pub fn new(workspace_root: &Path) -> Self {
        let deps = workspace_root.join(vibe_core::layout::current_vibedeps_root());
        let stash_parent = deps
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| workspace_root.to_path_buf());
        Self {
            stash_parent,
            stash: None,
            steps: Vec::new(),
            cancellation: None,
        }
    }

    /// Own the completed step effects `cancellation`'s operation adopted
    /// ([`vibe_core::cancellation::CancellationToken::adopt_completed_effect`]):
    /// when an acknowledged stop asks [`Self::restore`] to put the tree back,
    /// those effects are undone first, newest first, each through its own
    /// receipt-checked inverse, so a later stop restores an earlier completed
    /// agent step's outputs together with the slots and boot files. A foreign
    /// replacement is preserved and named in the restore failure.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS-LAW"
    )]
    #[must_use]
    pub fn with_cancellation(
        mut self,
        cancellation: vibe_core::cancellation::CancellationToken,
    ) -> Self {
        self.cancellation = Some(cancellation);
        self
    }

    /// Before `slot` is reconciled in place: a missing slot is recorded as
    /// created, a present one is copied into the stash.
    pub(crate) fn before_reconcile(&mut self, slot: &Path) -> Result<(), WorkspaceError> {
        if fs::symlink_metadata(slot).is_err() {
            self.steps.push(Step::Created {
                path: slot.to_path_buf(),
                ancestors: missing_ancestors(slot),
            });
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
    pub fn remove(&mut self, dir: &Path) -> Result<(), WorkspaceError> {
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

    /// Run `regenerate` — one boot regeneration over `resolution` — with its
    /// inverse in this log. Every node's boot lane and agent instruction
    /// files, and every resolved package's unit lane, are recorded first; what
    /// the regeneration left is recorded after it, whether it succeeded or
    /// failed, so an undo restores exactly the files it changed.
    pub fn regenerate_boot<T>(
        &mut self,
        workspace: &crate::Workspace,
        resolution: &[super::ResolvedDep],
        regenerate: impl FnOnce() -> Result<T, WorkspaceError>,
    ) -> Result<T, WorkspaceError> {
        let nodes = workspace
            .iter_nodes()
            .map(|(rel, _)| workspace.node_abs_path(rel))
            .collect::<Vec<_>>();
        let units = resolution
            .iter()
            .map(|dep| {
                workspace
                    .root
                    .join(super::bootgen::unit_slot_rel_path(dep))
                    .join(vibe_core::layout::current_boot_dir())
            })
            .collect::<Vec<_>>();
        let mut images = crate::boot_artifacts::BootImages::capture(nodes, units)?;
        let result = regenerate();
        images.settle();
        self.steps.push(Step::BootArtifacts(images));
        result
    }

    fn move_aside(&mut self, path: &Path) -> Result<(), WorkspaceError> {
        if fs::symlink_metadata(path).is_err() {
            self.steps.push(Step::Created {
                path: path.to_path_buf(),
                ancestors: missing_ancestors(path),
            });
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

    /// Undo every recorded change, newest first, and tell `observer` that each
    /// of `materialised` is no longer a change this apply made. On failure the
    /// stash is retained and its path named in the returned reason.
    pub fn restore(
        &mut self,
        observer: Option<&dyn SlotLifecycle>,
        materialised: &[String],
    ) -> Result<(), String> {
        let mut first_error = None;
        // A failure that is not a stop keeps a completed step's outputs (a
        // park resumes over them); only an acknowledged stop rolls them back.
        if let Some(token) = self
            .cancellation
            .as_ref()
            .filter(|token| token.acknowledged())
            && let Err(error) = token.undo_completed_effects()
        {
            first_error = Some(error);
        }
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
        if let Some(mut error) = first_error {
            if let Some(stash) = self.stash.take() {
                let path = stash.keep();
                error.push_str(&format!("; recovery backup retained at {}", path.display()));
            }
            Err(error)
        } else {
            Ok(())
        }
    }
}

fn undo_step(step: Step) -> Result<(), String> {
    match step {
        Step::BootArtifacts(images) => images.restore(),
        Step::CheckedIgnore(ignore) => ignore.undo(),
        Step::CheckedPlacement {
            path,
            stashed,
            after,
            ancestors,
        } => {
            match fs::symlink_metadata(&path) {
                Ok(_) => {
                    if super::checked_placement::Tree::read(&path).map_err(|e| e.to_string())?
                        != after
                    {
                        return Err(format!(
                            "{}: rollback conflict; changed replacement preserved",
                            path.display()
                        ));
                    }
                    remove_any(&path)?;
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(describe(&path, e)),
            }
            match stashed {
                Some(stashed) => fs::rename(stashed, &path).map_err(|e| describe(&path, e))?,
                None => remove_created_ancestors(&ancestors),
            }
            Ok(())
        }
        Step::Created { path, ancestors } => {
            remove_any(&path)?;
            // A `<kind>-<name>` directory this apply created for the slot is
            // empty again; one that held other versions is left alone.
            if let Some(parent) = path.parent() {
                let _ = fs::remove_dir(parent);
            }
            remove_created_ancestors(&ancestors);
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

/// The ancestors of `path` that do not exist yet, deepest first: what an
/// apply that creates `path` also creates, and what its undo removes again.
fn missing_ancestors(path: &Path) -> Vec<PathBuf> {
    path.ancestors()
        .skip(1)
        .take_while(|dir| !dir.as_os_str().is_empty() && fs::symlink_metadata(dir).is_err())
        .map(Path::to_path_buf)
        .collect()
}

/// Remove the directories an undone creation made, deepest first, each only
/// while it is empty: anything another writer put there keeps it.
fn remove_created_ancestors(ancestors: &[PathBuf]) {
    for dir in ancestors {
        match fs::remove_dir(dir) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return,
        }
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
    fs::set_permissions(
        to,
        fs::metadata(from)
            .map_err(|e| io_err(from, e))?
            .permissions(),
    )
    .map_err(|e| io_err(to, e))
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
