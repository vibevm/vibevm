//! Decision annotations follow the validated selected relation, not inventory discovery.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#solver");
use super::*;
use crate::source_candidates::{ActiveSourceEdge, EdgeOwner};
#[cfg(test)]
use vibe_core::manifest::CanonicalSourceBinding;
use vibe_registry::source_candidates::{
    SourceCandidateDecision as Decision, SourceCandidateObservation,
    SourceCandidateObservationKind as Observation,
};

struct Rank {
    key: CandidateKey,
    eligible: bool,
    preference: usize,
    order: usize,
}

pub(super) fn annotate(
    selected: &mut BTreeMap<Coordinate, CandidateMetadata>,
    active: &[ActiveSourceEdge],
    request: &SourceSolveRequest<'_>,
    world: Option<&Universe>,
) -> Result<(), SolveError> {
    // Resolve every pin through its exact witnessed original occurrence. Historical
    // parents remain admissible only through the retained operation phase evidence.
    let mut instructions = Vec::new();
    for (coordinate, metadata) in selected.iter() {
        let key = metadata.key();
        let mut pins = BTreeSet::new();
        for witnessed in active.iter().filter(|edge| edge.target == key) {
            let requirement = match &witnessed.id.owner {
                EdgeOwner::Root { .. } => request
                    .roots
                    .iter()
                    .find(|root| root.id == witnessed.id)
                    .map(|root| &root.requirement),
                EdgeOwner::Candidate(parent) => request
                    .activated
                    .iter()
                    .find(|activation| {
                        activation.parent.key() == **parent
                            && activation.edge.site == witnessed.id.site
                            && activation.phase > 0
                            && activation.phase <= 5
                    })
                    .and_then(|activation| {
                        activation
                            .parent
                            .original_edges
                            .iter()
                            .find(|edge| edge.site == witnessed.id.site)
                    })
                    .or_else(|| {
                        selected
                            .values()
                            .find(|m| m.key() == **parent)
                            .and_then(|m| {
                                m.original_edges
                                    .iter()
                                    .find(|edge| edge.site == witnessed.id.site)
                            })
                    }),
            };
            // Capability and greedy recommendation witnesses carry no registry pin.
            if matches!(
                witnessed.id.site,
                crate::source_edges::RequirementSite::Capability(_)
                    | crate::source_edges::RequirementSite::Recommendation(_)
            ) {
                continue;
            }
            let choice = requirement
                .and_then(|edge| edge.alternatives.get(witnessed.alternative))
                .ok_or_else(|| invalid("selected source decision lost original edge authority"))?;
            if choice.reference.group.as_ref() != Some(&key.group)
                || choice.reference.name != key.name
                || !choice.reference.version.matches(&key.version)
            {
                return Err(invalid(
                    "selected source decision disagrees with original edge target",
                ));
            }
            if let Some(pin) = &choice.registry_pin {
                if key.binding.registry_name() != Some(pin.as_str()) {
                    return Err(invalid(
                        "selected source decision contradicts witnessed registry pin",
                    ));
                }
                pins.insert(pin.as_str().to_owned());
            }
        }
        if pins.len() > 1 {
            return Err(invalid(
                "selected source decision has conflicting active pin authorities",
            ));
        }
        instructions.push((coordinate.clone(), key, pins.into_iter().next()));
    }
    let ranks = world
        .map(|world| {
            world
                .entries
                .iter()
                .map(|entry| Rank {
                    key: entry.metadata.key(),
                    eligible: entry.default_eligible,
                    preference: entry.preference_rank,
                    order: entry.registry_order,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for (coordinate, key, pin) in instructions {
        let metadata = selected
            .get_mut(&coordinate)
            .ok_or_else(|| invalid("selected source decision owner disappeared"))?;
        annotate_rows(&mut metadata.observations, &key, pin.as_deref(), &ranks);
    }
    Ok(())
}

fn annotate_rows(
    rows: &mut [SourceCandidateObservation],
    selected: &CandidateKey,
    pin: Option<&str>,
    ranks: &[Rank],
) {
    let chosen_rank = ranks.iter().find(|rank| rank.key == *selected);
    for row in rows {
        // Rows may repeat one binding after different version probes. Only an
        // observed version witness receives Selected; failure/unknown rows keep
        // their observation and are not relabelled as a successful attempt.
        let selected_version = matches!(&row.observation,Observation::Available{versions} if versions.contains(&selected.version));
        row.decision = if matches!(
            row.decision,
            Decision::ExcludedHost { .. } | Decision::Disabled | Decision::VersionRejected
        ) {
            row.decision.clone()
        } else if row.binding == selected.binding && selected_version {
            Decision::Selected
        } else if let Some(registry) =
            pin.filter(|registry| row.binding.registry_name() != Some(*registry))
        {
            Decision::ExcludedByPin {
                registry: registry.into(),
            }
        } else if matches!(row.observation, Observation::Available { .. })
            && chosen_rank.is_some_and(|chosen| {
                ranks.iter().any(|other| {
                    other.key.group == selected.group
                        && other.key.name == selected.name
                        && other.key.version == selected.version
                        && other.key.binding == row.binding
                        && other.eligible
                        && (other.preference, other.order) > (chosen.preference, chosen.order)
                })
            })
        {
            Decision::LowerPreference
        } else {
            Decision::NotSelected
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn binding(name: &str) -> CanonicalSourceBinding {
        CanonicalSourceBinding::Registry {
            registry: vibe_core::manifest::RegistryPin::new(name.into()).unwrap(),
            source_url: vibe_core::SourceUrl::new(format!("https://{name}.example/registry")),
        }
    }
    fn key(name: &str) -> CandidateKey {
        CandidateKey {
            group: Group::parse("org.test").unwrap(),
            name: "runtime".into(),
            version: semver::Version::new(1, 0, 0),
            binding: binding(name),
        }
    }
    fn row(name: &str, observation: Observation) -> SourceCandidateObservation {
        SourceCandidateObservation {
            binding: binding(name),
            observation,
            decision: Decision::Eligible,
        }
    }
    fn available() -> Observation {
        Observation::Available {
            versions: vec![semver::Version::new(1, 0, 0)],
        }
    }
    fn rank(name: &str, value: usize) -> Rank {
        Rank {
            key: key(name),
            eligible: true,
            preference: value,
            order: 0,
        }
    }
    #[test]
    fn same_version_sources_keep_exact_selected_identity_and_observations() {
        let mut rows = vec![
            row("a", available()),
            row("b", available()),
            row(
                "b",
                Observation::Unavailable {
                    reason: vibe_registry::source_candidates::SourceCandidateFailure::Trust,
                },
            ),
        ];
        annotate_rows(&mut rows, &key("b"), None, &[rank("a", 1), rank("b", 0)]);
        assert_eq!(rows[0].decision, Decision::LowerPreference);
        assert_eq!(rows[1].decision, Decision::Selected);
        assert_eq!(rows[2].decision, Decision::NotSelected);
        assert!(matches!(
            rows[2].observation,
            Observation::Unavailable {
                reason: vibe_registry::source_candidates::SourceCandidateFailure::Trust
            }
        ));
    }
    #[test]
    fn inactive_pin_is_not_inferred_from_observation_or_selection() {
        let mut rows = vec![
            row("a", available()),
            row("b", Observation::NotProbed),
            row("c", Observation::Unknown),
            row(
                "d",
                Observation::NoMatchingVersion {
                    constraint: "^2".into(),
                },
            ),
        ];
        annotate_rows(&mut rows, &key("a"), None, &[]);
        assert_eq!(rows[1].decision, Decision::NotSelected);
        assert_eq!(rows[2].decision, Decision::NotSelected);
        assert!(matches!(rows[1].observation, Observation::NotProbed));
        assert!(matches!(rows[2].observation, Observation::Unknown));
        assert_eq!(rows[3].decision, Decision::NotSelected);
        assert!(
            matches!(&rows[3].observation,Observation::NoMatchingVersion{constraint} if constraint=="^2")
        );
    }
    #[test]
    fn lower_preference_requires_observed_same_version_eligible_candidate() {
        let mut rows = vec![
            row("a", available()),
            row("b", Observation::NotProbed),
            row("c", available()),
        ];
        let mut ineligible = rank("c", 2);
        ineligible.eligible = false;
        annotate_rows(
            &mut rows,
            &key("a"),
            None,
            &[rank("a", 0), rank("b", 1), ineligible],
        );
        assert_eq!(rows[1].decision, Decision::NotSelected);
        assert_eq!(rows[2].decision, Decision::NotSelected);
    }
    #[test]
    fn active_pin_changes_decision_without_rewriting_unknown_or_failed_observation() {
        let mut rows = vec![
            row("a", Observation::Unknown),
            row("b", available()),
            row(
                "c",
                Observation::Unavailable {
                    reason:
                        vibe_registry::source_candidates::SourceCandidateFailure::Authentication,
                },
            ),
        ];
        annotate_rows(&mut rows, &key("b"), Some("b"), &[]);
        assert_eq!(
            rows[0].decision,
            Decision::ExcludedByPin {
                registry: "b".into()
            }
        );
        assert_eq!(rows[1].decision, Decision::Selected);
        assert_eq!(
            rows[2].decision,
            Decision::ExcludedByPin {
                registry: "b".into()
            }
        );
        assert!(matches!(rows[0].observation, Observation::Unknown));
        assert!(matches!(
            rows[2].observation,
            Observation::Unavailable { .. }
        ));
    }
}
