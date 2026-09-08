//! Canonical pre-gather artifact discovery over parsed directives.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::use_graph::use_addresses;
use crate::{DirectiveKind, SectionSource, SpecAddress};

use super::embed_snapshot::EmbedResolutionSnapshot;
use super::ir::{
    ArtifactInputKind, ArtifactPlan, DocumentAddress, DocumentIr, DocumentSubject, SourceIr,
};
use super::source_snapshot::{DocumentObservation, ExpansionObservation, SourceResolutionSnapshot};

#[derive(Debug)]
pub(crate) struct Worklist {
    pub(crate) documents: Vec<DocumentIr>,
    pub(crate) sources: SourceResolutionSnapshot,
    pub(crate) embeds: EmbedResolutionSnapshot,
    pub(crate) owners: ErrorOwners,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ErrorOwners(BTreeMap<String, usize>);

impl ErrorOwners {
    fn record(&mut self, address: &SpecAddress, input: usize) {
        self.0.entry(address.to_string()).or_insert(input);
        self.0.entry(address.without_pin()).or_insert(input);
    }

    pub(crate) fn owner(&self, address: &str) -> Option<usize> {
        self.0.get(address).copied().or_else(|| {
            SpecAddress::parse(address)
                .ok()
                .and_then(|parsed| self.0.get(&parsed.without_pin()).copied())
        })
    }
}

#[derive(Debug, Clone)]
enum DiscoveryKey {
    Spec(String),
    Simple(DocumentKey),
}

enum ArtifactRoot {
    Normal { input: usize, keys: Vec<String> },
    Simple { input: usize, key: DocumentKey },
}

pub(crate) fn discover<E>(
    plan: &ArtifactPlan,
    source: &impl SectionSource,
    parse: impl Fn(SourceIr) -> Result<DocumentIr, E>,
    record_use_failure: impl Fn(&SpecAddress, String),
) -> Result<Worklist, E> {
    discover_with_formats(
        plan,
        source,
        &[],
        |source, _physical_stem| parse(source),
        record_use_failure,
    )
}

pub(crate) fn discover_with_formats<E>(
    plan: &ArtifactPlan,
    source: &impl SectionSource,
    active_formats: &[String],
    parse: impl Fn(SourceIr, &str) -> Result<DocumentIr, E>,
    record_use_failure: impl Fn(&SpecAddress, String),
) -> Result<Worklist, E> {
    let source = ActiveSource {
        source,
        active_formats,
    };
    let mut resolved = BTreeMap::new();
    let mut simple = BTreeMap::new();
    let mut failures = BTreeMap::new();
    let mut discovery_order = Vec::new();
    let mut use_order = Vec::new();
    let mut roots = Vec::new();
    let mut owners = ErrorOwners::default();

    for (input_index, input) in plan.contributions().iter().enumerate() {
        match input.kind() {
            ArtifactInputKind::Normal { seed, .. } => {
                let mut seen = HashSet::new();
                let mut membership = Vec::new();
                discover_uses(
                    seed,
                    input.subject(),
                    &source,
                    &parse,
                    &record_use_failure,
                    &mut seen,
                    &mut membership,
                    &mut use_order,
                    &mut discovery_order,
                    &mut resolved,
                    &mut failures,
                    input_index,
                    &mut owners,
                )?;
                roots.push(ArtifactRoot::Normal {
                    input: input_index,
                    keys: membership,
                });
            }
            ArtifactInputKind::Simple { source, .. } => {
                let key = document_key(source.address());
                roots.push(ArtifactRoot::Simple {
                    input: input_index,
                    key: key.clone(),
                });
                if let std::collections::btree_map::Entry::Vacant(entry) = simple.entry(key.clone())
                {
                    discovery_order.push(DiscoveryKey::Simple(key));
                    let physical_stem = physical_stem(source);
                    entry.insert(parse(source.clone(), &physical_stem)?);
                }
            }
            ArtifactInputKind::Elided { .. } | ArtifactInputKind::Hoisted { .. } => {}
        }
    }

    let mut expansions = BTreeMap::new();
    let mut source_membership = BTreeMap::new();
    for key in use_order.clone() {
        let mut seen = HashSet::new();
        let mut membership = Vec::new();
        discover_sources(
            &key,
            &source,
            &parse,
            &mut seen,
            &mut membership,
            &mut discovery_order,
            &mut resolved,
            &mut failures,
            &mut expansions,
            owners.owner(&key).unwrap_or(0),
            &mut owners,
        )?;
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
            ArtifactRoot::Normal { input, keys } => {
                for key in keys {
                    discover_embeds(
                        &key,
                        &source,
                        &parse,
                        &mut embed_seen,
                        &mut embed_order,
                        &mut discovery_order,
                        &mut resolved,
                        &mut failures,
                        input,
                        &mut owners,
                    )?;
                    for source_key in source_membership.get(&key).into_iter().flatten() {
                        discover_embeds(
                            source_key,
                            &source,
                            &parse,
                            &mut embed_seen,
                            &mut embed_order,
                            &mut discovery_order,
                            &mut resolved,
                            &mut failures,
                            input,
                            &mut owners,
                        )?;
                    }
                }
            }
            ArtifactRoot::Simple { input, key } => {
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
                    &source,
                    &parse,
                    &mut embed_seen,
                    &mut embed_order,
                    &mut discovery_order,
                    &mut resolved,
                    &mut failures,
                    input,
                    &mut owners,
                )?;
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
    Ok(Worklist {
        documents,
        sources,
        embeds,
        owners,
    })
}

struct ActiveSource<'source, Source> {
    source: &'source Source,
    active_formats: &'source [String],
}

impl<Source: SectionSource> ActiveSource<'_, Source> {
    fn load(&self, address: &SpecAddress) -> Result<crate::embed::ResolvedSource, String> {
        self.source.resolved_source(address, self.active_formats)
    }

