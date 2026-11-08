//! Shared projection for the live event and the metadata-only preview.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY");
use vibe_registry::source_candidates::{
    SourceCandidateDecision as Decision, SourceCandidateFailure as Failure,
    SourceCandidateObservation, SourceCandidateObservationKind as Observation,
};
use vibe_wire::generated::install::e2::plan::{
    CandidateDecision, CandidateFailure, CandidateObservation, SourceCandidate,
};

pub(crate) fn candidate_rows(rows: &[SourceCandidateObservation]) -> Vec<SourceCandidate> {
    rows.iter()
        .map(|row| {
            let (observation, versions, failure) = match &row.observation {
                Observation::Unknown => (CandidateObservation::UnknownObservation, None, None),
                Observation::NoMatchingVersion { .. } => {
                    (CandidateObservation::NoMatchingVersion, None, None)
                }
                Observation::NotProbed => (CandidateObservation::NotProbed, None, None),
                Observation::Available { versions } => (
                    CandidateObservation::Available,
                    Some(versions.iter().map(ToString::to_string).collect()),
                    None,
                ),
                Observation::NotFound => (CandidateObservation::NotFound, None, None),
                Observation::Unavailable { reason } => (
                    CandidateObservation::Unavailable,
                    None,
                    Some(match reason {
                        Failure::Trust => CandidateFailure::Trust,
                        Failure::SourceUnavailable => CandidateFailure::SourceUnavailable,
                        Failure::Transport => CandidateFailure::Transport,
                        Failure::Authentication => CandidateFailure::Authentication,
                        Failure::InvalidMetadata => CandidateFailure::InvalidMetadata,
                        Failure::Unsupported => CandidateFailure::Unsupported,
                    }),
                ),
            };
            let (decision, pinned_registry, excluded_endpoint) = match &row.decision {
                Decision::Eligible => (CandidateDecision::Eligible, None, None),
                Decision::Selected => (CandidateDecision::Selected, None, None),
                Decision::ExcludedByPin { registry } => (
                    CandidateDecision::ExcludedByPin,
                    Some(registry.clone()),
                    None,
                ),
                Decision::ExcludedHost { endpoint } => (
                    CandidateDecision::ExcludedHost,
                    None,
                    Some(vibe_core::progress::transfer::safe_remote(endpoint)),
                ),
                Decision::Disabled => (CandidateDecision::Disabled, None, None),
                Decision::VersionRejected => (CandidateDecision::VersionRejected, None, None),
                Decision::LowerPreference => (CandidateDecision::LowerPreference, None, None),
                Decision::NotSelected => (CandidateDecision::NotSelected, None, None),
            };
            SourceCandidate {
                source_evidence: vibe_registry::binding_source_evidence(&row.binding),
                observation,
                decision,
                versions,
                constraint: match &row.observation {
                    Observation::NoMatchingVersion { constraint } => Some(constraint.clone()),
                    _ => None,
                },
                failure,
                pinned_registry,
                excluded_endpoint,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY-LAW")]
    fn preview_projection_keeps_unknown_constraint_and_failure_distinctions() {
        let observations = [
            Observation::Unknown,
            Observation::NoMatchingVersion {
                constraint: "^2".into(),
            },
            Observation::Unavailable {
                reason: Failure::Trust,
            },
            Observation::Unavailable {
                reason: Failure::SourceUnavailable,
            },
        ];
        let input = observations
            .into_iter()
            .map(|observation| SourceCandidateObservation {
                binding: vibe_core::manifest::CanonicalSourceBinding::DirectoryRegistry {
                    root: "fixture".into(),
                },
                observation,
                decision: Decision::NotSelected,
            })
            .collect::<Vec<_>>();
        let rows = candidate_rows(&input);
        assert!(matches!(
            rows[0].observation,
            CandidateObservation::UnknownObservation
        ));
        assert!(matches!(
            rows[1].observation,
            CandidateObservation::NoMatchingVersion
        ));
        assert_eq!(rows[1].constraint.as_deref(), Some("^2"));
        assert!(matches!(rows[2].failure, Some(CandidateFailure::Trust)));
        assert!(matches!(
            rows[3].failure,
            Some(CandidateFailure::SourceUnavailable)
        ));
        assert!(
            rows.iter()
                .all(|row| matches!(row.decision, CandidateDecision::NotSelected))
        );
        assert!(rows.iter().all(|row| row.versions.is_none()));
    }
}
