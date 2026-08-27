//! Retention: keep the newest nine complete runs, and touch nothing else.
//!
//! This is the only code in the writer that DELETES, so it is written as a
//! proof obligation rather than a cleanup. A run directory is deletion
//! eligible only when every one of these holds:
//!
//! 1. its name is an exact 32-lowercase-hex run id;
//! 2. it opens as a real directory without following a link or reparse point;
//! 3. its `index.json` reads through a pinned capability as an ordinary owned
//!    file and parses to the generated type;
//! 4. that index passes the epoch's own relational validator;
//! 5. the index names ITSELF — `run_id` equals the directory name — and names
//!    THIS project — `project` equals the identity of the canonical root this
//!    sweep is running under. Together those two are what make the directory
//!    trace-owned rather than merely trace-shaped;
//! 6. the run is terminal (`ok` or `failed`), never `running`;
//! 7. the exact set of files the index references, plus the index, IS the
//!    whole directory.
//!
//! Anything that fails any of those survives and is reported as residue. That
//! includes the cases somebody planted deliberately (a link, a hostile
//! spelling, another project's index) and the ones a crash left behind (a
//! snapshot no index names, a run that never wrote its terminal word) — a
//! diagnostic tool that deletes what it cannot explain is worse than one that
//! keeps it.
//!
//! ## Deleting the object, not the name
//!
//! Eligibility is decided by opening things. Deletion happens afterwards. In
//! between, a name can be rebound — and a sweep that then removed *by name*
//! would delete whatever is there now, having judged something else. So every
//! candidate carries an opaque [`EntryProof`] for each of its snapshots, for
//! its `index.json`, and for the run directory itself, taken from the very
//! handles the judgement read. Deletion CONSUMES those proofs: `vibe-safefs`
//! re-derives each one through the same pinned capability and refuses unless
//! it is still the same object. There is no by-name fallback on this path.
//!
//! Ordering is `index.started`, tie-broken by run id so two runs that share a
//! timestamp still order the same way on every host. The architecture allows
//! directory mtime as a fallback ordering fact — but `started` is a REQUIRED
//! member of an index that has already passed the validator, so that fallback
//! is unreachable here, and reaching for a syscall to break a tie the data
//! already breaks would be inventing an authority nobody needs.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE");

use vibe_safefs::{EntryProof, Pinned, Project};
use vibe_wire::generated::compiler_trace_index::e1::index::{
    ProjectIdentity, RunStatus, Timestamp,
};

use super::identity::is_run_id;
use super::store::{INDEX_FILE, IndexRead, owned_entries, read_index};
use super::{TraceWarning, bounded};

/// One directory this sweep proved it may delete, and the proofs that license
/// removing exactly the objects it judged.
struct Complete {
    name: String,
    started: Timestamp,
    /// Every owned file — the snapshots and `index.json` — with the proof of
    /// WHICH object each name held at judgement time.
    files: Vec<(String, EntryProof)>,
    /// The capability the judgement read through. Removals of the files go
    /// through THIS handle, never through a re-walked path.
    pinned: Pinned,
    /// The proof of the run directory itself, for the final removal — which
    /// needs the handle released first.
    directory: EntryProof,
}

/// Sweep `.vibe/trace` down to `keep` complete runs, before the new one is
/// created. Returns everything that survived for a reason worth reporting.
pub(super) fn sweep(
    project: &Project,
    trace_dir: &Pinned,
    keep: usize,
    expected: &ProjectIdentity,
) -> Vec<TraceWarning> {
    let mut warnings = Vec::new();
    let names = match project.child_names(trace_dir) {
        Ok(names) => names,
        Err(error) => {
            warnings.push(TraceWarning::Residue {
                path: bounded::path(trace_dir.path()),
                reason: bounded::diagnostic(format_args!(
                    "the trace directory cannot be listed: {error:#}"
                )),
            });
            return warnings;
        }
    };

    let mut complete: Vec<Complete> = Vec::new();
    for name in names {
        if !is_run_id(&name) {
            warnings.push(residue(
                trace_dir,
                &name,
                "not an exact 32-lowercase-hex run id; retention never inspects it",
            ));
            continue;
        }
        match judge(project, trace_dir, &name, expected) {
            Judgement::Complete(run) => complete.push(*run),
            Judgement::Vanished => {}
            Judgement::Survives(reason) => warnings.push(residue(trace_dir, &name, &reason)),
        }
    }

    // Oldest first, so the tail this drops is the tail that is oldest.
    complete.sort_by(|left, right| {
        left.started
            .cmp(&right.started)
            .then_with(|| left.name.cmp(&right.name))
    });
    let doomed = complete.len().saturating_sub(keep);
    for run in complete.into_iter().take(doomed) {
        let name = run.name.clone();
        if let Some(reason) = remove(project, trace_dir, run) {
            warnings.push(residue(trace_dir, &name, &reason));
        }
    }
    warnings
}

