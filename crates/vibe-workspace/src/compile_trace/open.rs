//! The two ways a run directory is reached, and the gates they share.
//!
//! There are exactly two: the ordinary one, which may CREATE the run (and
//! sweeps retention before it does), and the existing-only one, which may not.
//! They are one cell because the interesting half is what they have in common:
//! every gate that runs before either touches `.vibe/trace` — the checked run
//! id, the ONE canonicalisation, the path-pressure measurement, the project
//! capability and the nonblocking cooperative lock — is shared verbatim rather
//! than copied. A second spelling of any of them would be a second definition
//! of what "this project's run" means.
//!
//! The order of those gates is load-bearing and is the ordinary open's order,
//! unchanged:
//!
//! 1. the run id is exactly 32 lowercase hex, checked before a path is built;
//! 2. the root canonicalises ONCE, and that single spelling is the identity;
//! 3. the run directory's path pressure is measured — a directory that cannot
//!    afford a snapshot name refuses to open rather than failing every event —
//!    and this happens BEFORE any capability or lock is taken, so a hopeless
//!    depth costs no handle and no contention;
//! 4. the project capability opens;
//! 5. the cooperative lock is taken, nonblocking.
//!
//! Only then do the two paths differ, and the difference is exactly one
//! question: may this call bring the directory into existence?
//!
//! **The lock precedes every presence check on purpose.** `Busy` and `Ok(None)`
//! are different answers to different questions, and a writer that looked first
//! would answer "there is no run" about a directory another writer is at that
//! moment creating. Contention is decided before existence is, so a busy
//! project always says `Busy` — never absence.
//!
//! The existing-only path is what a displaced or adopted run needs: it must be
//! able to say "that trace never opened" without MAKING it exist. So it walks
//! with capability-pinned presence checks — `dir_if_present`, then
//! `open_child_checked` — and only a genuine `NotFound` becomes `Ok(None)`. An
//! unreadable, link-like or non-directory ancestor is a `Directory` refusal and
//! a present-but-unsafe run child is `Residue`: absence is a claim, and this
//! writer only makes it when the filesystem actually proved it.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE");

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use vibe_safefs::{LockGuard, Project};
use vibe_wire::generated::compiler_trace_index::e1::index::{CompilerTraceIndex, Timestamp};

use super::state::RunState;
use super::store::TraceStore;
use super::{
    TRACE_LOCK, TraceLimits, TraceOpenError, TraceRun, bounded, fresh_index, identity, retention,
    store,
};

/// Everything both paths establish before either looks at `.vibe/trace`.
///
/// The lock is IN here, and it is held for the whole decision: it is taken
/// before the presence walk and handed to the run state afterwards, so no
/// window exists in which this process has decided something about a directory
/// it does not own.
struct Prepared {
    run_id: String,
    run_path: PathBuf,
    filename_cap: usize,
    project: Project,
    lock: LockGuard,
    expected: CompilerTraceIndex,
}

/// The shared gates, in the ordinary open's exact order.
fn prepare(root: &Path, run_id: &str, started: Timestamp) -> Result<Prepared, TraceOpenError> {
    let run_id = identity::checked_run_id(run_id)?;
    // ONE canonical spelling, resolved once and used for everything after
    // it: the digest, the run path, the capability and the path pressure.
    // Two spellings of one root would make a reopen look like somebody
    // else's project.
    let canonical = identity::canonical_root(root)?;
    let run_path = store::run_directory_path(&canonical, &run_id);
    // Measured before anything is created: a directory that cannot afford
    // a filename must refuse to open rather than fail every event.
    let filename_cap = identity::filename_cap(&run_path)?;
    let project = Project::open(&canonical).map_err(|error| TraceOpenError::Directory {
        reason: bounded::diagnostic(format_args!("{error:#}")),
    })?;
    // Serialize every cooperating writer BEFORE anything is inspected,
    // reopened, retained or created. Non-blocking on purpose: an observer
    // that can make a compile wait on another process is an observer that
    // can deadlock one, so a busy project is simply not traced.
    let lock = match project.try_lock(TRACE_LOCK) {
        Ok(Some(lock)) => lock,
        Ok(None) => {
            return Err(TraceOpenError::Busy {
                project: bounded::path(&canonical),
            });
        }
        Err(error) => {
            return Err(TraceOpenError::Directory {
                reason: bounded::diagnostic(format_args!("{error:#}")),
            });
        }
    };
    let expected = fresh_index(&canonical, &run_id, started);
    Ok(Prepared {
        run_id,
        run_path,
        filename_cap,
        project,
        lock,
        expected,
    })
}