    fn expand_pattern(&self, address: &SpecAddress) -> Result<Vec<SpecAddress>, String> {
        self.source.expand_pattern(address)
    }
}

fn physical_stem(source: &SourceIr) -> String {
    match source.address() {
        DocumentAddress::Spec(address) => address
            .doc_path
            .rsplit('/')
            .next()
            .unwrap_or(&address.doc_path)
            .to_owned(),
        DocumentAddress::StaticEntry { path, .. } => std::path::Path::new(path)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or(path)
            .to_owned(),
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

/// The canonical identity of one gathered document.
///
/// Typed, never a delimiter-joined string. A joined spelling such as
/// `static:{origin}\0{path}` cannot separate `("a", "b\0c")` from `("a\0b", "c")`,
/// so two genuinely distinct static entries would land on one map slot and the
/// second would silently overwrite the first. This is the key every map that
/// can overwrite a document uses — discovery, close, and the inter-pass gather
/// guard alike — so the guard's collision set is exactly the map's.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum DocumentKey {
    Spec(String),
    Static { origin: String, path: String },
}

impl DocumentKey {
    /// A human label for diagnostics; never used as an identity.
    pub(crate) fn label(&self) -> String {
        match self {
            Self::Spec(key) => key.clone(),
            Self::Static { origin, path } => {
                format!("static entry (origin {origin:?}, path {path:?})")
            }
        }
    }
}

impl std::fmt::Display for DocumentKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.label())
    }
}

pub(crate) fn document_key(address: &DocumentAddress) -> DocumentKey {
    match address {
        DocumentAddress::Spec(address) => DocumentKey::Spec(address.without_pin()),
        DocumentAddress::StaticEntry { origin, path } => DocumentKey::Static {
            origin: origin.clone(),
            path: path.clone(),
        },
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
            .map(|(key, document)| (key.clone(), DocumentObservation::resolved(document.clone()))),
    );
    out
}

