//! The durable half: pinned capabilities, the two publication laws, and the
//! guarded reopen of an existing run.
//!
//! Two writes with two different laws live here, and the difference is the
//! whole design:
//!
//! * a **snapshot** is written once and never again, so it is published
//!   create-new — an existing file, a directory, a link or another name of the
//!   same inode all refuse, and a successfully written snapshot is never
//!   replaced by a later event;
//! * the **index** is rewritten continuously, so it is published by the
//!   existing staged atomic-replace writer — a reader always sees either the
//!   previous whole index or the next whole index, never a torn one.
//!
//! Both go through [`vibe_safefs`] capabilities pinned when the run opened, so
//! a directory swapped into an ancestor afterwards cannot redirect either.
//! Neither is part of the boot-artifact transaction: a failed compile
//! deliberately leaves its partial diagnostic run behind.
//!
//! A publication failure is reported with its [`PublishStage`] intact, because
//! the two stages license different bookkeeping, and collapsing them gets the
//! answer wrong in BOTH directions.
//!
//! `BeforePublication` proves nothing landed: for the index the previous whole
//! file is still the only claim, and for a snapshot nothing is charged.
//! `PossiblyPublished` proves nothing either way, so it is resolved rather
//! than assumed:
//!
//! * the **index** destination is re-read and compared byte-for-byte to what
//!   this call serialized. If those exact bytes are safely visible and still
//!   validate, the update IS durable — a run whose terminal index is on disk
//!   must not call itself unfinished — and the fault is kept as a warning
//!   rather than erased;
//! * a **snapshot** past that step is charged at its ATTEMPTED length, without
//!   asking. A probe cannot answer here: a stage-cleanup failure leaves the
//!   payload under two names, which every exclusive-ownership check correctly
//!   refuses, so probing would report "nothing there" about a full payload. A
//!   diagnostic that can leave bytes on disk it does not count is a diagnostic
//!   whose budget is decorative.
//!
//! Because the two writes are independently atomic and never one transaction,
//! a crash between them is a real state: a snapshot on disk that no index
//! names. The reopen below therefore REFUSES such a directory and names the
//! residue rather than guessing which of the two halves to believe — and it
//! never deletes it, because a residue nobody can explain is exactly the thing
//! a diagnostic tool must keep.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE");

use std::path::{Path, PathBuf};

use vibe_safefs::{Pinned, Project, PublishStage};
use vibe_wire::behaviour::compiler_trace_index::validate;
use vibe_wire::generated::compiler_trace_index::e1::index::CompilerTraceIndex;

use super::{TraceOpenError, bounded};

/// The pinned run directory and everything a write needs to reach it.
#[derive(Debug)]
pub(super) struct TraceStore {
    project: Project,
    run_dir: Pinned,
    filename_cap: usize,
}

/// The relative name of the one metadata file of a run.
pub(super) const INDEX_FILE: &str = "index.json";

/// What an index update did. The three arms are three different obligations,
/// and collapsing them is exactly how a run reports itself finished while the
/// terminal bytes never reached the disk — or, in the other direction, how a
/// run calls itself unfinished while the terminal bytes ARE the cold reader's
/// truth.
pub(super) enum IndexUpdate {
    /// The bytes are durably on disk. `Some` carries a bounded note that the
    /// publication reported a fault *after* its irreversible step, and the
    /// destination was then re-read and proved to hold exactly these bytes.
    /// The fault is kept rather than erased: something did go wrong, and the
    /// operator should see it even though the outcome is sound.
    Written(Option<String>),
    /// The I/O failed. The in-memory model is still the truth, the previous
    /// whole index is still readable, and a later update retries.
    Deferred(String),
    /// The index would break its own epoch. Nothing was attempted; the caller
    /// rolls the change back.
    Refused(String),
}

/// A snapshot publication that did not deliver a named file, and what it may
/// nevertheless have left on disk.
pub(super) struct SnapshotRefusal {
    pub(super) reason: String,
    /// Present exactly when the refusal happened at or after the irreversible
    /// step. The value is the ATTEMPTED payload length, charged
    /// conservatively: past that step a full payload may be at the final name
    /// under one name or two, and the two-name case is invisible to any probe
    /// that insists on exclusive ownership. Counting the attempt is the only
    /// accounting that cannot under-count. It is one payload's bytes, never
    /// two, however many directory entries point at that inode.
    pub(super) landed: Option<u64>,
}

impl TraceStore {
    pub(super) fn new(project: Project, run_dir: Pinned, filename_cap: usize) -> Self {
        Self {
            project,
            run_dir,
            filename_cap,
        }
    }

    /// The absolute run directory, for diagnostics and for the caller's
    /// summary — never re-opened with ambient authority.
    pub(super) fn run_path(&self) -> &Path {
        self.run_dir.path()
    }

    /// How many units of filename this run directory can afford.
    pub(super) const fn filename_cap(&self) -> usize {
        self.filename_cap
    }