/// The ordinary open: reopen this run if it is already there, otherwise sweep
/// retention and create it.
pub(super) fn create_or_reopen(
    root: &Path,
    run_id: &str,
    started: Timestamp,
    limits: TraceLimits,
) -> Result<TraceRun, TraceOpenError> {
    let Prepared {
        run_id,
        run_path,
        filename_cap,
        project,
        lock,
        expected,
    } = prepare(root, run_id, started)?;
    let trace_dir =
        project
            .dir(&[".vibe", "trace"], true)
            .map_err(|error| TraceOpenError::Directory {
                reason: bounded::diagnostic(format_args!("{error:#}")),
            })?;

    let (run_dir, index, spent, warnings, adopted) = match trace_dir.open_child_checked(&run_id) {
        Ok(Some(existing)) => {
            let (index, spent) = store::reopen(&project, &existing, &expected)?;
            (existing, index, spent, Vec::new(), true)
        }
        Ok(None) => {
            let warnings = retention::sweep(
                &project,
                &trace_dir,
                limits.retained_runs,
                &expected.project,
            );
            let created = trace_dir.create_child_exclusive(&run_id).map_err(|error| {
                TraceOpenError::Residue {
                    path: bounded::path(&run_path),
                    reason: bounded::diagnostic(format_args!("{error}")),
                }
            })?;
            (created, Box::new(expected), 0, warnings, false)
        }
        Err(error) => {
            return Err(TraceOpenError::Residue {
                path: bounded::path(&run_path),
                reason: bounded::diagnostic(format_args!(
                    "does not open as a link-free directory: {error:#}"
                )),
            });
        }
    };

    let run_dir_path = run_dir.path().to_path_buf();
    let store = TraceStore::new(project, run_dir, filename_cap);
    let mut state = RunState::adopt(lock, store, *index, spent, limits, warnings);
    if !adopted {
        // A fresh run is readable the moment it exists, not once its
        // first event lands. If that first index cannot land, the
        // exclusively created directory is left EXACTLY as it is and
        // named as residue: deleting it would mean reaching for the
        // identity-bound removal path on a directory this run never
        // explained, and an unexplained empty run id is precisely what an
        // operator should get to see. Returning here drops `state`, which
        // releases the project lock.
        if let Err(reason) = state.open_index() {
            return Err(TraceOpenError::Residue {
                path: bounded::path(&run_dir_path),
                reason: bounded::diagnostic(format_args!(
                    "the run directory was created but no index landed, so nothing here \
                     describes it: {reason}"
                )),
            });
        }
    }
    Ok(TraceRun {
        inner: Arc::new(Mutex::new(state)),
        run_dir: run_dir_path,
    })
}

/// The existing-only open: reopen this run if — and only if — it is already
/// there, and otherwise say so without leaving a trace of having asked.
///
/// Every refusal a reopen can give is given here identically, because the
/// identity law is the same law: a terminal, torn, foreign or start-mismatched
/// directory is `Residue`, not absence. What differs is only that nothing is
/// created and retention never runs — so a caller asking "did that run ever
/// open?" cannot accidentally answer "yes, now".
pub(super) fn existing_only(
    root: &Path,
    run_id: &str,
    started: Timestamp,
    limits: TraceLimits,
) -> Result<Option<TraceRun>, TraceOpenError> {
    let Prepared {
        run_id,
        run_path,
        filename_cap,
        project,
        lock,
        expected,
    } = prepare(root, run_id, started)?;
    // `dir_if_present` walks one authored component at a time and answers
    // `Ok(None)` ONLY for a genuine `NotFound`. A `.vibe/trace` that is a
    // file, a link or unreadable propagates instead — absence is a claim,
    // and this path does not make it on a guess.
    let Some(trace_dir) = project
        .dir_if_present(&[".vibe", "trace"])
        .map_err(|error| TraceOpenError::Directory {
            reason: bounded::diagnostic(format_args!("{error:#}")),
        })?
    else {
        return Ok(None);
    };
    let existing = match trace_dir.open_child_checked(&run_id) {
        Ok(Some(existing)) => existing,
        Ok(None) => return Ok(None),
        Err(error) => {
            return Err(TraceOpenError::Residue {
                path: bounded::path(&run_path),
                reason: bounded::diagnostic(format_args!(
                    "does not open as a link-free directory: {error:#}"
                )),
            });
        }
    };
    // The SAME guarded reopen the ordinary path uses: same run id, same
    // project identity, same start, still running, nothing in the directory
    // its own index does not name — and the spent budget recovered from the
    // snapshots that are actually on disk.
    let (index, spent) = store::reopen(&project, &existing, &expected)?;
    let run_dir_path = existing.path().to_path_buf();
    let store = TraceStore::new(project, existing, filename_cap);
    // Warnings begin empty: they are this process's in-memory account of what
    // IT could not do, and the epoch persists none. Reconstructing a previous
    // process's warnings would mean reading prose back out of the index and
    // calling it an observation of this run.
    let state = RunState::adopt(lock, store, *index, spent, limits, Vec::new());
    Ok(Some(TraceRun {
        inner: Arc::new(Mutex::new(state)),
        run_dir: run_dir_path,
    }))
}
