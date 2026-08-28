//! The post-publication recovery window, split from `store.rs` when the
//! transaction half outgrew the 600-line budget (PROP-054
//! `##PHASE-STATE-HOME`, R7.4 architecture §2.2).
//!
//! A staged atomic replace has one boundary: the rename. Everything before
//! it is provably invisible; the rename itself and the verification after it
//! are not. So when a publication fails with `PossiblyPublished`, "the write
//! failed" is not a fact the store may assume — the bytes may already be the
//! candidate's. The recovery here resolves that doubt exactly once, through
//! the SAME pinned capability, by comparing the durable bytes against the two
//! byte strings the store can name: the exact candidate bytes it tried to
//! publish, and the exact prior bytes it already knows.
//!
//! Three outcomes, three behaviors:
//!
//! - the exact candidate bytes are visible → the candidate IS durable, so the
//!   store adopts it in memory (memory and disk agree) while still returning
//!   the typed post-publication diagnostic;
//! - the exact prior bytes — or the prior absence — are visible → the write
//!   in fact did not land, so the prior state and bytes stay current;
//! - anything else — a third state, a vanished file, an unreadable or unsafe
//!   shape — is a disk the store can no longer describe. It POISONS the
//!   store: every later mutation refuses before another write, and no retry
//!   or "healing" write is attempted, because writing over an unattributable
//!   state is how a concurrent writer's park is silently destroyed.

use specmark::spec;
use vibe_wire::generated::lifecycle_state::LifecycleState;

use super::error::{LifecycleStateError, PostPublicationRecovery};
use super::io::STATE_CAP;
use super::store::LifecycleStateStore;

impl LifecycleStateStore {
    /// Resolve one `PossiblyPublished` failure by re-reading the state file
    /// exactly once, bounded, through the pinned project. See the module doc
    /// for the outcome table; in every branch the ORIGINAL publication
    /// failure is preserved verbatim in the returned diagnostic.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#PHASE-STATE-HOME")]
    pub(super) fn recover_after_possibly_published(
        &mut self,
        candidate: LifecycleState,
        candidate_bytes: Vec<u8>,
        rendered_source: String,
    ) -> Result<(), LifecycleStateError> {
        let diagnostic = |recovery| LifecycleStateError::PostPublication {
            path: self.path.clone(),
            stage: vibe_safefs::PublishStage::PossiblyPublished,
            publication: rendered_source.clone(),
            recovery,
        };
        // The one bounded re-read. A refusal here (a link, a hard link, a
        // directory, an over-cap file) is the unsafe outcome: the file this
        // store is about to reason over cannot be proven to hold either byte
        // string, which is a third state by another name.
        let visible = self
            .lease
            .project()
            .read_file_bounded(Self::FILE, STATE_CAP)
            .map_err(|error| format!("the bounded re-read refused: {error:#}"));
        let outcome = match visible {
            Ok(Some(bytes)) if bytes == candidate_bytes => {
                self.state = candidate;
                self.durable = Some(candidate_bytes);
                return Err(diagnostic(PostPublicationRecovery::CandidateAdopted));
            }
            Ok(Some(bytes)) if self.durable.as_ref() == Some(&bytes) => {
                return Err(diagnostic(PostPublicationRecovery::PriorRetained));
            }
            Ok(None) if self.durable.is_none() => {
                return Err(diagnostic(PostPublicationRecovery::PriorRetained));
            }
            Ok(None) => "the state file vanished between the publication attempt and the re-read"
                .to_string(),
            Ok(Some(third)) => format!(
                "the durable bytes are a third state: {} bytes that are neither the {} candidate \
                 bytes nor {}",
                third.len(),
                candidate_bytes.len(),
                match &self.durable {
                    Some(prior) =>
                        format!("the {} prior bytes this store last proved", prior.len()),
                    // Absence is rendered as absence — never as a zero-length
                    // prior file, which would assert a file that never
                    // existed in the one diagnostic an operator uses to
                    // decide what to keep.
                    None => "any prior state (the file was absent before this write)".to_string(),
                },
            ),
            Err(reason) => reason,
        };
        self.poisoned.get_or_insert_with(|| outcome.clone());
        Err(diagnostic(PostPublicationRecovery::Poisoned {
            reason: outcome,
        }))
    }
}