    /// Replace `index.json` atomically, reporting which of the three
    /// obligations the caller now has.
    pub(super) fn write_index(&self, index: &CompilerTraceIndex) -> IndexUpdate {
        if let Err(error) = validate(index) {
            return IndexUpdate::Refused(bounded::diagnostic(format_args!(
                "the trace index would break its own `{}` law: {error}",
                error.law()
            )));
        }
        let mut bytes = match serde_json::to_vec_pretty(index) {
            Ok(bytes) => bytes,
            Err(error) => {
                return IndexUpdate::Refused(bounded::diagnostic(format_args!(
                    "serializing the trace index: {error}"
                )));
            }
        };
        bytes.push(b'\n');
        let Err(error) = self
            .project
            .write_atomic_in(&self.run_dir, INDEX_FILE, &bytes)
        else {
            return IndexUpdate::Written(None);
        };
        let stage = error.stage;
        let reason = bounded::diagnostic(format_args!(
            "writing `{}`: {:#}",
            self.run_dir.join(INDEX_FILE).display(),
            error.into_report()
        ));
        match stage {
            // Provably invisible: the previous whole index is still the only
            // claim, and the next update retries.
            PublishStage::BeforePublication => IndexUpdate::Deferred(reason),
            // The replace may already BE the cold reader's truth. Asking the
            // disk is the only way to know, and the answer is what decides
            // whether this run may call itself finished.
            PublishStage::PossiblyPublished => {
                before_index_recovery(&self.run_dir.join(INDEX_FILE));
                if self.index_is_exactly(&bytes) {
                    IndexUpdate::Written(Some(reason))
                } else {
                    IndexUpdate::Deferred(reason)
                }
            }
        }
    }

    /// Whether `index.json` is, right now, safely readable as EXACTLY these
    /// bytes — final newline included — and still validates.
    ///
    /// `read_file_in` is what makes it "safely": a link, a reparse point, a
    /// directory or a hard-linked file is not a readable claim at all. The
    /// comparison is against the bytes THIS call serialized rather than
    /// against a fresh re-serialisation, so a formatting drift can never be
    /// mistaken for the same document.
    fn index_is_exactly(&self, bytes: &[u8]) -> bool {
        let Ok(Some(visible)) = self.project.read_file_in(&self.run_dir, INDEX_FILE) else {
            return false;
        };
        visible == bytes
            && serde_json::from_slice::<CompilerTraceIndex>(&visible)
                .is_ok_and(|index| validate(&index).is_ok())
    }

    /// Publish one snapshot create-new.
    ///
    /// A refusal is a bounded reason plus the fact only this layer can
    /// establish: whether bytes are physically at the final name despite the
    /// failure. `BeforePublication` never probes — that stage is a proof of
    /// absence, and probing it would be spending a syscall to doubt a
    /// guarantee the publication already made.
    pub(super) fn publish_snapshot(&self, name: &str, bytes: &[u8]) -> Result<(), SnapshotRefusal> {
        let Err(error) = self.project.publish_new_in(&self.run_dir, name, bytes) else {
            return Ok(());
        };
        let stage = error.stage;
        let reason = bounded::diagnostic(format_args!(
            "publishing `{}`: {:#}",
            self.run_dir.join(name).display(),
            error.into_report()
        ));
        let landed = match stage {
            // A proof of absence. Probing it would be spending a syscall to
            // doubt a guarantee the publication already made.
            PublishStage::BeforePublication => None,
            // The irreversible step was crossed, so the payload may be on
            // disk — possibly under two names, where a stage cleanup failed.
            // `inspect_file_in` REFUSES a two-named file (it is not
            // exclusively owned), so a probe would answer "nothing there"
            // about a full payload. The attempt is charged instead; the probe
            // below only sharpens the diagnostic.
            PublishStage::PossiblyPublished => Some(bytes.len() as u64),
        };
        Err(SnapshotRefusal { reason, landed })
    }
}

/// What one `index.json` turned out to be. Every arm except `Present` is a
/// reason to leave a directory alone rather than a reason to act on it.
pub(super) enum IndexRead {
    /// No index at that name at all.
    Missing,
    /// Present but not something this reader may trust: a link, a hard link,
    /// a directory, unreadable bytes, invalid JSON, or an index the epoch's
    /// own validator refuses.
    Refused(String),
    Present(Box<CompilerTraceIndex>),
}

