//! `mirror::probe` — decide one target's sync state relative to local
//! mainline by ancestry (the fan-out's gate), not equality. Split out from
//! the push/fan-out machinery so the decision matrix is its own unit that
//! table-tests offline, without git or the network.

use std::path::Path;

use anyhow::Result;

use super::{Target, git, load_targets, local_main, remote_main};

/// One target's sync state relative to local mainline — the result both
/// `mirror --check` and `health --mirrors` read. The contract is ancestry,
/// not equality (the fan-out's gate): a target whose `main` is an ancestor of
/// mainline is `Behind` — legitimately behind and healthy — never `Drift`.
/// The carried `String` is the target's `main` sha (for the health JSON).
pub(crate) enum SyncState {
    InSync,
    /// Ancestor of mainline: behind, healthy (the normal state between two
    /// fan-outs).
    Behind(String),
    /// Not an ancestor — diverged, or its tip is an object we do not have.
    Drift(String),
    Missing,
}

pub(crate) struct TargetStatus {
    pub name: String,
    pub state: SyncState,
}

/// The ancestry verdict for "is `remote` an ancestor of `head`?" — the
/// tri-state `git merge-base --is-ancestor` reduces to, so the pure
/// classifier below is testable without git or the network.
enum Ancestry {
    /// `remote` is an ancestor of `head` (exit 0).
    IsAncestor,
    /// `remote` is provably not an ancestor (exit 1).
    NotAncestor,
    /// git could not answer — `remote` is absent from the local object store
    /// (exit 128, "Not a valid object name"): the target went ahead or
    /// diverged and we never fetched its commits.
    Unknown,
}

/// `git merge-base --is-ancestor <remote> <head>`, reduced to an `Ancestry`.
/// The remote sha comes from `ls-remote`, so the local store may lack it —
/// git then exits 128 rather than 1. We treat exit code 1 as the honest
/// `NotAncestor` and any other non-zero (object missing / git error) as
/// `Unknown` ⇒ drift, so a missing object never aborts the probe.
fn is_ancestor(root: &Path, remote: &str, head: &str) -> Ancestry {
    match git(root, &["merge-base", "--is-ancestor", remote, head]) {
        Ok(o) if o.status.success() => Ancestry::IsAncestor,
        Ok(o) => match o.status.code() {
            Some(1) => Ancestry::NotAncestor,
            _ => Ancestry::Unknown,
        },
        // git failed to spawn (io) — no answer we can act on ⇒ drift.
        Err(_) => Ancestry::Unknown,
    }
}

/// Pure decision: the sync state for a target whose tip is `remote`, given
/// local `head` and the ancestry verdict. Equality is settled first (the only
/// truly-green state); a non-equal ancestor is `Behind` (healthy); a
/// non-ancestor or an unknown object is `Drift`. `ancestry` is consulted only
/// when `remote == Some(sha)` with `sha != head` — for equal/missing the
/// value passed is ignored. Extracted so the matrix is table-tested offline.
fn classify(head: &str, remote: Option<&str>, ancestry: Ancestry) -> SyncState {
    match remote {
        None => SyncState::Missing,
        Some(sha) if sha == head => SyncState::InSync,
        Some(sha) => match ancestry {
            Ancestry::IsAncestor => SyncState::Behind(sha.to_string()),
            Ancestry::NotAncestor | Ancestry::Unknown => SyncState::Drift(sha.to_string()),
        },
    }
}

/// Probe every target's `main` against local mainline by ancestry (the
/// fan-out's gate): equal ⇒ sync, an ancestor ⇒ behind (healthy), anything
/// else ⇒ drift. Shared by `mirror --check` (which fails on drift) and
/// `health --mirrors` (advisory).
pub(super) fn probe(root: &Path, targets: &[Target]) -> Result<(String, Vec<TargetStatus>)> {
    let head = local_main(root)?;
    let mut statuses = Vec::with_capacity(targets.len());
    for t in targets {
        let remote = remote_main(root, &t.url)?;
        // The ancestry gate only changes the answer when the target has a
        // main that is not equal to ours; equal ⇒ sync, absent ⇒ missing
        // (`classify` settles both without consulting `ancestry`, so the
        // placeholder below is never read in those cases).
        let ancestry = match remote.as_deref() {
            Some(sha) if sha != head => is_ancestor(root, sha, &head),
            _ => Ancestry::IsAncestor,
        };
        let state = classify(&head, remote.as_deref(), ancestry);
        statuses.push(TargetStatus {
            name: t.name.clone(),
            state,
        });
    }
    Ok((head, statuses))
}

/// Load the manifest and probe every target — the entry `health --mirrors`
/// calls (it carries no loaded targets of its own).
pub(crate) fn sync_report(root: &Path) -> Result<(String, Vec<TargetStatus>)> {
    let targets = load_targets(root)?;
    probe(root, &targets)
}

#[cfg(test)]
mod tests {
    use super::{Ancestry, SyncState, classify};

    #[test]
    fn classify_sync_state_by_ancestry() {
        // `probe`'s per-target decision matrix, table-tested offline (no git,
        // no network): equality first, then ancestry; the unknown-object trap
        // (exit 128) folds to drift, never an abort.
        let head = "HEAD";
        // Equality is the only green state and wins regardless of ancestry —
        // proves `classify` does not consult ancestry when the tips are equal.
        assert!(matches!(
            classify(head, Some(head), Ancestry::IsAncestor),
            SyncState::InSync
        ));
        assert!(matches!(
            classify(head, Some(head), Ancestry::NotAncestor),
            SyncState::InSync
        ));
        // A non-equal ancestor ⇒ behind (healthy); a non-ancestor, or an
        // object the local store lacks, ⇒ drift.
        assert!(matches!(
            classify(head, Some("anc"), Ancestry::IsAncestor),
            SyncState::Behind(_)
        ));
        assert!(matches!(
            classify(head, Some("div"), Ancestry::NotAncestor),
            SyncState::Drift(_)
        ));
        assert!(matches!(
            classify(head, Some("unk"), Ancestry::Unknown),
            SyncState::Drift(_)
        ));
        // No `main` on the target at all ⇒ missing.
        assert!(matches!(
            classify(head, None, Ancestry::Unknown),
            SyncState::Missing
        ));
    }
}
