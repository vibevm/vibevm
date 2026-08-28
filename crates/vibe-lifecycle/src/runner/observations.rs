//! The invocation-local artifact observations: the CURRENT half of the
//! evidence comparison.
//!
//! Durable `StateArtifact.witness` is a **baseline** — what the execution that
//! produced or accepted an artifact saw at the moment it did. This map is what
//! THIS invocation sees now. Verify compares the two, so they must stay two.
//!
//! The distinction is the whole correctness of E5. A fresh skip observes the
//! current object but produced nothing, so it may neither overwrite W1 with W2
//! nor upgrade a legacy unwitnessed row into a baseline: either move makes an
//! externally mutated output compare against itself and report `matched`
//! forever. Keeping the current reading here — and only here — is what makes
//! that impossible rather than merely discouraged.
//!
//! Ids key the map because they are already unique across a run's accumulated
//! artifacts; `validate_shape` refuses a duplicate before any of this runs.

use crate::LifecycleRun;
use crate::artifacts::observe::{ArtifactObserver, WitnessOutcome};

impl LifecycleRun {
    /// What this invocation physically observed at each accumulated
    /// artifact's path — the CURRENT half of the evidence comparison, whose
    /// baseline half lives in durable state.
    ///
    /// Present for every artifact any execution of this invocation produced,
    /// accepted or fresh-skipped past, including the ones whose observation
    /// was refused: a typed refusal is an answer about the current object, and
    /// A5 needs it to say `unstable` rather than inventing a reason.
    ///
    /// Read by this atom's REDs and by A5's reconciler, which is the only
    /// production consumer this half of the comparison can have — there is
    /// nothing for the runner itself to do with a current observation, and
    /// giving it one would be the very fold-into-the-baseline this map exists
    /// to prevent. The allow goes when A5 lands, not before.
    #[allow(dead_code, reason = "A5's reconciler is the production consumer")]
    #[must_use]
    pub(crate) fn artifact_observation(&self, id: &str) -> Option<&WitnessOutcome> {
        self.artifact_observations.get(id)
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
