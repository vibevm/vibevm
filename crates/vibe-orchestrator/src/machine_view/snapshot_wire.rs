//! Replay a generated composite claim through the existing pure composer.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-COMPOSITE-SNAPSHOT");

use vibe_core::machine_view::OpaqueText;
use vibe_wire::generated::machine::e1::composite_snapshot as wire;

use super::{
    AxisCompleteness, AxisFreshness, AxisParticipation, AxisWitness, CompositeCompleteness,
    CompositeIssue, CompositeSnapshot, RevisionExpectation, SnapshotAxis,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompositeReplay {
    Composed(CompositeSnapshot),
    Refused(CompositeIssue),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositeWireFault {
    UnsupportedEpoch,
    BlankReference,
    OutcomeMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositeWireFailure {
    pub original: wire::CompositeSnapshot,
    pub fault: CompositeWireFault,
}

fn text(value: &str) -> Result<OpaqueText, CompositeWireFault> {
    OpaqueText::new(value).map_err(|_| CompositeWireFault::BlankReference)
}
fn axis(value: &wire::Axis) -> SnapshotAxis {
    match value {
        wire::Axis::Source => SnapshotAxis::Source,
        wire::Axis::World => SnapshotAxis::World,
        wire::Axis::Target => SnapshotAxis::Target,
        wire::Axis::Operation => SnapshotAxis::Operation,
    }
}
fn slot(value: &wire::Participation) -> Result<AxisParticipation, CompositeWireFault> {
    let wire::Participation::Required(required) = value else {
        return Ok(AxisParticipation::NotParticipating);
    };
    let Some(witness) = &required.witness else {
        return Ok(AxisParticipation::Required(None));
    };
    let completeness = match &witness.completeness {
        wire::Completeness::Complete(_) => AxisCompleteness::DeclaredComplete,
        wire::Completeness::Partial(row) => AxisCompleteness::Partial {
            reason: text(&row.reason)?,
        },
        wire::Completeness::Unavailable(row) => AxisCompleteness::Unavailable {
            reason: text(&row.reason)?,
        },
    };
    Ok(AxisParticipation::Required(Some(AxisWitness {
        owner: text(&witness.owner)?,
        capture: text(&witness.capture)?,
        revision: text(&witness.revision)?,
        observation: text(&witness.observation)?,
        compatibility_claim: witness
            .compatibility_claim
            .as_deref()
            .map(text)
            .transpose()?,
        expected_revisions: witness
            .expected_revisions
            .iter()
            .map(|row| {
                Ok(RevisionExpectation {
                    axis: axis(&row.axis),
                    revision: text(&row.revision)?,
                })
            })
            .collect::<Result<_, CompositeWireFault>>()?,
        freshness: match witness.freshness {
            wire::WitnessFreshness::Current => AxisFreshness::DeclaredCurrent,
            wire::WitnessFreshness::Stale => AxisFreshness::Stale,
        },
        completeness,
    })))
}
fn completeness(value: &wire::OutcomeComposedCompleteness) -> CompositeCompleteness {
    match value {
        wire::OutcomeComposedCompleteness::DeclaredCompleteSingleOwner => {
            CompositeCompleteness::DeclaredCompleteSingleOwner
        }
        wire::OutcomeComposedCompleteness::Partial => CompositeCompleteness::Partial,
        wire::OutcomeComposedCompleteness::UnverifiedCrossOwner => {
            CompositeCompleteness::UnverifiedCrossOwner
        }
        wire::OutcomeComposedCompleteness::UnverifiedMissingClaim => {
            CompositeCompleteness::UnverifiedMissingClaim
        }
    }
}
fn issue(value: &wire::Issue) -> CompositeIssue {
    match value {
        wire::Issue::NoParticipatingAxis(_) => CompositeIssue::NoParticipatingAxis,
        wire::Issue::Missing(row) => CompositeIssue::Missing(axis(&row.axis)),
        wire::Issue::Stale(row) => CompositeIssue::Stale(axis(&row.axis)),
        wire::Issue::ConflictingClaim(row) => CompositeIssue::ConflictingClaim(axis(&row.axis)),
        wire::Issue::ConflictingRevision(row) => CompositeIssue::ConflictingRevision {
            claimant: axis(&row.claimant),
            target: axis(&row.target),
        },
    }
}

/// The generated document is untrusted until its stated outcome is replayed.
/// This checks supplied values only; no owner byte or atomicity proof appears.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-COMPOSITE-SNAPSHOT",
    r = 1
)]
pub fn replay_generated_composite(
    original: wire::CompositeSnapshot,
) -> Result<CompositeReplay, CompositeWireFailure> {
    let check = || -> Result<CompositeReplay, CompositeWireFault> {
        if original.schema != 1 {
            return Err(CompositeWireFault::UnsupportedEpoch);
        }
        let slots = [
            slot(&original.source)?,
            slot(&original.world)?,
            slot(&original.target)?,
            slot(&original.operation)?,
        ];
        match (CompositeSnapshot::compose(slots), &original.outcome) {
            (Ok(snapshot), wire::Outcome::Composed(row)) => {
                if snapshot.completeness() != completeness(&row.completeness)
                    || snapshot.stamps().len() != row.stamps.len()
                {
                    return Err(CompositeWireFault::OutcomeMismatch);
                }
                for (actual, declared) in snapshot.stamps().iter().zip(&row.stamps) {
                    if actual.axis != axis(&declared.axis)
                        || actual.owner.as_str() != declared.owner
                        || actual.capture.as_str() != declared.capture
                        || actual.revision.as_str() != declared.revision
                        || actual.observation.as_str() != declared.observation
                    {
                        return Err(CompositeWireFault::OutcomeMismatch);
                    }
                }
                Ok(CompositeReplay::Composed(snapshot))
            }
            (Err(actual), wire::Outcome::Refused(row)) if actual == issue(&row.issue) => {
                Ok(CompositeReplay::Refused(actual))
            }
            _ => Err(CompositeWireFault::OutcomeMismatch),
        }
    };
    check().map_err(|fault| CompositeWireFailure { original, fault })
}
