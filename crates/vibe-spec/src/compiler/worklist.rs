//! Canonical pre-gather use/source/embed worklist observation.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::use_graph::use_addresses;
use crate::{DirectiveKind, SectionSource, SpecAddress};

use super::embed_snapshot::EmbedResolutionSnapshot;
use super::ir::{DocumentAddress, DocumentIr, SourceFormatId, SourceIr};
use super::source_snapshot::{DocumentObservation, ExpansionObservation, SourceResolutionSnapshot};

pub(crate) struct Worklist {
    pub(crate) documents: Vec<DocumentIr>,
    pub(crate) sources: SourceResolutionSnapshot,
    pub(crate) embeds: EmbedResolutionSnapshot,
}

pub(crate) fn discover(
    seed: &SpecAddress,
    source: &impl SectionSource,
    parse: impl Fn(SourceIr) -> DocumentIr,
    record_use_failure: impl Fn(&SpecAddress, String),
) -> Worklist {
    let mut resolved = BTreeMap::new();
    let mut failures = BTreeMap::new();
    let mut discovery_order = Vec::new();
    let mut use_seen = HashSet::new();
    let mut use_order = Vec::new();
    discover_uses(
        seed,
        source,
        &parse,
        &record_use_failure,
        &mut use_seen,
        &mut use_order,
        &mut discovery_order,
        &mut resolved,
        &mut failures,
    );

    let mut expansions = BTreeMap::new();
    let mut source_seen = HashSet::new();
    for key in use_order.clone() {
        discover_sources(
            &key,
            source,
            &parse,
            &mut source_seen,
            &mut discovery_order,
            &mut resolved,
            &mut failures,
            &mut expansions,
        );
    }

    let source_observations = observations(&resolved, &failures);
    let sources = SourceResolutionSnapshot {
        discovery_order: discovery_order.clone(),
        documents: source_observations,
        expansions,
        explicit_use_keys: use_order.iter().cloned().collect(),
    };

    let embed_seeds = discovery_order.clone();
    let mut embed_seen = HashSet::new();
    let mut embed_order = Vec::new();
    for key in embed_seeds {
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
    }
    let embeds = EmbedResolutionSnapshot {
        discovery_order: embed_order,
        documents: observations(&resolved, &failures),
        explicit_use_keys: use_order.into_iter().collect::<BTreeSet<_>>(),
    };
    let documents = discovery_order
        .into_iter()
        .filter_map(|key| resolved.remove(&key))
        .collect();
    Worklist {
        documents,
        sources,
        embeds,
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
    use_order: &mut Vec<String>,
    discovery_order: &mut Vec<String>,
    resolved: &mut BTreeMap<String, DocumentIr>,
    failures: &mut BTreeMap<String, DocumentObservation>,
) {
    let key = address.without_pin();
    if !seen.insert(key.clone()) {
        return;
    }
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
    let targets = use_addresses(document.tree().directives());
    discovery_order.push(key.clone());
    use_order.push(key.clone());
    resolved.insert(key, document);
    for target in targets {
        discover_uses(
            &target,
            source,
            parse,
            record_failure,
            seen,
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
    discovery_order: &mut Vec<String>,
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
        let pattern_key = pattern.without_pin();
        expansions.entry(pattern_key.clone()).or_insert_with(|| {
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
        let targets = match &expansions[&pattern_key] {
            ExpansionObservation::Resolved { targets, .. } => targets.clone(),
            ExpansionObservation::Failed { .. } => continue,
        };
        for target in targets {
            observe_document(&target, source, parse, discovery_order, resolved, failures);
            let target_key = target.without_pin();
            if resolved.contains_key(&target_key) {
                discover_sources(
                    &target_key,
                    source,
                    parse,
                    seen,
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
    discovery_order: &mut Vec<String>,
    resolved: &mut BTreeMap<String, DocumentIr>,
    failures: &mut BTreeMap<String, DocumentObservation>,
) {
    if !seen.insert(key.to_string()) {
        return;
    }
    let Some(document) = resolved.get(key) else {
        return;
    };
    let targets: Vec<SpecAddress> = document
        .tree()
        .directives()
        .directives
        .iter()
        .filter(|directive| directive.kind == DirectiveKind::Embed)
        .map(|directive| directive.address.clone())
        .collect();
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
    discovery_order: &mut Vec<String>,
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
            discovery_order.push(key.clone());
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
