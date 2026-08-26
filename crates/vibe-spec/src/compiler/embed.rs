//! The built-in recursive `#embed` pass.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR");

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::embed::{EmbedError, expand_with};
use crate::{DirectiveKind, DocTree, SpecAddress};

use super::close::document_origin;
use super::ir::{
    ClosureContribution, ClosureDocument, ClosureEdge, ClosureEdgeKind, ClosureIr, ClosureNodeId,
    DocumentAddress,
};
use super::pass::{Pass, PassName};
use super::source_snapshot::DocumentObservation;

pub(crate) const EMBED_PASS_NAME: &str = "embed";

pub(crate) struct EmbedPass {
    name: PassName,
}

impl EmbedPass {
    pub(crate) fn new() -> Self {
        Self {
            name: PassName::new(EMBED_PASS_NAME)
                .expect("the static built-in embed pass name is non-blank"),
        }
    }
}

impl Pass for EmbedPass {
    type Input = ClosureIr;
    type Output = ClosureIr;
    type Error = EmbedPassError;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, input: ClosureIr) -> Result<ClosureIr, EmbedPassError> {
        #[cfg(test)]
        EMBED_INVOCATIONS.with(|count| count.set(count.get() + 1));
        embed_closure(input)
    }
}

#[cfg(test)]
std::thread_local! {
    static EMBED_INVOCATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_embed_invocations() {
    EMBED_INVOCATIONS.with(|count| count.set(0));
}

#[cfg(test)]
pub(crate) fn embed_invocations() -> usize {
    EMBED_INVOCATIONS.with(std::cell::Cell::get)
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum EmbedPassError {
    #[error(transparent)]
    Embed(#[from] EmbedError),
    #[error("embed observation missing for `{addr}`")]
    MissingObservation { addr: String },
}

impl EmbedPassError {
    pub(crate) fn into_compile_error(self) -> crate::pipeline::CompileError {
        match self {
            Self::Embed(error) => crate::pipeline::CompileError::Embed(error),
            Self::MissingObservation { addr } => {
                panic!("current closure introduced unobserved embed `{addr}`")
            }
        }
    }
}

struct RootUpdate {
    node: ClosureNodeId,
    tree: DocTree,
    aliases: BTreeMap<String, SpecAddress>,
}

struct SimpleUpdate {
    contribution: usize,
    tree: DocTree,
    aliases: BTreeMap<String, SpecAddress>,
}

enum EmbedRoot {
    Normal(ClosureNodeId),
    Simple(usize),
}

fn embed_closure(mut closure: ClosureIr) -> Result<ClosureIr, EmbedPassError> {
    embed_closure_in_place(&mut closure)?;
    Ok(closure)
}

fn embed_closure_in_place(closure: &mut ClosureIr) -> Result<(), EmbedPassError> {
    let snapshot = closure
        .pending_embeds
        .as_ref()
        .expect("close supplies one pending embed snapshot");
    let roots: Vec<EmbedRoot> = closure
        .contributions
        .iter()
        .enumerate()
        .flat_map(|(index, contribution)| match contribution {
            ClosureContribution::Normal { emission_order, .. } => emission_order
                .iter()
                .map(|occurrence| EmbedRoot::Normal(occurrence.node))
                .collect(),
            ClosureContribution::Simple { .. } => vec![EmbedRoot::Simple(index)],
            ClosureContribution::Elided { .. } | ClosureContribution::Hoisted { .. } => Vec::new(),
        })
        .collect();
    let current: HashMap<String, DocTree> = roots
        .iter()
        .filter_map(|root| match root {
            EmbedRoot::Normal(node) => match &closure.nodes[node.0].address {
                DocumentAddress::Spec(address) => {
                    Some((address.without_pin(), closure.nodes[node.0].tree.clone()))
                }
                DocumentAddress::StaticEntry { .. } => unreachable!(),
            },
            EmbedRoot::Simple(_) => None,
        })
        .collect();

    let mut updates = Vec::new();
    let mut accepted_nodes = Vec::new();
    let mut accepted_edges = Vec::new();
    let mut accepted_occurrences = HashSet::new();
    let mut simple_updates = Vec::new();
    let mut normal_seen = HashSet::new();
    let mut simple_cache: HashMap<
        super::worklist::DocumentKey,
        (DocTree, BTreeMap<String, SpecAddress>),
    > = HashMap::new();
    for root in roots {
        match root {
            EmbedRoot::Normal(node) if normal_seen.insert(node) => {
                let DocumentAddress::Spec(address) = &closure.nodes[node.0].address else {
                    unreachable!("normal embed root is spec-addressed")
                };
                let tree = &closure.nodes[node.0].tree;
                let aliases = tree.directives().aliases.clone();
                let normalized = strip_resolved_directives(tree);
                let mut missing = None;
                let mut observed_failure = None;
                let mut resolve =
                    |target: &SpecAddress| match lookup_text(target, snapshot, &current) {
                        Ok(text) => Ok(text),
                        Err(EmbedPassError::Embed(error @ EmbedError::Unresolved { .. })) => {
                            observed_failure = Some(error);
                            Err("recorded embed observation failed".to_string())
                        }
                        Err(EmbedPassError::MissingObservation { addr }) => {
                            missing = Some(addr);
                            Err("missing compiler embed observation".to_string())
                        }
                        Err(EmbedPassError::Embed(EmbedError::Cycle(_))) => unreachable!(),
                    };
                let mut edge = |from: &str, ordinal: usize, target: &SpecAddress| {
                    let target_key = target.without_pin();
                    if accepted_occurrences.insert((from.to_string(), ordinal)) {
                        accepted_edges.push((from.to_string(), target.clone()));
                    }
                    if !accepted_nodes
                        .iter()
                        .any(|accepted: &SpecAddress| accepted.without_pin() == target_key)
                    {
                        accepted_nodes.push(target.clone());
                    }
                };
                let expanded =
                    expand_with(&normalized, &address.without_pin(), &mut resolve, &mut edge);
                let expanded = finish_expansion(expanded, missing, observed_failure)?;
                updates.push(RootUpdate {
                    node,
                    tree: DocTree::parse(&expanded),
                    aliases,
                });
            }
            EmbedRoot::Normal(_) => {}
            EmbedRoot::Simple(contribution) => {
                let ClosureContribution::Simple { document, .. } =
                    &closure.contributions[contribution]
                else {
                    unreachable!()
                };
                let base = super::worklist::document_key(&document.address);
                let (tree, aliases) = if let Some(cached) = simple_cache.get(&base) {
                    cached.clone()
                } else {
                    let aliases = document.tree.directives().aliases.clone();
                    let mut missing = None;
                    let mut observed_failure = None;
                    let mut resolve =
                        |target: &SpecAddress| match lookup_text(target, snapshot, &current) {
                            Ok(text) => Ok(text),
                            Err(EmbedPassError::Embed(error @ EmbedError::Unresolved { .. })) => {
                                observed_failure = Some(error);
                                Err("recorded embed observation failed".to_string())
                            }
                            Err(EmbedPassError::MissingObservation { addr }) => {
                                missing = Some(addr);
                                Err("missing compiler embed observation".to_string())
                            }
                            Err(EmbedPassError::Embed(EmbedError::Cycle(_))) => unreachable!(),
                        };
                    let mut ignore_edge = |_: &str, _: usize, _: &SpecAddress| {};
                    let expanded = expand_with(
                        &document.tree.text(document.tree.root()),
                        &base.label(),
                        &mut resolve,
                        &mut ignore_edge,
                    );
                    let expanded = finish_expansion(expanded, missing, observed_failure)?;
                    let value = (DocTree::parse(&expanded), aliases);
                    simple_cache.insert(base, value.clone());
                    value
                };
                simple_updates.push(SimpleUpdate {
                    contribution,
                    tree,
                    aliases,
                });
            }
        }
    }

    let existing_keys: HashSet<String> = closure
        .nodes
        .iter()
        .map(|node| match &node.address {
            DocumentAddress::Spec(address) => address.without_pin(),
            DocumentAddress::StaticEntry { origin, path } => {
                format!("{origin}:{path}")
            }
        })
        .collect();
    let pending_nodes: Vec<(String, ClosureDocument)> = accepted_nodes
        .into_iter()
        .filter(|address| !existing_keys.contains(&address.without_pin()))
        .map(|address| {
            let key = address.without_pin();
            let document = resolved_document(&address, snapshot)?;
            let DocumentAddress::Spec(observed_address) = document.source().address() else {
                unreachable!("embed snapshot contains only spec-addressed documents")
            };
            Ok((
                key,
                ClosureDocument {
                    address: DocumentAddress::Spec(observed_address.clone()),
                    origin: document_origin(observed_address),
                    tree: document.tree().clone(),
                    aliases: Default::default(),
                },
            ))
        })
        .collect::<Result<_, EmbedPassError>>()?;

    for update in updates {
        closure.nodes[update.node.0].tree = update.tree;
        closure.nodes[update.node.0].aliases = update.aliases;
    }
    for update in simple_updates {
        let ClosureContribution::Simple { document, .. } =
            &mut closure.contributions[update.contribution]
        else {
            unreachable!("the staged simple contribution kept its identity")
        };
        document.tree = update.tree;
        document.aliases = update.aliases;
    }
    let mut node_ids: HashMap<String, ClosureNodeId> = closure
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| match &node.address {
            DocumentAddress::Spec(address) => (address.without_pin(), ClosureNodeId(index)),
            DocumentAddress::StaticEntry { origin, path } => {
                (format!("{origin}:{path}"), ClosureNodeId(index))
            }
        })
        .collect();
    for (key, node) in pending_nodes {
        let id = ClosureNodeId(closure.nodes.len());
        closure.nodes.push(node);
        node_ids.insert(key, id);
    }
    closure.edges.extend(
        accepted_edges
            .into_iter()
            .map(|(from, target)| ClosureEdge {
                from: node_ids[&from],
                to: node_ids[&target.without_pin()],
                kind: ClosureEdgeKind::Embed,
                requested_target: target,
            }),
    );
    closure.pending_embeds = None;
    Ok(())
}

fn finish_expansion(
    expanded: Result<String, EmbedError>,
    missing: Option<String>,
    observed_failure: Option<EmbedError>,
) -> Result<String, EmbedPassError> {
    match expanded {
        Ok(expanded) => Ok(expanded),
        Err(error) => {
            if let Some(addr) = missing {
                return Err(EmbedPassError::MissingObservation { addr });
            }
            if let Some(error) = observed_failure {
                return Err(EmbedPassError::Embed(error));
            }
            Err(EmbedPassError::Embed(error))
        }
    }
}

fn lookup_text(
    address: &SpecAddress,
    snapshot: &super::embed_snapshot::EmbedResolutionSnapshot,
    current: &HashMap<String, DocTree>,
) -> Result<String, EmbedPassError> {
    if let Some(tree) = current.get(&address.without_pin()) {
        return Ok(tree.text(tree.root()));
    }
    let document = resolved_document(address, snapshot)?;
    Ok(document.tree().text(document.tree().root()))
}

fn resolved_document<'a>(
    address: &SpecAddress,
    snapshot: &'a super::embed_snapshot::EmbedResolutionSnapshot,
) -> Result<&'a super::ir::DocumentIr, EmbedPassError> {
    match snapshot.document(address) {
        Some(DocumentObservation::Resolved(document)) => Ok(document),
        Some(DocumentObservation::Failed { reason, .. }) => {
            Err(EmbedPassError::Embed(EmbedError::Unresolved {
                addr: address.to_string(),
                reason: reason.clone(),
            }))
        }
        None => Err(EmbedPassError::MissingObservation {
            addr: address.to_string(),
        }),
    }
}

fn strip_resolved_directives(tree: &DocTree) -> String {
    let strip: HashSet<usize> = tree
        .directives()
        .directives
        .iter()
        .filter(|directive| matches!(directive.kind, DirectiveKind::Use | DirectiveKind::Source))
        .map(|directive| directive.line)
        .collect();
    tree.text(tree.root())
        .lines()
        .enumerate()
        .filter(|(line, _)| !strip.contains(line))
        .map(|(_, text)| text)
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests;