#[allow(clippy::too_many_arguments)]
fn discover_uses<E>(
    address: &SpecAddress,
    subject: &DocumentSubject,
    source: &ActiveSource<'_, impl SectionSource>,
    parse: &impl Fn(SourceIr, &str) -> Result<DocumentIr, E>,
    record_failure: &impl Fn(&SpecAddress, String),
    seen: &mut HashSet<String>,
    membership: &mut Vec<String>,
    use_order: &mut Vec<String>,
    discovery_order: &mut Vec<DiscoveryKey>,
    resolved: &mut BTreeMap<String, DocumentIr>,
    failures: &mut BTreeMap<String, DocumentObservation>,
    input: usize,
    owners: &mut ErrorOwners,
) -> Result<(), E> {
    owners.record(address, input);
    let key = address.without_pin();
    if !seen.insert(key.clone()) {
        return Ok(());
    }
    membership.push(key.clone());
    if let Some(DocumentObservation::Failed { reason, .. }) = failures.get(&key) {
        record_failure(address, reason.clone());
        return Ok(());
    }
    if !resolved.contains_key(&key) {
        let source_observation = match source.load(address) {
            Ok(resolved) => resolved,
            Err(reason) => {
                record_failure(address, reason.clone());
                failures.entry(key).or_insert(DocumentObservation::Failed {
                    requested: address.clone(),
                    reason,
                });
                return Ok(());
            }
        };
        let (text, format, physical_stem) = source_observation.into_parts();
        let document = parse(
            SourceIr::new(
                DocumentAddress::Spec(address.clone()),
                format,
                subject.clone(),
                text,
            ),
            &physical_stem,
        )?;
        discovery_order.push(DiscoveryKey::Spec(key.clone()));
        resolved.insert(key.clone(), document);
    }
    if !use_order.contains(&key) {
        use_order.push(key.clone());
    }
    let targets = use_addresses(resolved[&key].tree().directives());
    for target in targets {
        let reached = DocumentSubject::reached(&DocumentAddress::Spec(target.clone()));
        discover_uses(
            &target,
            &reached,
            source,
            parse,
            record_failure,
            seen,
            membership,
            use_order,
            discovery_order,
            resolved,
            failures,
            input,
            owners,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn discover_sources<E>(
    key: &str,
    source: &ActiveSource<'_, impl SectionSource>,
    parse: &impl Fn(SourceIr, &str) -> Result<DocumentIr, E>,
    seen: &mut HashSet<String>,
    membership: &mut Vec<String>,
    discovery_order: &mut Vec<DiscoveryKey>,
    resolved: &mut BTreeMap<String, DocumentIr>,
    failures: &mut BTreeMap<String, DocumentObservation>,
    expansions: &mut BTreeMap<String, ExpansionObservation>,
    input: usize,
    owners: &mut ErrorOwners,
) -> Result<(), E> {
    if !seen.insert(key.to_string()) {
        return Ok(());
    }
    let Some(document) = resolved.get(key) else {
        return Ok(());
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
        owners.record(&pattern, input);
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
            owners.record(&target, input);
            observe_document(&target, source, parse, discovery_order, resolved, failures)?;
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
                    input,
                    owners,
                )?;
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn discover_embeds<E>(
    key: &str,
    source: &ActiveSource<'_, impl SectionSource>,
    parse: &impl Fn(SourceIr, &str) -> Result<DocumentIr, E>,
    seen: &mut HashSet<String>,
    embed_order: &mut Vec<String>,
    discovery_order: &mut Vec<DiscoveryKey>,
    resolved: &mut BTreeMap<String, DocumentIr>,
    failures: &mut BTreeMap<String, DocumentObservation>,
    input: usize,
    owners: &mut ErrorOwners,
) -> Result<(), E> {
    if !seen.insert(key.to_string()) {
        return Ok(());
    }
    let Some(document) = resolved.get(key) else {
        return Ok(());
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
        input,
        owners,
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn discover_embed_targets<E>(
    targets: impl IntoIterator<Item = SpecAddress>,
    source: &ActiveSource<'_, impl SectionSource>,
    parse: &impl Fn(SourceIr, &str) -> Result<DocumentIr, E>,
    seen: &mut HashSet<String>,
    embed_order: &mut Vec<String>,
    discovery_order: &mut Vec<DiscoveryKey>,
    resolved: &mut BTreeMap<String, DocumentIr>,
    failures: &mut BTreeMap<String, DocumentObservation>,
    input: usize,
    owners: &mut ErrorOwners,
) -> Result<(), E> {
    for target in targets {
        owners.record(&target, input);
        let target_key = target.without_pin();
        if !embed_order.contains(&target_key) {
            embed_order.push(target_key.clone());
        }
        observe_document(&target, source, parse, discovery_order, resolved, failures)?;
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
                input,
                owners,
            )?;
        }
    }
    Ok(())
}

fn observe_document<E>(
    address: &SpecAddress,
    source: &ActiveSource<'_, impl SectionSource>,
    parse: &impl Fn(SourceIr, &str) -> Result<DocumentIr, E>,
    discovery_order: &mut Vec<DiscoveryKey>,
    resolved: &mut BTreeMap<String, DocumentIr>,
    failures: &mut BTreeMap<String, DocumentObservation>,
) -> Result<(), E> {
    let key = address.without_pin();
    if resolved.contains_key(&key) || failures.contains_key(&key) {
        return Ok(());
    }
    match source.load(address) {
        Ok(source_observation) => {
            // A `#source` expansion or `#embed` target: reached, never
            // declared, so it carries its own address identity as its subject.
            let (text, format, physical_stem) = source_observation.into_parts();
            let document = parse(
                SourceIr::reached(DocumentAddress::Spec(address.clone()), format, text),
                &physical_stem,
            )?;
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
    Ok(())
}

#[cfg(test)]
#[path = "worklist/tests.rs"]
mod tests;
