//! Create-new publication: a file that must not already exist, landed
//! whole or not at all.
//!
//! [`Project::write_atomic_in`](crate::Project::write_atomic_in) publishes by
//! rename, which **replaces**. That is right for a file the caller owns and
//! rewrites — a lock, a receipt, a trace index. It is wrong for an
//! append-only archive, where a name is claimed exactly once and an entry
//! already sitting there is either somebody else's or a crash residue: a
//! rename would destroy it silently and call that success.
//!
//! So the publication step here is `hard_link` rather than `rename`. Both are
//! one directory-entry operation; the difference is that `link` **fails**
//! when the destination exists — on any host, for any occupant, including a
//! directory, a symlink, a junction and another name of the same inode. That
//! refusal is the ownership proof, and it is taken through the pinned
//! capability, so an ancestor swapped after the walk cannot redirect it.
//!
//! The cost of `link` over `rename` is that publication leaves the file with
//! **two** names for a moment — the stage and the destination — so the last
//! two steps are load-bearing rather than decorative:
//!
//! 1. the stage is removed, and
//! 2. the destination is reopened and held to
//!    [`verify_regular_single_link`](super::verify_regular_single_link).
//!
//! A surviving stage is therefore not something a caller has to notice: the
//! link count says so, and the publication reports
//! [`PublishStage::PossiblyPublished`] with the residue named. A crash between
//! the link and the removal leaves exactly that state on disk, and it is
//! readable as residue by the next run rather than mistakable for a clean
//! publication.

use std::io::Write;

use anyhow::{Result, bail};

use crate::component::ensure_safe_component;
use crate::project::{Pinned, Project};
use crate::publish::PublishError;

use super::{cap_options, create_unique_stage};

impl Project {
    /// Publish `bytes` at `name` directly inside `directory`, refusing to
    /// replace anything.
    ///
    /// `name` is one safe path component, never a path: this is the archive
    /// case, and an archive entry that could name a subdirectory would be an
    /// archive that can escape itself. Missing parents are **not** created —
    /// the caller pins the holding directory first, which is what makes the
    /// publication capability-relative.
    ///
    /// The failure carries [`PublishStage`](crate::PublishStage) with the same
    /// meaning as the replacing publication:
    /// [`BeforePublication`](crate::PublishStage::BeforePublication) is a proof
    /// that nothing at `name` was touched and no stage survives, and
    /// [`PossiblyPublished`](crate::PublishStage::PossiblyPublished) means the
    /// entry may exist — possibly with a surviving stage beside it — and the
    /// caller must treat it as residue rather than as either outcome.
    pub fn publish_new_in(
        &self,
        directory: &Pinned,
        name: &str,
        bytes: &[u8],
    ) -> Result<(), PublishError> {
        let before = |error: anyhow::Error| PublishError::before(Vec::new(), error);
        ensure_safe_component(name).map_err(&before)?;
        // Cheap, honest early refusal with a legible diagnostic. It is NOT
        // the authority: the `hard_link` below is, because only that refusal
        // cannot be raced.
        refuse_any_occupant(directory, name).map_err(&before)?;

        let (staged_name, staged_file) = create_unique_stage(directory).map_err(&before)?;
        let mut std_file = staged_file.into_std();
        let written = std_file
            .write_all(bytes)
            .and_then(|()| std_file.flush())
            .and_then(|()| std_file.sync_all());
        drop(std_file);
        if let Err(error) = written {
            let _ = directory.dir.remove_file(&staged_name);
            return Err(before(anyhow::Error::new(error).context(format!(
                "writing staged `{}`",
                directory.join(&staged_name).display()
            ))));
        }
        if let Some(injected) = injected_pre_publication_failure(name) {
            let _ = directory.dir.remove_file(&staged_name);
            return Err(before(injected));
        }
        // The preflight above already said the name was free, and the stage is
        // written and synced. Everything that can still contest the name now
        // happens HERE — so this is the window a race hook occupies, and the
        // `hard_link` below is the only thing left that can refuse.
        crate::race_hook::before_link(directory, name);

        if let Err(error) = directory.dir.hard_link(&staged_name, &directory.dir, name) {
            // Our own stage, claimed with `create_new`: removing it removes
            // nothing anyone else owns, and a failed link leaves `name`
            // exactly as it was.
            let _ = directory.dir.remove_file(&staged_name);
            return Err(before(anyhow::Error::new(error).context(format!(
                "publishing `{}` create-new (an existing entry refuses)",
                directory.join(name).display()
            ))));
        }

        // Past this line `name` exists. Everything below either proves it is
        // the single-link file we meant, or reports what is really there.
        let possibly = |error: anyhow::Error| PublishError::possibly(Vec::new(), error);
        if let Some(injected) = injected_stage_cleanup_failure(name) {
            // Deliberately WITHOUT removing the stage: this is the crash-shaped
            // state where the link landed and the cleanup did not, so the
            // payload has two names and `nlink == 2`.
            return Err(possibly(injected));
        }
        if let Err(error) = directory.dir.remove_file(&staged_name) {
            return Err(possibly(anyhow::Error::new(error).context(format!(
                "`{}` was published but its staging name `{}` survives, so the file has two \
                 names; treat both as residue",
                directory.join(name).display(),
                directory.join(&staged_name).display()
            ))));
        }
        // The single-link check inside `read_file_in` is what proves step
        // above actually took effect — a surviving second name cannot pass it.
        match self.read_file_in(directory, name).map_err(&possibly)? {
            Some(visible) if visible.as_slice() == bytes => {}
            Some(_) => {
                return Err(possibly(anyhow::anyhow!(
                    "published bytes of `{}` do not match the staged bytes",
                    directory.join(name).display()
                )));
            }
            None => {
                return Err(possibly(anyhow::anyhow!(
                    "published file `{}` is absent immediately after publication",
                    directory.join(name).display()
                )));
            }
        }
        // Directory sync is best-effort: some platforms do not support fsync
        // on directory handles.
        if let Ok(handle) = directory.dir.try_clone() {
            let _ = handle.into_std_file().sync_all();
        }
        if let Some(injected) = super::injected_post_publication_failure(name) {
            return Err(possibly(injected));
        }
        Ok(())
    }
}

