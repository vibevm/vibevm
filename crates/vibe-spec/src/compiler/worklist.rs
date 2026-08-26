//! Canonical pre-gather artifact discovery over parsed directives.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::use_graph::use_addresses;
use crate::{DirectiveKind, SectionSource, SpecAddress};

use super::embed_snapshot::EmbedResolutionSnapshot;
use super::ir::{
    ArtifactInput, ArtifactPlan, DocumentAddress, DocumentIr, SourceFormatId, SourceIr,
};
use super::source_snapshot::{DocumentObservation, ExpansionObservation, SourceResolutionSnapshot};

pub(crate) struct Worklist {
    pub(crate) documents: Vec<DocumentIr>,
    pub(crate) sources: SourceResolutionSnapshot,
    pub(crate) embeds: EmbedResolutionSnapshot,
}

#[derive(Debug, Clone)]
enum DiscoveryKey {
    Spec(String),
    Simple(String),
}

enum ArtifactRoot {
    Normal(Vec<String>),
    Simple(String),
}

pub(crate) fn discover(
    plan: &ArtifactPlan,
    source: &impl SectionSource,
    parse: impl Fn(SourceIr) -> DocumentIr,
    record_use_failure: impl Fn(&SpecAddress, String),
) -> Worklist {
    let mut resolved = BTreeMap::new();
    let mut simple = BTreeMap::new();
    let mut failures = BTreeMap::new();
    let mut discovery_order = Vec::new();
    let mut use_order = Vec::new();
    let mut roots = Vec::new();

    for input in plan.contributions() {
        match input {
            ArtifactInput::Normal { seed, .. } => {
                let mut seen = HashSet::new();
                let mut membership = Vec::new();
                discover_uses(
                    seed,
                    source,
                    &parse,
                    &record_use_failure,
                    &mut seen,
                    &mut membership,
                    &mut use_order,
                    &mut discovery_order,
                    &mut resolved,
                    &mut failures,
                );
                roots.push(ArtifactRoot::Normal(membership));
            }
            ArtifactInput::Simple { source, .. } => {
                let key = document_key(source.address());
                roots.push(ArtifactRoot::Simple(key.clone()));
                if let std::collections::btree_map::Entry::Vacant(entry) = simple.entry(key.clone())
                {
                    discovery_order.push(DiscoveryKey::Simple(key));
                    entry.insert(parse(source.clone()));
                }
            }
            ArtifactInput::Elided { .. } | ArtifactInput::Hoisted { .. } => {}
        }
    }

    let mut expansions = BTreeMap::new();
    let mut source_membership = BTreeMap::new();
    for key in use_order.clone() {
        let mut seen = HashSet::new();
        let mut membership = Vec::new();
        discover_sources(
            &key,
            source,
            &parse,
            &mut seen,
            &mut membership,
            &mut discovery_order,
            &mut resolved,
            &mut failures,
            &mut expansions,
        );
        source_membership.insert(key, membership);
    }

    let sources = SourceResolutionSnapshot {
        discovery_order: spec_order(&discovery_order),
        documents: observations(&resolved, &failures),
        expansions,
        explicit_use_keys: use_order.iter().cloned().collect(),
    };

    let mut embed_seen = HashSet::new();
    let mut embed_order = Vec::new();
    for root in roots {
        match root {
            ArtifactRoot::Normal(keys) => {
                for key in keys {
                    discover_embeds(
                        &key,
                        source,
                        &parse,
                        &mut embed_seen,
                        &mut embed_order,
                        &mut discovery_order,
                        &mut resolved,
                        &mut failures,
                    );
                    for source_key in source_membership.get(&key).into_iter().flatten() {
                        discover_embeds(
                            source_key,
                            source,
                            &parse,
                            &mut embed_seen,
                            &mut embed_order,
                            &mut discovery_order,
                            &mut resolved,
                            &mut failures,
                        );
                    }
                }
            }
            ArtifactRoot::Simple(key) => {
                let document = &simple[&key];
                let targets = document
                    .tree()
                    .directives()
                    .directives
                    .iter()
                    .filter(|directive| directive.kind == DirectiveKind::Embed)
                    .map(|directive| directive.address.clone())
                    .collect::<Vec<_>>();
                discover_embed_targets(
                    targets,
                    source,
                    &parse,
                    &mut embed_seen,
                    &mut embed_order,
                    &mut discovery_order,
                    &mut resolved,
                    &mut failures,
                );
            }
        }
    }

    let embeds = EmbedResolutionSnapshot {
        discovery_order: embed_order,
        documents: observations(&resolved, &failures),
        explicit_use_keys: use_order.into_iter().collect::<BTreeSet<_>>(),
    };
    let documents = discovery_order
        .into_iter()
        .filter_map(|key| match key {
            DiscoveryKey::Spec(key) => resolved.remove(&key),
            DiscoveryKey::Simple(key) => simple.remove(&key),
        })
        .collect();
    Worklist {
        documents,
        sources,
        embeds,
    }
}

