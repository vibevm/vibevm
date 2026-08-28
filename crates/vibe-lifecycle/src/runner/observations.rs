//! The invocation-local artifact halves: what this run SAW, and which durable
//! row it saw it against.
//!
//! Durable `StateArtifact.witness` is a **baseline** — what the execution that
//! produced or accepted an artifact saw at the moment it did. The observation
//! map is what THIS invocation sees now. Verify compares the two, so they must
//! stay two.
//!
//! The distinction is the whole correctness of E5. A fresh skip observes the
//! current object but produced nothing, so it may neither overwrite W1 with W2
//! nor upgrade a legacy unwitnessed row into a baseline: either move makes an
//! externally mutated output compare against itself and report `matched`
//! forever. Keeping the current reading here — and only here — is what makes
//! that impossible rather than merely discouraged.
//!
//! The baseline map answers the OTHER question, and it exists because the
//! phase plan cannot: an artifact accumulated by an install-stage slot
//! execution belongs to no row of `RitualPlan.executions`, and hunting for its
//! owner by scanning state would have to guess between a live row and a stale
//! one carrying the same id. So each durable-checkpoint site records the exact
//! row it just wrote or preserved, AFTER the write proved durable. Park,
//! dispatch failure and the state-blind runner record nothing, because none of
//! them made a durable row to copy.
//!
//! Ids key both maps because they are already unique across a run's
//! accumulated artifacts; `validate_shape` refuses a duplicate before any of
//! this runs.

use vibe_wire::generated::lifecycle_state::StateArtifact;

use crate::LifecycleRun;
use crate::artifacts::observe::{ArtifactObserver, WitnessOutcome};

impl LifecycleRun {
    /// What this invocation physically observed at each accumulated
    /// artifact's path — the CURRENT half of the evidence comparison, whose
    /// baseline half lives in durable state.
    ///
    /// Present for every artifact any execution of this invocation produced,
    /// accepted or fresh-skipped past, and re-taken at the verify instant by
    /// the reconciliation, including the ones whose observation was refused: a
    /// typed refusal is an answer about the current object, and the reconciler
    /// needs it to say `unstable` rather than inventing a reason.
    ///
    /// This IS the production read: the reconciler observes through
    /// [`observe_artifact`](Self::observe_artifact) and then takes the outcome
    /// back out of this map, so what it compares is exactly what the map
    /// holds — never a private copy that could drift from it.
    #[must_use]
    pub(crate) fn artifact_observation(&self, id: &str) -> Option<&WitnessOutcome> {
        self.artifact_observations.get(id)
    }

    /// The durable row this invocation checkpointed or preserved for one
    /// accumulated artifact — the PRIOR half, by exact id. `None` means no
    /// execution of this run owns that id, which is honest `unavailable`
    /// rather than an invitation to go looking through old records.
    #[must_use]
    pub(super) fn artifact_baseline(&self, id: &str) -> Option<&StateArtifact> {
        self.artifact_baselines.get(id)
    }

    /// Remember the exact rows a durable checkpoint just made current.
    ///
    /// Called ONLY after the state write returned `Ok`: a baseline that
    /// appeared before its checkpoint would be a claim about a row the state
    /// file does not hold, and a crash between the two would leave the
    /// comparison naming a measurement nobody could find.
    pub(super) fn remember_baselines(&mut self, artifacts: &[StateArtifact]) {
        for artifact in artifacts {
            self.artifact_baselines
                .insert(artifact.id.clone(), artifact.clone());
        }
    }

    /// Observe one artifact for this invocation and remember what was seen.
    /// One physical observation, two carriers: the returned outcome may become
    /// a durable pair at a production boundary, and the clone kept here is the
    /// current reading verify will compare against the baseline.
    pub(super) fn observe_artifact(
        &mut self,
        observer: &ArtifactObserver,
        id: &str,
        path: &str,
    ) -> WitnessOutcome {
        let outcome = observer.observe(id, path);
        self.artifact_observations
            .insert(id.to_string(), outcome.clone());
        outcome
    }
}
