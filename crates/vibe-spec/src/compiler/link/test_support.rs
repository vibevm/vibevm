use super::*;
use crate::compiler::ir::{
    AbsorptionOccurrence, AbsorptionPlan, ArtifactId, ClosureDocument, ContributionAbsorption,
};
use crate::{DocTree, RenameEntry, SpecAddress};

pub(super) fn spec(raw: &str) -> SpecAddress {
    SpecAddress::parse(raw).unwrap()
}

pub(super) fn normal_node(raw: &str, origin: &str, body: &str) -> ClosureDocument {
    ClosureDocument {
        address: DocumentAddress::Spec(spec(raw)),
        origin: origin.to_string(),
        tree: DocTree::parse(body),
        aliases: Default::default(),
    }
}

pub(super) fn simple_node(origin: &str, path: &str, body: &str) -> ClosureDocument {
    ClosureDocument {
        address: DocumentAddress::StaticEntry {
            origin: origin.to_string(),
            path: path.to_string(),
        },
        origin: origin.to_string(),
        tree: DocTree::parse(body),
        aliases: Default::default(),
    }
}

pub(super) fn meta(origin: &str, path: &str) -> ContributionMeta {
    ContributionMeta {
        origin: origin.to_string(),
        path: path.to_string(),
    }
}

pub(super) fn normal(
    origin: &str,
    path: &str,
    seed: usize,
    order: &[usize],
) -> ClosureContribution {
    ClosureContribution::Normal {
        meta: meta(origin, path),
        seed: ClosureNodeId(seed),
        emission_order: order.iter().copied().map(ClosureNodeId).collect(),
    }
}

pub(super) fn simple(origin: &str, path: &str, body: &str) -> ClosureContribution {
    ClosureContribution::Simple {
        meta: meta(origin, path),
        document: Box::new(simple_node(origin, path, body)),
    }
}

pub(super) fn rename(origin: &str, original: &str, qualified: &str) -> OriginRename {
    OriginRename {
        origin: origin.to_string(),
        rename: RenameEntry {
            original: original.to_string(),
            qualified: qualified.to_string(),
        },
    }
}

fn spec_address(nodes: &[ClosureDocument], id: ClosureNodeId) -> SpecAddress {
    let DocumentAddress::Spec(address) = &nodes[id.0].address else {
        panic!("normal fixture node must be spec-addressed")
    };
    address.clone()
}

pub(super) fn closure(
    mode: StaticCompileMode,
    nodes: Vec<ClosureDocument>,
    contributions: Vec<ClosureContribution>,
    renames: Vec<OriginRename>,
) -> ClosureIr {
    let absorption = contributions
        .iter()
        .map(|contribution| match contribution {
            ClosureContribution::Normal {
                meta,
                seed,
                emission_order,
            } => ContributionAbsorption::Normal {
                meta: meta.clone(),
                seed: *seed,
                seed_address: spec_address(&nodes, *seed),
                occurrences: emission_order
                    .iter()
                    .map(|node| AbsorptionOccurrence {
                        node: *node,
                        address: spec_address(&nodes, *node),
                        absorbed: false,
                    })
                    .collect(),
            },
            ClosureContribution::Simple { meta, document } => ContributionAbsorption::Simple {
                meta: meta.clone(),
                address: document.address.clone(),
            },
        })
        .collect();
    ClosureIr {
        artifact: ArtifactId::new("link-test").unwrap(),
        nodes,
        edges: Vec::new(),
        contributions,
        renames,
        qualification: QualificationState::Applied(mode),
        absorption: AbsorptionState::Applied(AbsorptionPlan {
            mode,
            contributions: absorption,
        }),
        link: LinkState::Unlinked,
        pending_sources: None,
        pending_embeds: None,
    }
}

pub(super) fn linked_result(closure: &ClosureIr) -> &LinkResult {
    let LinkState::Linked(result) = &closure.link else {
        panic!("expected linked state")
    };
    result
}

pub(super) fn occurrence_bytes(result: &LinkResult) -> Vec<&str> {
    result
        .chunks
        .iter()
        .filter_map(|chunk| match chunk {
            LinkChunk::NormalOccurrence { bytes, .. }
            | LinkChunk::SimpleOccurrence { bytes, .. } => Some(bytes.as_str()),
            LinkChunk::Literal { .. } => None,
        })
        .collect()
}