/// Refuse **any** occupant, not merely an unpublishable one. A regular file
/// at the destination is the ordinary replace case elsewhere; here it is the
/// exact thing that must not be overwritten.
fn refuse_any_occupant(directory: &Pinned, name: &str) -> Result<()> {
    let display = directory.join(name);
    let mut options = cap_options();
    match directory.dir.open_with(name, options.read(true)) {
        Ok(_) => bail!(
            "`{}` already exists; a create-new publication never replaces an entry",
            display.display()
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        // A no-follow open of a link, a junction or a directory answers with
        // an error rather than the target, which is exactly the refusal this
        // wants — and it is reported as an occupant, not as a missing name.
        Err(error) => Err(anyhow::Error::new(error).context(format!(
            "`{}` is occupied by something that cannot be opened as a plain file without \
             following a link; a create-new publication refuses it",
            display.display()
        ))),
    }
}

/// Injection point for a failure that happens **before** the publication step,
/// so the provably-invisible branch has a deterministic counterexample. Thread
/// local and compiled out entirely unless the `inject-failures` feature is on,
/// exactly like its post-publication twin; it reads no environment.
#[cfg(any(test, feature = "inject-failures"))]
mod inject {
    use std::cell::RefCell;

    thread_local! {
        static BEFORE_PUBLISH: RefCell<Option<String>> = const { RefCell::new(None) };
        static BEFORE_STAGE_CLEANUP: RefCell<Option<String>> = const { RefCell::new(None) };
    }

    /// Make the next create-new publication of `name` **on this thread** fail
    /// with its stage already written and synced, before the link is made.
    /// Pass `None` to disarm.
    pub fn fail_before_publish(name: Option<&str>) {
        BEFORE_PUBLISH.with(|armed| *armed.borrow_mut() = name.map(str::to_string));
    }

    pub(super) fn armed_before(name: &str) -> Option<anyhow::Error> {
        BEFORE_PUBLISH.with(|armed| {
            armed
                .borrow()
                .as_deref()
                .filter(|target| *target == name)
                .map(|_| anyhow::anyhow!("injected pre-publication failure for `{name}`"))
        })
    }

    /// Make the next create-new publication of `name` **on this thread** fail
    /// after its `hard_link` has already succeeded and BEFORE the owned stage
    /// is collected. Pass `None` to disarm.
    ///
    /// This is the one fault a caller cannot simulate from outside: it leaves
    /// the payload under two names — the final one and the stage — so the
    /// entry is real but not exclusively owned, which is exactly what a crash
    /// in that window leaves behind.
    pub fn fail_before_stage_cleanup(name: Option<&str>) {
        BEFORE_STAGE_CLEANUP.with(|armed| *armed.borrow_mut() = name.map(str::to_string));
    }

    pub(super) fn armed_stage_cleanup(name: &str) -> Option<anyhow::Error> {
        BEFORE_STAGE_CLEANUP.with(|armed| {
            armed
                .borrow()
                .as_deref()
                .filter(|target| *target == name)
                .map(|_| {
                    anyhow::anyhow!(
                        "injected stage-cleanup failure for `{name}`: the payload is published                          and its staging name survives beside it"
                    )
                })
        })
    }
}

#[cfg(any(test, feature = "inject-failures"))]
pub use inject::{fail_before_publish, fail_before_stage_cleanup};

#[cfg(any(test, feature = "inject-failures"))]
fn injected_stage_cleanup_failure(name: &str) -> Option<anyhow::Error> {
    inject::armed_stage_cleanup(name)
}

#[cfg(not(any(test, feature = "inject-failures")))]
fn injected_stage_cleanup_failure(_name: &str) -> Option<anyhow::Error> {
    None
}

#[cfg(any(test, feature = "inject-failures"))]
pub(super) fn injected_pre_publication_failure(name: &str) -> Option<anyhow::Error> {
    inject::armed_before(name)
}

#[cfg(not(any(test, feature = "inject-failures")))]
pub(super) fn injected_pre_publication_failure(_name: &str) -> Option<anyhow::Error> {
    None
}

#[cfg(test)]
#[path = "create_new_tests.rs"]
mod tests;
