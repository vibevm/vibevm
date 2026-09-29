//! Four explicitly participating observations, without a global read claim.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-COMPOSITE-SNAPSHOT");

use vibe_core::machine_view::OpaqueText;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotAxis {
    Source,
    World,
    Target,
    Operation,
}
impl SnapshotAxis {
    pub const ALL: [Self; 4] = [Self::Source, Self::World, Self::Target, Self::Operation];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisFreshness {
    DeclaredCurrent,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AxisCompleteness {
    DeclaredComplete,
    Partial { reason: OpaqueText },
    Unavailable { reason: OpaqueText },
}

/// A caller-stated exact join to another participating owner's revision.
/// Comparing it detects contradiction, not authenticity of either witness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionExpectation {
    pub axis: SnapshotAxis,
    pub revision: OpaqueText,
}

/// Each owner supplies its own identity; equal compatibility claims are still
/// claims, not proof that four reads happened atomically.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxisWitness {
    pub owner: OpaqueText,
    pub capture: OpaqueText,
    pub revision: OpaqueText,
    pub observation: OpaqueText,
    pub compatibility_claim: Option<OpaqueText>,
    pub expected_revisions: Vec<RevisionExpectation>,
    pub freshness: AxisFreshness,
    pub completeness: AxisCompleteness,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AxisParticipation {
    NotParticipating,
    Required(Option<AxisWitness>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompositeIssue {
    NoParticipatingAxis,
    Missing(SnapshotAxis),
    Stale(SnapshotAxis),
    ConflictingClaim(SnapshotAxis),
    ConflictingRevision {
        claimant: SnapshotAxis,
        target: SnapshotAxis,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositeCompleteness {
    DeclaredCompleteSingleOwner,
    Partial,
    UnverifiedCrossOwner,
    UnverifiedMissingClaim,
}

/// A typed key component for one owner's source/capture/revision/projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxisStamp {
    pub axis: SnapshotAxis,
    pub owner: OpaqueText,
    pub capture: OpaqueText,
    pub revision: OpaqueText,
    pub observation: OpaqueText,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositeSnapshot {
    axes: [AxisParticipation; 4],
    stamps: Vec<AxisStamp>,
    completeness: CompositeCompleteness,
}
impl CompositeSnapshot {
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-COMPOSITE-SNAPSHOT",
        r = 1
    )]
    pub fn compose(axes: [AxisParticipation; 4]) -> Result<Self, CompositeIssue> {
        let mut stamps = Vec::new();
        let mut compatibility: Option<&OpaqueText> = None;
        let mut missing_claim = false;
        let mut partial = false;
        for (axis, slot) in SnapshotAxis::ALL.into_iter().zip(&axes) {
            let witness = match slot {
                AxisParticipation::NotParticipating => continue,
                AxisParticipation::Required(None) => return Err(CompositeIssue::Missing(axis)),
                AxisParticipation::Required(Some(witness)) => witness,
            };
            if witness.freshness == AxisFreshness::Stale {
                return Err(CompositeIssue::Stale(axis));
            }
            if let Some(claim) = &witness.compatibility_claim {
                if compatibility.is_some_and(|earlier| earlier != claim) {
                    return Err(CompositeIssue::ConflictingClaim(axis));
                }
                compatibility = Some(claim);
            } else {
                missing_claim = true;
            }
            partial |= witness.completeness != AxisCompleteness::DeclaredComplete;
            stamps.push(AxisStamp {
                axis,
                owner: witness.owner.clone(),
                capture: witness.capture.clone(),
                revision: witness.revision.clone(),
                observation: witness.observation.clone(),
            });
        }
        if stamps.is_empty() {
            return Err(CompositeIssue::NoParticipatingAxis);
        }
        for (claimant, slot) in SnapshotAxis::ALL.into_iter().zip(&axes) {
            let AxisParticipation::Required(Some(witness)) = slot else {
                continue;
            };
            for expected in &witness.expected_revisions {
                let target_slot = match expected.axis {
                    SnapshotAxis::Source => &axes[0],
                    SnapshotAxis::World => &axes[1],
                    SnapshotAxis::Target => &axes[2],
                    SnapshotAxis::Operation => &axes[3],
                };
                let AxisParticipation::Required(Some(actual)) = target_slot else {
                    return Err(CompositeIssue::Missing(expected.axis));
                };
                if actual.revision != expected.revision {
                    return Err(CompositeIssue::ConflictingRevision {
                        claimant,
                        target: expected.axis,
                    });
                }
            }
        }
        let completeness = if partial {
            CompositeCompleteness::Partial
        } else if stamps.len() > 1 && missing_claim {
            CompositeCompleteness::UnverifiedMissingClaim
        } else if stamps.len() > 1 {
            CompositeCompleteness::UnverifiedCrossOwner
        } else {
            CompositeCompleteness::DeclaredCompleteSingleOwner
        };
        Ok(Self {
            axes,
            stamps,
            completeness,
        })
    }

    pub fn axis(&self, axis: SnapshotAxis) -> &AxisParticipation {
        let index = match axis {
            SnapshotAxis::Source => 0,
            SnapshotAxis::World => 1,
            SnapshotAxis::Target => 2,
            SnapshotAxis::Operation => 3,
        };
        &self.axes[index]
    }
    pub fn stamps(&self) -> &[AxisStamp] {
        &self.stamps
    }
    pub fn completeness(&self) -> CompositeCompleteness {
        self.completeness
    }
}