enum Judgement {
    /// Every obligation discharged: this directory may be deleted.
    Complete(Box<Complete>),
    /// It disappeared between the listing and the open. Nothing to say.
    Vanished,
    /// It stays, and this is why.
    Survives(String),
}

/// Discharge obligations 2–7 for one candidate, collecting the proofs that
/// would license removing exactly what was judged.
fn judge(
    project: &Project,
    trace_dir: &Pinned,
    name: &str,
    expected: &ProjectIdentity,
) -> Judgement {
    let directory = match trace_dir.open_child_checked(name) {
        Ok(Some(directory)) => directory,
        Ok(None) => return Judgement::Vanished,
        Err(error) => {
            return Judgement::Survives(bounded::diagnostic(format_args!(
                "does not open as a link-free directory: {error:#}"
            )));
        }
    };
    let index = match read_index(project, &directory) {
        IndexRead::Present(index) => index,
        IndexRead::Missing => {
            return Judgement::Survives(format!("carries no `{INDEX_FILE}`"));
        }
        IndexRead::Refused(reason) => return Judgement::Survives(reason),
    };
    if index.run_id != name {
        return Judgement::Survives(bounded::diagnostic(format_args!(
            "its index names run `{}`, so this directory is not the run it holds",
            bounded::preview(&index.run_id)
        )));
    }
    if index.project != *expected {
        return Judgement::Survives(
            "its index was written for a different project root; this sweep does not own it"
                .to_string(),
        );
    }
    if index.status == RunStatus::Running {
        return Judgement::Survives("is still running".to_string());
    }
    let owned = owned_entries(&index);
    let present = match project.child_names(&directory) {
        Ok(present) => present,
        Err(error) => {
            return Judgement::Survives(bounded::diagnostic(format_args!(
                "cannot be listed: {error:#}"
            )));
        }
    };
    if let Some(extra) = present.iter().find(|name| !owned.contains(&name.as_str())) {
        return Judgement::Survives(bounded::diagnostic(format_args!(
            "holds `{}`, which its own index does not reference",
            bounded::preview(extra)
        )));
    }
    let mut files = Vec::with_capacity(owned.len());
    for entry in &owned {
        match project.inspect_file_in(&directory, entry) {
            Ok(Some((proof, _))) => files.push(((*entry).to_string(), proof)),
            Ok(None) => {
                return Judgement::Survives(bounded::diagnostic(format_args!(
                    "references `{}`, which is not there",
                    bounded::preview(entry)
                )));
            }
            Err(error) => {
                return Judgement::Survives(bounded::diagnostic(format_args!(
                    "references `{}`, which is not an ordinary owned file: {error:#}",
                    bounded::preview(entry)
                )));
            }
        }
    }
    let proof = match directory.proof() {
        Ok(proof) => proof,
        Err(error) => {
            return Judgement::Survives(bounded::diagnostic(format_args!(
                "could not be identified: {error:#}"
            )));
        }
    };
    Judgement::Complete(Box::new(Complete {
        name: name.to_string(),
        started: index.started,
        files,
        pinned: directory,
        directory: proof,
    }))
}

/// Remove exactly the proved objects, then the proved directory. `Some` is the
/// reason something survived after all.
///
/// Every removal is identity-bound: a name rebound between the judgement and
/// here is a [`vibe_safefs::ProofRefusal`] whose `changed()` is true, and the
/// sweep stops on that run and reports it rather than deleting whatever
/// arrived. Snapshots go first, `index.json` last — so a sweep interrupted
/// midway leaves a directory whose own index no longer matches its contents,
/// which the next open reports as residue instead of silently adopting.
fn remove(project: &Project, trace_dir: &Pinned, run: Complete) -> Option<String> {
    let Complete {
        name,
        files,
        pinned,
        directory,
        ..
    } = run;
    for (entry, proof) in files.iter().filter(|(entry, _)| entry != INDEX_FILE) {
        if let Err(refusal) = project.remove_file_proved_in(&pinned, entry, proof) {
            return Some(bounded::diagnostic(format_args!("{refusal}")));
        }
    }
    if let Some((entry, proof)) = files.iter().find(|(entry, _)| entry == INDEX_FILE)
        && let Err(refusal) = project.remove_file_proved_in(&pinned, entry, proof)
    {
        return Some(bounded::diagnostic(format_args!("{refusal}")));
    }
    // Released before the directory removal: Windows refuses to delete a
    // directory an open handle still names, and from here the PROOF is what
    // travels rather than the handle.
    drop(pinned);
    match project.remove_dir_proved_in(trace_dir, &name, &directory) {
        Ok(()) => None,
        Err(refusal) => Some(bounded::diagnostic(format_args!("{refusal}"))),
    }
}

fn residue(trace_dir: &Pinned, name: &str, reason: &str) -> TraceWarning {
    TraceWarning::Residue {
        path: bounded::path(&trace_dir.join(name)),
        reason: reason.to_string(),
    }
}
