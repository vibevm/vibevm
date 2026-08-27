//! What a scoped `vibe update` has ALREADY made durable, measured as it
//! happens.
//!
//! ## Why an outer accumulator exists at all
//!
//! A scoped update mutates the project before an [`InstallSlotLifecycle`]
//! exists: it `git fetch`-es every in-place slot onto its own working tree, and
//! it removes every superseded versioned slot. Both are irreversible facts
//! about the operator's disk. If the run then fails — the provisional world
//! cannot be built, the lifecycle state refuses, a later prune hits an
//! unremovable directory — the report must say what already happened.
//!
//! The failure draft used to be `InstallProgress::default()`, which claims a
//! run that moved nothing. That is not a small inaccuracy: an operator reading
//! it concludes the tree is untouched and retries or reverts on that basis,
//! while several slots are in fact gone and several others have been advanced.
//!
//! ## Why it survives the lifecycle
//!
//! Once the lifecycle exists it owns the prune prefix (transferred once, via
//! `record_pruned`) and everything the materialise pass measures. It does NOT
//! own the in-place mutations, because those preceded the completed resolution
//! it was constructed from. So a post-lifecycle failure draft is the JOIN of
//! the two — [`Measured::joined`] — and the join is deduplicating by
//! construction: whatever the lifecycle already reports is never added twice,
//! and `pruned` is deliberately taken from the lifecycle alone, because that
//! is precisely where this accumulator's copy was transferred to.
//!
//! Success and park keep `lifecycle.progress()` UNCHANGED. Those outcomes have
//! characterised bytes, the accumulator is only a truthfulness floor for
//! failures, and widening a green report is not this cell's job.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use vibe_install::InstallProgress;

/// The durable prefix of a scoped update, in the order it really happened.
#[derive(Debug, Default, Clone)]
pub(super) struct Measured {
    /// In-place slots whose working tree this run really advanced.
    in_place_changed: Vec<String>,
    /// In-place slots this run fetched and found already current.
    in_place_unchanged: Vec<String>,
    /// Superseded versioned slots this run really removed, in removal order.
    pruned: Vec<String>,
    /// One line per package whose locked version moved. Prose for a human, and
    /// recorded per package BEFORE its slot removal is attempted: the bump is
    /// a fact about the resolution, and stays true whether or not the stale
    /// slot could then be deleted.
    bumps: Vec<String>,
}

impl Measured {
    /// An in-place slot this run fetched onto. `changed` is the registry's own
    /// answer, never inferred from a timestamp.
    pub(super) fn record_in_place(&mut self, slot: String, changed: bool) {
        if changed {
            self.in_place_changed.push(slot);
        } else {
            self.in_place_unchanged.push(slot);
        }
    }

    pub(super) fn record_bump(&mut self, line: String) {
        self.bumps.push(line);
    }

    /// A removal that REALLY removed something. A slot already absent (or an
    /// unversioned in-place slot, which has no superseded copy) removed
    /// nothing and is not recorded — `pruned` names paths this run deleted.
    pub(super) fn record_pruned(&mut self, slot: String) {
        self.pruned.push(slot);
    }

    /// Sort the prune list, as the completed command has always reported it.
    ///
    /// Called only on the success path: a partial list left by a failed
    /// removal keeps removal order, which is the more useful shape for reading
    /// "it got this far".
    pub(super) fn sort_pruned(&mut self) {
        self.pruned.sort();
    }

    pub(super) fn bumps(&self) -> &[String] {
        &self.bumps
    }

    pub(super) fn pruned(&self) -> &[String] {
        &self.pruned
    }

    /// The progress a failure draft reports while NO lifecycle exists yet.
    pub(super) fn progress(&self) -> InstallProgress {
        InstallProgress {
            // Whatever happened, the command did not finish.
            complete: false,
            fresh: false,
            materialised: self.in_place_changed.clone(),
            skipped: self.in_place_unchanged.clone(),
            pruned: self.pruned.clone(),
            nodes_regenerated: Vec::new(),
        }
    }

    /// The progress a failure draft reports once the lifecycle exists.
    ///
    /// Every list is merged the same way — prefix first, nothing repeated —
    /// including `pruned`, even though the prune prefix is normally transferred
    /// into the run by `record_pruned` before this is ever called.
    ///
    /// Deduplicating rather than DEFERRING to the run is the whole point. A
    /// version that read `pruned` from the run alone would be correct only
    /// while the transfer happens on every path out, and silently report zero
    /// removed slots the moment one path forgets — which is precisely the
    /// class of bug this accumulator exists to make impossible. Merging is
    /// right whether or not the transfer happened, and the dedup keeps it to
    /// one copy when it did.
    pub(super) fn joined(&self, mut progress: InstallProgress) -> InstallProgress {
        progress.complete = false;
        prefix(&mut progress.materialised, &self.in_place_changed);
        prefix(&mut progress.skipped, &self.in_place_unchanged);
        prefix(&mut progress.pruned, &self.pruned);
        progress
    }
}

