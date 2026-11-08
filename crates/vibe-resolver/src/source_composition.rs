//! Ordered source families retain their original version domain and source owner.
use crate::{DepProviderError, SourceCandidateProvider};
use std::collections::{BTreeMap, BTreeSet};
use vibe_core::Group;
use vibe_registry::source_candidates::*;
pub(crate) fn absent(error: &DepProviderError) -> bool {
    matches!(
        error,
        DepProviderError::UnknownPackage { .. }
            | DepProviderError::NoMatchingVersion { .. }
            | DepProviderError::AggregateNotFound { .. }
            | DepProviderError::OfflineUnavailable { .. }
    )
}
pub(crate) fn mode(providers: &[&dyn SourceCandidateProvider]) -> SourceChoiceMode {
    let mut result = SourceChoiceMode::LegacyKnown;
    for provider in providers {
        match provider.source_choice_mode() {
            SourceChoiceMode::CandidateRequired => return SourceChoiceMode::CandidateRequired,
            SourceChoiceMode::Deferred => result = SourceChoiceMode::Deferred,
            SourceChoiceMode::LegacyKnown => {}
        }
    }
    result
}
pub(crate) fn first<T>(
    providers: &[&dyn SourceCandidateProvider],
    group: &Group,
    name: &str,
    op: impl Fn(&dyn SourceCandidateProvider) -> Result<T, DepProviderError>,
) -> Result<T, DepProviderError> {
    if let Some(owner) = providers
        .iter()
        .find(|p| p.has_locked_coordinate(group, name))
    {
        return op(*owner);
    }
    let mut last = None;
    for provider in providers {
        match op(*provider) {
            Ok(value) => return Ok(value),
            Err(error) if absent(&error) => last = Some(error),
            Err(error) => return Err(error),
        }
    }
    Err(last.unwrap_or_else(|| DepProviderError::UnknownPackage {
        group: group.clone(),
        name: name.into(),
    }))
}
pub(crate) fn inventory(
    providers: &[&dyn SourceCandidateProvider],
    group: &Group,
    name: &str,
    scope: CandidateInventoryScope,
    limits: &SourceInventoryLimits,
    short_circuit: bool,
) -> Result<SourceCandidateInventory, DepProviderError> {
    if let Some(owner) = providers
        .iter()
        .find(|p| p.has_locked_coordinate(group, name))
    {
        return owner.source_inventory(group, name, scope, limits);
    }
    let mut result = SourceCandidateInventory {
        candidates: Vec::new(),
        observations: Vec::new(),
    };
    let mut default_domain = BTreeSet::new();
    let mut first_family = BTreeMap::new();
    let mut first_serving = None;
    let mut offline_miss = None;
    for (family, provider) in providers.iter().enumerate() {
        let inventory = match provider.source_inventory(group, name, scope, limits) {
            Ok(value) => value,
            Err(error @ DepProviderError::OfflineUnavailable { .. }) => {
                offline_miss = Some(error);
                continue;
            }
            Err(error) if absent(&error) => continue,
            Err(error) => return Err(error),
        };
        let count = result
            .candidates
            .len()
            .saturating_add(inventory.candidates.len());
        if count > limits.max_candidates {
            return Err(DepProviderError::SourceMetadataLimit {
                observed: count,
                limit: limits.max_candidates,
            });
        }
        let domain: BTreeSet<_> = inventory
            .candidates
            .iter()
            .filter(|r| r.in_default_version_domain)
            .map(|r| r.candidate.version().clone())
            .collect();
        if !domain.is_empty() && first_serving.is_none() {
            first_serving = Some(family);
        }
        if !short_circuit || first_serving == Some(family) {
            for version in domain {
                default_domain.insert(version.clone());
                first_family.entry(version).or_insert(family);
            }
        }
        for mut record in inventory.candidates {
            let version = record.candidate.version();
            let family_owns = first_family.get(version) == Some(&family);
            record.in_default_version_domain = default_domain.contains(version);
            record.default_eligible &= family_owns;
            record.legacy_preferred_source &= family_owns;
            result.candidates.push(record);
        }
        result.observations.extend(inventory.observations);
        if short_circuit
            && first_serving.is_some()
            && matches!(scope, CandidateInventoryScope::DefaultOnly)
        {
            break;
        }
    }
    // Later families can add versions to a union. Update the diagnostic fact
    // for earlier nondefault records without broadening their source eligibility.
    for record in &mut result.candidates {
        record.in_default_version_domain = default_domain.contains(record.candidate.version());
    }
    // No family served it and one could not look because of the offline
    // posture: that is the actionable verdict, not an empty inventory.
    if let Some(miss) = offline_miss
        && result.candidates.is_empty()
        && result.observations.is_empty()
    {
        return Err(miss);
    }
    Ok(result)
}
