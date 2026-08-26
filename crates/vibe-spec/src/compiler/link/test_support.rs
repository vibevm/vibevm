use super::*;
use crate::compiler::ir::{
    AbsorptionOccurrence, AbsorptionPlan, ArtifactContext, ClosureDocument, ClosureOccurrence,
    ContributionAbsorption, OriginRename,
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
        seed_address: spec("spec://org.placeholder/pkg/boot/entry#root"),
        emission_order: order
            .iter()
            .map(|node| ClosureOccurrence {
                node: ClosureNodeId(*node),
                requested_address: spec("spec://org.placeholder/pkg/boot/entry#root"),
            })
            .collect(),
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
    mut contributions: Vec<ClosureContribution>,
    renames: Vec<OriginRename>,
) -> ClosureIr {
    for contribution in &mut contributions {
        if let ClosureContribution::Normal {
            seed,
            seed_address,
            emission_order,
            ..
        } = contribution
        {
            *seed_address = spec_address(&nodes, *seed);
            for occurrence in emission_order {
                occurrence.requested_address = spec_address(&nodes, occurrence.node);
            }
        }
    }
    let absorption = contributions
        .iter()
        .map(|contribution| match contribution {
            ClosureContribution::Normal {
                meta,
                seed,
                seed_address,
                emission_order,
            } => ContributionAbsorption::Normal {
                meta: meta.clone(),
                seed: *seed,
                seed_address: seed_address.clone(),
                occurrences: emission_order
                    .iter()
                    .map(|occurrence| AbsorptionOccurrence {
                        node: occurrence.node,
                        requested_address: occurrence.requested_address.clone(),
                        absorbed: false,
                    })
                    .collect(),
            },
            ClosureContribution::Simple { meta, document } => ContributionAbsorption::Simple {
                meta: meta.clone(),
                address: document.address.clone(),
            },
            ClosureContribution::Elided { meta } => {
                ContributionAbsorption::Elided { meta: meta.clone() }
            }
            ClosureContribution::Hoisted { meta, target } => ContributionAbsorption::Hoisted {
                meta: meta.clone(),
                target: target.clone(),
            },
        })
        .collect();
    ClosureIr::testing(
        ArtifactContext::compatibility(mode),
        nodes,
        Vec::new(),
        contributions,
        renames,
        QualificationState::Applied(mode),
        AbsorptionState::Applied(AbsorptionPlan {
            mode,
            contributions: absorption,
        }),
        LinkState::Unlinked,
        None,
        None,
    )
}

pub(super) fn linked_result(closure: &ClosureIr) -> &LinkResult {
    let LinkState::Linked(result) = &closure.link else {
        panic!("expected linked state")
    };
    result
}

pub(super) fn occurrence_bytes(result: &LinkResult) -> Vec<&str> {
    result
        .occurrences
        .iter()
        .map(|occurrence| match occurrence {
            LinkOccurrence::Normal { body, .. } | LinkOccurrence::Simple { body, .. } => {
                body.as_str()
            }
        })
        .collect()
}
