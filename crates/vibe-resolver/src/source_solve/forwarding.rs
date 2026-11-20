//! Compulsory one-hop forwarding preconditions, before the single existing solve.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#EFFECT-CONSUMERS");
use super::*;
use crate::source_candidates::{ActiveSourceEdge, EdgeOwner};
use vibe_core::manifest::{
    ConfigurationEffect, ForwardStrength, ForwardedInput, PackageConfiguration,
};
use vibe_registry::source_candidates::{
    CandidateInventoryScope, RetainedSourceCandidate, SourceInventoryLimits,
};

pub(super) struct Preseed {
    pub inputs: BTreeMap<String, Vec<ForwardedInput>>,
    pairs: Vec<(CandidateKey, CandidateKey, usize, vibe_core::ContentHash)>,
}
fn ensure_complete_inventory(
    observations: &[vibe_registry::source_candidates::SourceCandidateObservation],
) -> Result<(), SolveError> {
    use vibe_registry::source_candidates::SourceCandidateObservationKind as Observation;
    if observations.is_empty()
        || observations.iter().any(|row| {
            matches!(
                row.observation,
                Observation::Unknown | Observation::NotProbed | Observation::Unavailable { .. }
            )
        })
    {
        return Err(invalid(
            "forwarding uniqueness requires complete admitted source observations",
        ));
    }
    Ok(())
}