/// Read and fully judge one run directory's index through pinned
/// capabilities. Parsing is PERMISSIVE at the object boundary by registry
/// policy; the relational laws are not.
pub(super) fn read_index(project: &Project, directory: &Pinned) -> IndexRead {
    let bytes = match project.read_file_in(directory, INDEX_FILE) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return IndexRead::Missing,
        Err(error) => {
            return IndexRead::Refused(bounded::diagnostic(format_args!(
                "reading `{}`: {error:#}",
                directory.join(INDEX_FILE).display()
            )));
        }
    };
    let index: CompilerTraceIndex = match serde_json::from_slice(&bytes) {
        Ok(index) => index,
        Err(error) => {
            return IndexRead::Refused(bounded::diagnostic(format_args!(
                "`{}` is not a readable trace index: {error}",
                directory.join(INDEX_FILE).display()
            )));
        }
    };
    if let Err(error) = validate(&index) {
        return IndexRead::Refused(bounded::diagnostic(format_args!(
            "`{}` breaks the `{}` law: {error}",
            directory.join(INDEX_FILE).display(),
            error.law()
        )));
    }
    IndexRead::Present(Box::new(index))
}

/// Which files a run directory is allowed to contain: its index, and exactly
/// the snapshots its own events name.
pub(super) fn owned_entries(index: &CompilerTraceIndex) -> Vec<&str> {
    let mut names = vec![INDEX_FILE];
    names.extend(
        index
            .events
            .iter()
            .filter_map(|event| event.snapshot.as_deref()),
    );
    names
}

/// The guarded reopen of a run directory that already exists.
///
/// Never a blind overwrite and never a merge: the directory either IS the same
/// still-running trace this process is resuming — same run id, same project,
/// same start, every referenced file present and safe, and nothing else in it
/// — or it is residue, and residue is reported, not repaired.
pub(super) fn reopen(
    project: &Project,
    run_dir: &Pinned,
    expected: &CompilerTraceIndex,
) -> Result<(Box<CompilerTraceIndex>, u64), TraceOpenError> {
    let residue = |reason: String| TraceOpenError::Residue {
        path: bounded::path(run_dir.path()),
        reason,
    };
    let index = match read_index(project, run_dir) {
        IndexRead::Present(index) => index,
        IndexRead::Missing => {
            return Err(residue(bounded::diagnostic(format_args!(
                "the run directory exists but carries no `{INDEX_FILE}`"
            ))));
        }
        IndexRead::Refused(reason) => return Err(residue(reason)),
    };
    if index.status != expected.status {
        return Err(residue(bounded::diagnostic(format_args!(
            "the run is already terminal (`{:?}`); a finished trace is never reopened",
            index.status
        ))));
    }
    if index.run_id != expected.run_id {
        return Err(residue(bounded::diagnostic(format_args!(
            "the index names run `{}`, not this run",
            bounded::preview(&index.run_id)
        ))));
    }
    if index.project != expected.project {
        return Err(residue(
            "the index was written for a different project root".to_string(),
        ));
    }
    if index.started != expected.started {
        return Err(residue(
            "the index records a different start time for this run id".to_string(),
        ));
    }

    let owned = owned_entries(&index);
    let present = project
        .child_names(run_dir)
        .map_err(|error| residue(bounded::diagnostic(format_args!("{error:#}"))))?;
    if let Some(extra) = present.iter().find(|name| !owned.contains(&name.as_str())) {
        return Err(residue(bounded::diagnostic(format_args!(
            "`{}` is not an entry this run wrote",
            bounded::preview(extra)
        ))));
    }
    let mut spent: u64 = 0;
    for name in owned {
        let Ok(Some((_, len))) = project.inspect_file_in(run_dir, name) else {
            return Err(residue(bounded::diagnostic(format_args!(
                "`{}` is referenced by the index but is missing or is not an ordinary owned file",
                bounded::preview(name)
            ))));
        };
        if name != INDEX_FILE {
            spent = spent.saturating_add(len);
        }
    }
    Ok((index, spent))
}

/// The absolute path of a run directory below a CANONICAL project root — the
/// value the filename ceiling is measured against, and the one this writer
/// displays.
pub(super) fn run_directory_path(canonical_root: &Path, run_id: &str) -> PathBuf {
    canonical_root.join(".vibe").join("trace").join(run_id)
}

/// Deterministic namespace mutation between a post-publication error and the
/// exact-byte recovery read. It makes the recovery guard independently red;
/// compiled out of every shipped build.
#[cfg(test)]
pub(crate) use inject::arm as arm_before_index_recovery;

#[cfg(test)]
fn before_index_recovery(path: &Path) {
    inject::run(path);
}

#[cfg(not(test))]
fn before_index_recovery(_path: &Path) {}

#[cfg(test)]
mod inject {
    use std::cell::RefCell;
    use std::path::Path;

    type Hook = Box<dyn Fn(&Path)>;

    thread_local! {
        static BEFORE_INDEX_RECOVERY: RefCell<Option<Hook>> = const { RefCell::new(None) };
    }

    pub(crate) fn arm(hook: Option<Hook>) {
        BEFORE_INDEX_RECOVERY.with(|slot| *slot.borrow_mut() = hook);
    }

    pub(super) fn run(path: &Path) {
        let hook = BEFORE_INDEX_RECOVERY.with(|slot| slot.borrow_mut().take());
        if let Some(hook) = hook {
            hook(path);
        }
    }
}