fn spec_order(order: &[DiscoveryKey]) -> Vec<String> {
    order
        .iter()
        .filter_map(|key| match key {
            DiscoveryKey::Spec(key) => Some(key.clone()),
            DiscoveryKey::Simple(_) => None,
        })
        .collect()
}

pub(crate) fn document_key(address: &DocumentAddress) -> String {
    match address {
        DocumentAddress::Spec(address) => address.without_pin(),
        DocumentAddress::StaticEntry { origin, path } => format!("static:{origin}\0{path}"),
    }
}

fn observations(
    resolved: &BTreeMap<String, DocumentIr>,
    failures: &BTreeMap<String, DocumentObservation>,
) -> BTreeMap<String, DocumentObservation> {
    let mut out = failures.clone();
    out.extend(
        resolved
            .iter()
            .map(|(key, document)| (key.clone(), DocumentObservation::Resolved(document.clone()))),
    );
    out
}

#[allow(clippy::too_many_arguments)]
fn discover_uses(
    address: &SpecAddress,
    source: &impl SectionSource,
    parse: &impl Fn(SourceIr) -> DocumentIr,
    record_failure: &impl Fn(&SpecAddress, String),
    seen: &mut HashSet<String>,
    membership: &mut Vec<String>,
    use_order: &mut Vec<String>,
    discovery_order: &mut Vec<DiscoveryKey>,
    resolved: &mut BTreeMap<String, DocumentIr>,
    failures: &mut BTreeMap<String, DocumentObservation>,
) {
    let key = address.without_pin();
    if !seen.insert(key.clone()) {
        return;
    }
    membership.push(key.clone());
    if let Some(DocumentObservation::Failed { reason, .. }) = failures.get(&key) {
        record_failure(address, reason.clone());
        return;
    }
    if !resolved.contains_key(&key) {
        let text = match source.section_text(address) {
            Ok(text) => text,
            Err(reason) => {
                record_failure(address, reason.clone());
                failures.entry(key).or_insert(DocumentObservation::Failed {
                    requested: address.clone(),
                    reason,
                });
                return;
            }
        };
        let document = parse(SourceIr::new(
            DocumentAddress::Spec(address.clone()),
            SourceFormatId::canonical_markdown(),
            text,
        ));
        discovery_order.push(DiscoveryKey::Spec(key.clone()));
        resolved.insert(key.clone(), document);
    }
    if !use_order.contains(&key) {
        use_order.push(key.clone());
    }
    let targets = use_addresses(resolved[&key].tree().directives());
    for target in targets {
        discover_uses(
            &target,
            source,
            parse,
            record_failure,
            seen,
            membership,
            use_order,
            discovery_order,
            resolved,
            failures,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn discover_sources(
    key: &str,
    source: &impl SectionSource,
    parse: &impl Fn(SourceIr) -> DocumentIr,
    seen: &mut HashSet<String>,
    membership: &mut Vec<String>,
    discovery_order: &mut Vec<DiscoveryKey>,
    resolved: &mut BTreeMap<String, DocumentIr>,
    failures: &mut BTreeMap<String, DocumentObservation>,
    expansions: &mut BTreeMap<String, ExpansionObservation>,
) {
    if !seen.insert(key.to_string()) {
        return;
    }
    let Some(document) = resolved.get(key) else {
        return;
    };
    let patterns: Vec<SpecAddress> = document
        .tree()
        .directives()
        .directives
        .iter()
        .filter(|directive| directive.kind == DirectiveKind::Source)
        .map(|directive| directive.address.clone())
        .collect();
    for pattern in patterns {
        let request_key = pattern.to_string();
        expansions.entry(request_key.clone()).or_insert_with(|| {
            match source.expand_pattern(&pattern) {
                Ok(targets) => ExpansionObservation::Resolved {
                    requested: pattern.clone(),
                    targets,
                },
                Err(reason) => ExpansionObservation::Failed {
                    requested: pattern.clone(),
                    reason,
                },
            }
        });
        let targets = match &expansions[&request_key] {
            ExpansionObservation::Resolved { targets, .. } => targets.clone(),
            ExpansionObservation::Failed { .. } => continue,
        };
        for target in targets {
            observe_document(&target, source, parse, discovery_order, resolved, failures);
            let target_key = target.without_pin();
            if !membership.contains(&target_key) {
                membership.push(target_key.clone());
            }
            if resolved.contains_key(&target_key) {
                discover_sources(
                    &target_key,
                    source,
                    parse,
                    seen,
                    membership,
                    discovery_order,
                    resolved,
                    failures,
                    expansions,
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn discover_embeds(
    key: &str,
    source: &impl SectionSource,
    parse: &impl Fn(SourceIr) -> DocumentIr,
    seen: &mut HashSet<String>,
    embed_order: &mut Vec<String>,
    discovery_order: &mut Vec<DiscoveryKey>,
    resolved: &mut BTreeMap<String, DocumentIr>,
    failures: &mut BTreeMap<String, DocumentObservation>,
) {
    if !seen.insert(key.to_string()) {
        return;
    }
    let Some(document) = resolved.get(key) else {
        return;
    };
    let targets = document
        .tree()
        .directives()
        .directives
        .iter()
        .filter(|directive| directive.kind == DirectiveKind::Embed)
        .map(|directive| directive.address.clone())
        .collect::<Vec<_>>();
    discover_embed_targets(
        targets,
        source,
        parse,
        seen,
        embed_order,
        discovery_order,
        resolved,
        failures,
    );
}

#[allow(clippy::too_many_arguments)]
fn discover_embed_targets(
    targets: impl IntoIterator<Item = SpecAddress>,
    source: &impl SectionSource,
    parse: &impl Fn(SourceIr) -> DocumentIr,
    seen: &mut HashSet<String>,
    embed_order: &mut Vec<String>,
    discovery_order: &mut Vec<DiscoveryKey>,
    resolved: &mut BTreeMap<String, DocumentIr>,
    failures: &mut BTreeMap<String, DocumentObservation>,
) {
    for target in targets {
        let target_key = target.without_pin();
        if !embed_order.contains(&target_key) {
            embed_order.push(target_key.clone());
        }
        observe_document(&target, source, parse, discovery_order, resolved, failures);
        if resolved.contains_key(&target_key) {
            discover_embeds(
                &target_key,
                source,
                parse,
                seen,
                embed_order,
                discovery_order,
                resolved,
                failures,
            );
        }
    }
}

fn observe_document(
    address: &SpecAddress,
    source: &impl SectionSource,
    parse: &impl Fn(SourceIr) -> DocumentIr,
    discovery_order: &mut Vec<DiscoveryKey>,
    resolved: &mut BTreeMap<String, DocumentIr>,
    failures: &mut BTreeMap<String, DocumentObservation>,
) {
    let key = address.without_pin();
    if resolved.contains_key(&key) || failures.contains_key(&key) {
        return;
    }
    match source.section_text(address) {
        Ok(text) => {
            let document = parse(SourceIr::new(
                DocumentAddress::Spec(address.clone()),
                SourceFormatId::canonical_markdown(),
                text,
            ));
            discovery_order.push(DiscoveryKey::Spec(key.clone()));
            resolved.insert(key, document);
        }
        Err(reason) => {
            failures.insert(
                key,
                DocumentObservation::Failed {
                    requested: address.clone(),
                    reason,
                },
            );
        }
    }
}