fn unique<P: SourceCandidateProvider>(
    provider: &P,
    reference: &PackageRef,
    request: &SourceSolveRequest<'_>,
) -> Result<(RetainedSourceCandidate, vibe_core::ContentHash), SolveError> {
    if !reference.version.is_exact_pin() {
        return Err(invalid(
            "forwarding requires authored exact versions; ranges are not narrowed",
        ));
    }
    let (group, name) = reference_coordinate(reference)?;
    let inventory = provider.source_inventory(
        &group,
        &name,
        CandidateInventoryScope::AllAllowed,
        &SourceInventoryLimits {
            max_candidates: request.limits.source_candidates,
        },
    )?;
    ensure_complete_inventory(&inventory.observations)?;
    let mut matches = inventory
        .candidates
        .into_iter()
        .filter(|record| reference.version.matches(record.candidate.version()));
    let record = matches
        .next()
        .ok_or_else(|| invalid("forwarding exact candidate unavailable"))?;
    if matches.next().is_some() || !record.default_eligible {
        return Err(invalid("forwarding source candidate is ambiguous"));
    }
    let pin = record
        .candidate
        .observe_local_content_pin()
        .map_err(crate::source_candidates::source_registry_error)?;
    Ok((record.candidate, pin))
}
pub(super) fn prepare<P: SourceCandidateProvider>(
    provider: &P,
    request: &SourceSolveRequest<'_>,
) -> Result<Option<Preseed>, SolveError> {
    let Some(snapshot) = request.configuration else {
        return Ok(None);
    };
    let mut result = Preseed {
        inputs: BTreeMap::new(),
        pairs: Vec::new(),
    };
    for root in request.roots {
        // Observe the incumbent metadata without presuming discovery is activation.
        if root.requirement.alternatives.len() != 1 {
            continue;
        }
        let reference = &root.requirement.alternatives[0].reference;
        let (group, name) = reference_coordinate(reference)?;
        let version = provider.resolve_version(reference)?;
        let observed = provider
            .retain_default_candidate(&group, &name, &version)?
            .candidate;
        let Some(definition) = &observed.manifest().configuration else {
            continue;
        };
        if !definition.features.iter().any(|feature| {
            feature
                .effects
                .iter()
                .any(|effect| matches!(effect, ConfigurationEffect::Forward { .. }))
        }) {
            continue;
        }
        let inputs = snapshot
            .authored_inputs(&reference.qualified_name())
            .ok_or_else(|| invalid("forwarding parent lacks complete original workspace inputs"))?;
        let parent_record = PackageConfiguration::evaluate(observed.manifest(), &inputs)
            .map_err(DepProviderError::Configuration)?;
        let selected = definition
            .features
            .iter()
            .filter(|feature| parent_record.features.contains(&feature.id))
            .flat_map(|feature| {
                feature
                    .effects
                    .iter()
                    .filter_map(move |effect| match effect {
                        ConfigurationEffect::Forward {
                            target,
                            feature: target_feature,
                            strength,
                        } => Some((
                            feature.id.clone(),
                            target.clone(),
                            target_feature.clone(),
                            *strength,
                        )),
                        _ => None,
                    })
            })
            .collect::<Vec<_>>();
        if selected.is_empty() {
            continue;
        }
        if !matches!(root.id.owner, EdgeOwner::Root { .. })
            || !matches!(
                root.requirement.activation,
                crate::source_edges::RequirementActivation::Mandatory
            )
            || root.requirement.alternatives[0].registry_pin.is_some()
            || !request.activated.is_empty()
            || !request.blocked.is_empty()
        {
            return Err(invalid(
                "forwarding parent is not a compulsory plain workspace root",
            ));
        }
        let (parent, parent_pin) = unique(provider, reference, request)?;
        if parent.metadata_digest() != observed.metadata_digest()
            || !parent.same_selected_source(&observed)
        {
            return Err(invalid(
                "forwarding parent source changed during observation",
            ));
        }
        if !parent.manifest().recommends.packages.is_empty()
            || !parent.manifest().conditional_deps.is_empty()
        {
            return Err(invalid(
                "forwarding does not silently suppress recommendation or conditional planning",
            ));
        }
        for (source_feature, target, target_feature, strength) in selected {
            let index = parent_record
                .declarations
                .packages
                .iter()
                .position(|edge| edge.qualified_name() == target)
                .ok_or_else(|| invalid("forwarded declaration missing"))?;
            let edge = &parent_record.declarations.packages[index];
            if strength == ForwardStrength::Weak
                && !parent_record.active_dependencies.contains(&target)
            {
                // A dormant request grants no target observation or feature
                // validation, even if another workspace root selects it.
                continue;
            }
            let (child, child_pin) = unique(provider, edge, request)?;
            if child.group() == parent.group() && child.name() == parent.name() {
                return Err(invalid("cyclic forwarding is not connected"));
            }
            if child
                .manifest()
                .configuration
                .as_ref()
                .is_none_or(|definition| {
                    definition.features.iter().any(|feature| {
                        feature
                            .effects
                            .iter()
                            .any(|effect| matches!(effect, ConfigurationEffect::Forward { .. }))
                    })
                })
            {
                return Err(invalid("forwarded target is missing or recursive"));
            }
            let key = edge.qualified_name();
            if result.inputs.contains_key(&key) {
                return Err(invalid(
                    "multiple forwarded package consumers are not connected",
                ));
            }
            result.inputs.insert(
                key,
                vec![ForwardedInput {
                    source_group: parent.group().clone(),
                    source_name: parent
                        .name()
                        .parse()
                        .map_err(|_| invalid("invalid source name"))?,
                    source_version: parent.version().clone(),
                    source_content_hash: parent_pin.clone(),
                    source_feature,
                    declaration_index: index,
                    target: edge.clone(),
                    target_feature,
                    strength,
                }],
            );
            result.pairs.push((
                CandidateKey::of(&parent),
                CandidateKey::of(&child),
                index,
                child_pin,
            ));
        }
    }
    Ok((!result.pairs.is_empty()).then_some(result))
}
impl Preseed {
    pub fn validate(&self, solved: &SourceResolvedGraph) -> Result<(), SolveError> {
        if solved.selected.values().any(|metadata| {
            !metadata.view.recommends.packages.is_empty()
                || !metadata.view.conditional_deps.is_empty()
        }) {
            return Err(invalid(
                "forwarding first slice cannot suppress selected-world recommendations or conditional inputs",
            ));
        }
        for (parent, target, index, target_pin) in &self.pairs {
            let incoming = solved
                .active_edges
                .iter()
                .filter(|edge| {
                    edge.target == *target && matches!(edge.id.owner, EdgeOwner::Candidate(_))
                })
                .collect::<Vec<&ActiveSourceEdge>>();
            if incoming.len() != 1
                || !matches!(&incoming[0].id.owner,EdgeOwner::Candidate(owner) if **owner==*parent)
                || incoming[0].id.site != crate::source_edges::RequirementSite::Requires(*index)
            {
                return Err(invalid(
                    "selected forwarding relation differs from compulsory preseed",
                ));
            }
            let parent_metadata = solved
                .selected
                .get(&(parent.group.clone(), parent.name.clone()))
                .ok_or_else(|| invalid("compulsory parent missing"))?;
            if parent_metadata.key() != *parent {
                return Err(invalid("compulsory parent candidate changed"));
            }
            let predicted = self
                .inputs
                .get(&format!("{}/{}", target.group, target.name))
                .ok_or_else(|| invalid("preseed input missing"))?;
            if predicted.iter().any(|input| {
                Some(&input.source_content_hash) != parent_metadata.forwarding_content_pin.as_ref()
            }) {
                return Err(invalid("compulsory parent content changed before solve"));
            }
            let target_metadata = solved
                .selected
                .get(&(target.group.clone(), target.name.clone()))
                .ok_or_else(|| invalid("compulsory target missing"))?;
            if target_metadata.key() != *target
                || target_metadata.forwarding_content_pin.as_ref() != Some(target_pin)
            {
                return Err(invalid("compulsory target content changed before solve"));
            }
            let actual = solved
                .selected
                .get(&(target.group.clone(), target.name.clone()))
                .and_then(|metadata| metadata.configuration.as_ref())
                .ok_or_else(|| invalid("forwarded target result missing"))?;
            if self
                .inputs
                .get(&format!("{}/{}", target.group, target.name))
                != Some(&actual.forwarded_inputs)
            {
                return Err(invalid("forwarded target input set changed"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibe_registry::source_candidates::{
        SourceCandidateDecision, SourceCandidateFailure, SourceCandidateObservation,
        SourceCandidateObservationKind as Observation,
    };
    fn row(observation: Observation) -> SourceCandidateObservation {
        SourceCandidateObservation {
            binding: vibe_core::manifest::CanonicalSourceBinding::DirectoryRegistry {
                root: "/synthetic-registry".into(),
            },
            observation,
            decision: SourceCandidateDecision::Eligible,
        }
    }
    #[test]
    fn one_observed_candidate_never_proves_uniqueness_with_incomplete_sources() {
        let positive = row(Observation::Available {
            versions: vec![semver::Version::new(1, 0, 0)],
        });
        assert!(ensure_complete_inventory(std::slice::from_ref(&positive)).is_ok());
        for absent in [
            Observation::Unknown,
            Observation::NotProbed,
            Observation::Unavailable {
                reason: SourceCandidateFailure::SourceUnavailable,
            },
        ] {
            assert!(ensure_complete_inventory(&[positive.clone(), row(absent)]).is_err());
        }
        assert!(ensure_complete_inventory(&[]).is_err());
    }
}

#[cfg(test)]
mod content_tests {
    use super::*;
    use crate::source_candidates::{EdgeId, EdgeOwner};
    use crate::source_edges::{
        OriginalRequirement, RequirementActivation, RequirementAlternative, RequirementSite,
    };
    #[test]
    fn same_target_manifest_and_binding_cannot_replace_early_content_pin() {
        for changed in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let parent = root.path().join("org.test/parent/v1.0.0");
            let target = root.path().join("org.test/target/v1.0.0");
            std::fs::create_dir_all(&parent).unwrap();
            std::fs::create_dir_all(&target).unwrap();
            std::fs::write(parent.join("vibe.toml"),"[package]\ngroup='org.test'\nname='parent'\nversion='1.0.0'\nkind='flow'\n[requires.packages]\n'org.test/target'='=1.0.0'\n[[configuration.features]]\nid='go'\nname='Go'\neffects=[{kind='forward',target='org.test/target',feature='answer',strength='strong'}]").unwrap();
            std::fs::write(target.join("vibe.toml"),"[package]\ngroup='org.test'\nname='target'\nversion='1.0.0'\nkind='flow'\n[configuration.config_values]\nanswer=true\n[[configuration.features]]\nid='answer'\nname='Answer'\neffects=[{kind='config-value',name='answer'}]").unwrap();
            std::fs::write(target.join("payload.txt"), "original").unwrap();
            let registry = vibe_registry::LocalRegistry::new(root.path()).unwrap();
            let provider = crate::LocalRegistryDepProvider::new(&registry);
            let input: vibe_core::manifest::ConfigurationIntentDecl = toml::from_str(
                "policy='require-selection'\ndefault_features=false\nfeatures=['go']",
            )
            .unwrap();
            let snapshot = crate::configuration::ConfigurationSnapshot::new(BTreeMap::from([(
                "org.test/parent".into(),
                BTreeMap::from([("./vibe.toml".into(), Some(input))]),
            )]));
            let roots = vec![RootRequirement {
                id: EdgeId {
                    owner: EdgeOwner::Root {
                        consumer: "./vibe.toml".into(),
                    },
                    site: RequirementSite::Requires(0),
                },
                requirement: OriginalRequirement {
                    site: RequirementSite::Requires(0),
                    activation: RequirementActivation::Mandatory,
                    alternatives: vec![RequirementAlternative {
                        reference: "org.test/parent@=1.0.0".parse().unwrap(),
                        registry_pin: None,
                    }],
                },
            }];
            let blocked = BTreeSet::new();
            let request = SourceSolveRequest {
                selected_roots: &[],
                roots: &roots,
                held_roots: &[],
                authoritative_locked: &[],
                activated: &[],
                limits: SourceSolveLimits::standard_operation(),
                configuration: Some(&snapshot),
                blocked: &blocked,
                recommends: false,
            };
            let preseed = prepare(&provider, &request).unwrap().unwrap();
            if changed {
                std::fs::write(
                    target.join("payload.txt"),
                    "replacement without metadata change",
                )
                .unwrap();
            }
            let context = snapshot.with_forwarded_context(preseed.inputs.clone());
            let bound = SourceSolveRequest {
                configuration: Some(&context),
                ..request
            };
            let solved = super::super::solve_once(&provider, &bound, true).unwrap();
            assert_eq!(preseed.validate(&solved).is_ok(), !changed);
        }
    }
}