/// Put `earlier` in front of `into`, without repeating anything already there.
///
/// Order is chronology: the in-place fetches happened before the materialise
/// pass whose list `into` is. Deduplication is what makes the join safe to
/// apply whether or not the later pass independently reported the same slot.
fn prefix(into: &mut Vec<String>, earlier: &[String]) {
    let mut merged: Vec<String> = earlier
        .iter()
        .filter(|slot| !into.iter().any(|existing| existing == *slot))
        .cloned()
        .collect();
    merged.append(into);
    *into = merged;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slots(measured: &Measured) -> (InstallProgress, InstallProgress) {
        (
            measured.progress(),
            measured.joined(InstallProgress {
                complete: true,
                fresh: false,
                materialised: vec!["vibedeps/org.demo.later/1.0.0".into()],
                skipped: Vec::new(),
                pruned: vec!["vibedeps/org.demo.later/0.9.0".into()],
                nodes_regenerated: vec![".".into()],
            }),
        )
    }

    /// The pre-lifecycle shape: everything this accumulator knows, and a
    /// `complete` that is false whatever else is true.
    #[test]
    fn the_prefix_alone_describes_a_failure_before_the_lifecycle_exists() {
        let mut measured = Measured::default();
        measured.record_in_place("vibedeps/org.demo.tools".into(), true);
        measured.record_in_place("vibedeps/org.demo.quiet".into(), false);
        measured.record_bump("org.demo/tools 0.1.0 -> 0.1.1".into());
        measured.record_pruned("vibedeps/org.demo.tools/0.1.0".into());

        let (progress, _) = slots(&measured);
        assert!(!progress.complete);
        assert_eq!(progress.materialised, ["vibedeps/org.demo.tools"]);
        assert_eq!(progress.skipped, ["vibedeps/org.demo.quiet"]);
        assert_eq!(progress.pruned, ["vibedeps/org.demo.tools/0.1.0"]);
        assert_eq!(measured.bumps(), ["org.demo/tools 0.1.0 -> 0.1.1"]);
    }

    /// The join: the in-place prefix comes forward, the run's own lists
    /// survive, and a slot the transfer already carried appears EXACTLY once.
    ///
    /// The duplication this refuses is the one a naive concatenation makes:
    /// the accumulator's prune list is normally transferred into the run by
    /// `record_pruned`, so appending it again would report every removed slot
    /// twice.
    #[test]
    fn the_join_prepends_the_prefix_and_never_doubles_the_prune_list() {
        let mut measured = Measured::default();
        measured.record_in_place("vibedeps/org.demo.tools".into(), true);
        measured.record_pruned("vibedeps/org.demo.later/0.9.0".into());

        let (_, joined) = slots(&measured);
        assert!(!joined.complete, "a join is only ever built for a failure");
        assert_eq!(
            joined.materialised,
            ["vibedeps/org.demo.tools", "vibedeps/org.demo.later/1.0.0"],
            "chronology: the in-place fetch preceded the materialise pass",
        );
        assert_eq!(
            joined.pruned,
            ["vibedeps/org.demo.later/0.9.0"],
            "one copy — the transfer and the accumulator agree, and agreeing \
             twice is still once",
        );
        assert_eq!(joined.nodes_regenerated, ["."], "and nothing else is lost");
    }

    /// The other half of the same law: a prune the run never received still
    /// reaches the report.
    ///
    /// This is the failure a "read `pruned` from the run alone" join produces —
    /// a removal that really happened, reported as nothing — and it is exactly
    /// what happens on any path that returns before the transfer.
    #[test]
    fn a_prune_the_run_never_received_is_still_reported() {
        let mut measured = Measured::default();
        measured.record_pruned("vibedeps/org.demo.tools/0.1.0".into());
        let joined = measured.joined(InstallProgress::default());
        assert_eq!(joined.pruned, ["vibedeps/org.demo.tools/0.1.0"]);
    }

    /// A slot the later pass ALSO reported is not listed twice.
    #[test]
    fn a_slot_both_halves_know_about_appears_once() {
        let mut measured = Measured::default();
        measured.record_in_place("vibedeps/org.demo.later/1.0.0".into(), true);
        let (_, joined) = slots(&measured);
        assert_eq!(joined.materialised, ["vibedeps/org.demo.later/1.0.0"]);
    }

    #[test]
    fn sorting_is_the_completed_shape_and_removal_order_is_the_partial_one() {
        let mut measured = Measured::default();
        measured.record_pruned("vibedeps/b/1.0.0".into());
        measured.record_pruned("vibedeps/a/1.0.0".into());
        assert_eq!(
            measured.pruned(),
            ["vibedeps/b/1.0.0", "vibedeps/a/1.0.0"],
            "a partial list keeps the order things really happened in",
        );
        measured.sort_pruned();
        assert_eq!(measured.pruned(), ["vibedeps/a/1.0.0", "vibedeps/b/1.0.0"]);
    }
}
