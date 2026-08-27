//! Closure conversion: the multi-seed graph cell — context, nodes, edges,
//! contributions, renames, qualification — plus the pass-snapshot and
//! witness-order gates. Typestate (absorption, link) and the pending
//! snapshots live in the child `typestate` cell.

use std::collections::BTreeMap;

use crate::{RenameEntry, SpecAddress};

use self::typestate::{
    decode_absorption_state, decode_link_state, encode_absorption_state, encode_embed_snapshot,
    encode_link_state, encode_source_snapshot,
};
use super::super::ir::{
    ArtifactContext, ArtifactFrame, ArtifactId, ArtifactTarget, BackendId, ClosureContribution,
    ClosureDocument, ClosureEdge, ClosureEdgeKind, ClosureIr, ClosureNodeId, ClosureOccurrence,
    ContributionMeta, OriginRename, QualificationState, validate_package_relation,
};
use super::address::{
    decode_document_address, decode_spec_address, encode_document_address, encode_spec_address,
};
use super::bounded::{display, preview};
use super::tree::{decode_doc_tree, encode_doc_tree};
use super::{
    G_ARENA_BOUNDS, G_CONTEXT_TUPLE, G_ORIGIN_PACKAGE, G_SCALAR_IDS, IrWireError, gate, narrow,
    require_scalar, widen, wire,
};

mod typestate;

pub(super) fn decode_closure(value: &wire::ClosureIr) -> Result<ClosureIr, IrWireError> {
    let context = decode_context(&value.context)?;
    let node_count = value.nodes.len();
    let mut nodes = Vec::with_capacity(node_count);
    for node in &value.nodes {
        nodes.push(decode_closure_document(node)?);
    }
    let mut edges = Vec::with_capacity(value.edges.len());
    for edge in &value.edges {
        let from = narrow("edge source", edge.from)?;
        let to = narrow("edge target", edge.to)?;
        if from >= node_count || to >= node_count {
            return Err(gate(
                G_ARENA_BOUNDS,
                format!("edge {from} -> {to} indexes a graph of {node_count} nodes"),
            ));
        }
        edges.push(ClosureEdge {
            from: ClosureNodeId(from),
            to: ClosureNodeId(to),
            kind: match edge.kind {
                wire::ClosureEdgeKind::Use => ClosureEdgeKind::Use,
                wire::ClosureEdgeKind::Source => ClosureEdgeKind::Source,
                wire::ClosureEdgeKind::Embed => ClosureEdgeKind::Embed,
            },
            requested_target: decode_spec_address(&edge.requested_target)?,
        });
    }
    let mut contributions = Vec::with_capacity(value.contributions.len());
    for contribution in &value.contributions {
        contributions.push(decode_closure_contribution(contribution, node_count)?);
    }
    let mut renames = Vec::with_capacity(value.renames.len());
    for rename in &value.renames {
        renames.push(decode_origin_rename(rename));
    }
    let qualification = match &value.qualification {
        wire::QualificationState::Pending(arm) => {
            QualificationState::Pending(super::decode_mode(&arm.mode))
        }
        wire::QualificationState::Applied(arm) => {
            QualificationState::Applied(super::decode_mode(&arm.mode))
        }
    };
    let absorption = decode_absorption_state(&value.absorption, node_count)?;
    let link = decode_link_state(&value.link, node_count)?;
    let pending_sources = value
        .pending_sources
        .as_ref()
        .map(self::typestate::decode_source_snapshot)
        .transpose()?;
    let pending_embeds = value
        .pending_embeds
        .as_ref()
        .map(self::typestate::decode_embed_snapshot)
        .transpose()?;

    Ok(ClosureIr::from_parts(
        context,
        nodes,
        edges,
        contributions,
        renames,
        qualification,
        absorption,
        link,
        pending_sources,
        pending_embeds,
    ))
}

pub(super) fn decode_context(
    value: &wire::ArtifactContext,
) -> Result<ArtifactContext, IrWireError> {
    require_scalar("artifact id", value.artifact.as_str())?;
    let target = match &value.target {
        wire::ArtifactTarget::StaticMd => ArtifactTarget::StaticMarkdown,
        wire::ArtifactTarget::StaticXml => ArtifactTarget::StaticXml,
        // The open vocabulary is lossless: every valid backend id rides, with
        // no registry lookup and no undocumented refusal.
        wire::ArtifactTarget::Unknown(id) => {
            // The refused id is attacker-sized; name it by bounded preview,
            // never by echoing the constructor's own message.
            let backend = BackendId::new(id.clone()).map_err(|_| {
                gate(
                    G_SCALAR_IDS,
                    format!(
                        "custom target id ({}) is refused by the id charset",
                        preview(id)
                    ),
                )
            })?;
            ArtifactTarget::custom_backend(backend)
        }
    };
    let frame = match &value.frame {
        wire::ArtifactFrame::StaticLane(lane) => {
            require_scalar("generated artifact path", &lane.generated_path)?;
            require_scalar("spec source root", &lane.source_root)?;
            ArtifactFrame::StaticLane {
                generated_path: lane.generated_path.clone(),
                source_root: lane.source_root.clone(),
            }
        }
        wire::ArtifactFrame::CompatibilityFragment(_) => ArtifactFrame::CompatibilityFragment,
    };
    let artifact = ArtifactId::new(value.artifact.clone())
        .map_err(|_| gate(G_SCALAR_IDS, "artifact id must not be blank"))?;
    ArtifactContext::new(artifact, target, frame, super::decode_mode(&value.mode)).map_err(
        |source| {
            gate(
                G_CONTEXT_TUPLE,
                format!(
                    "the id/target/frame/mode tuple is not one row: {}",
                    display(source)
                ),
            )
        },
    )
}

fn decode_closure_document(value: &wire::ClosureDocument) -> Result<ClosureDocument, IrWireError> {
    require_scalar("closure node origin", &value.origin)?;
    let mut aliases = BTreeMap::new();
    for (name, address) in &value.aliases {
        aliases.insert(name.clone(), decode_spec_address(address)?);
    }
    Ok(ClosureDocument {
        address: decode_document_address(&value.address)?,
        origin: value.origin.clone(),
        tree: decode_doc_tree(&value.tree)?,
        aliases,
    })
}

pub(super) fn check_node_id(
    site: &'static str,
    index: usize,
    len: usize,
) -> Result<ClosureNodeId, IrWireError> {
    if index >= len {
        return Err(gate(
            G_ARENA_BOUNDS,
            format!("{site} names node {index} outside the graph of {len} nodes"),
        ));
    }
    Ok(ClosureNodeId(index))
}

fn decode_closure_contribution(
    value: &wire::ClosureContribution,
    node_count: usize,
) -> Result<ClosureContribution, IrWireError> {
    match value {
        wire::ClosureContribution::Normal(normal) => {
            let meta = decode_meta(&normal.meta)?;
            let seed_address = decode_spec_address(&normal.seed_address)?;
            apply_package_relation("normal", &meta, &seed_address, false)?;
            let seed = narrow("normal seed", normal.seed)?;
            check_node_id("normal seed", seed, node_count)?;
            let mut emission_order = Vec::with_capacity(normal.emission_order.len());
            for occurrence in &normal.emission_order {
                let node = narrow("emission occurrence node", occurrence.node)?;
                check_node_id("emission occurrence node", node, node_count)?;
                emission_order.push(ClosureOccurrence {
                    node: ClosureNodeId(node),
                    requested_address: decode_spec_address(&occurrence.requested_address)?,
                });
            }
            Ok(ClosureContribution::Normal {
                meta,
                seed: ClosureNodeId(seed),
                seed_address,
                emission_order,
            })
        }
        wire::ClosureContribution::Simple(simple) => Ok(ClosureContribution::Simple {
            meta: decode_meta(&simple.meta)?,
            document: Box::new(decode_closure_document(&simple.document)?),
        }),
        wire::ClosureContribution::Elided(elided) => Ok(ClosureContribution::Elided {
            meta: decode_meta(&elided.meta)?,
        }),
        wire::ClosureContribution::Hoisted(hoisted) => {
            let meta = decode_meta(&hoisted.meta)?;
            let target = decode_spec_address(&hoisted.target)?;
            apply_package_relation("hoisted", &meta, &target, true)?;
            Ok(ClosureContribution::Hoisted { meta, target })
        }
    }
}

/// The origin/package relation gate: a normal/hoisted contribution's origin
/// coordinate equals its target package coordinate, and a hoisted target is an
/// unversioned whole document.
pub(super) fn apply_package_relation(
    kind: &'static str,
    meta: &ContributionMeta,
    target: &SpecAddress,
    whole_unversioned: bool,
) -> Result<(), IrWireError> {
    validate_package_relation(kind, &meta.origin, target, whole_unversioned).map_err(|source| {
        gate(
            G_ORIGIN_PACKAGE,
            format!(
                "{kind} contribution ({}) contradicts its target: {}",
                preview(&meta.origin),
                display(source)
            ),
        )
    })
}

pub(super) fn decode_meta(value: &wire::ContributionMeta) -> Result<ContributionMeta, IrWireError> {
    require_scalar("contribution origin", &value.origin)?;
    require_scalar("contribution path", &value.path)?;
    ContributionMeta::new(value.origin.clone(), value.path.clone()).map_err(|source| {
        gate(
            G_SCALAR_IDS,
            format!("contribution meta: {}", display(source)),
        )
    })
}

fn decode_origin_rename(value: &wire::OriginRename) -> OriginRename {
    OriginRename {
        origin: value.origin.clone(),
        rename: RenameEntry {
            original: value.rename.original.clone(),
            qualified: value.rename.qualified.clone(),
        },
    }
}

pub(super) fn encode_closure(value: &ClosureIr) -> Result<wire::ClosureIr, IrWireError> {
    let mut nodes = Vec::with_capacity(value.nodes.len());
    for node in &value.nodes {
        nodes.push(encode_closure_document(node)?);
    }
    let mut edges = Vec::with_capacity(value.edges.len());
    for edge in &value.edges {
        edges.push(wire::ClosureEdge {
            from: widen("edge source", edge.from.0)?,
            to: widen("edge target", edge.to.0)?,
            kind: match edge.kind {
                ClosureEdgeKind::Use => wire::ClosureEdgeKind::Use,
                ClosureEdgeKind::Source => wire::ClosureEdgeKind::Source,
                ClosureEdgeKind::Embed => wire::ClosureEdgeKind::Embed,
            },
            requested_target: encode_spec_address(&edge.requested_target),
        });
    }
    let mut contributions = Vec::with_capacity(value.contributions.len());
    for contribution in &value.contributions {
        contributions.push(encode_closure_contribution(contribution)?);
    }
    let renames = value.renames.iter().map(encode_origin_rename).collect();
    let qualification = match value.qualification {
        QualificationState::Pending(mode) => {
            wire::QualificationState::Pending(Box::new(wire::QualificationStatePending {
                mode: super::encode_mode(mode),
            }))
        }
        QualificationState::Applied(mode) => {
            wire::QualificationState::Applied(Box::new(wire::QualificationStateApplied {
                mode: super::encode_mode(mode),
            }))
        }
    };
    Ok(wire::ClosureIr {
        context: encode_context(value.context())?,
        nodes,
        edges,
        contributions,
        renames,
        qualification,
        absorption: encode_absorption_state(&value.absorption)?,
        link: encode_link_state(&value.link)?,
        pending_sources: value
            .pending_sources
            .as_ref()
            .map(encode_source_snapshot)
            .transpose()?,
        pending_embeds: value
            .pending_embeds
            .as_ref()
            .map(encode_embed_snapshot)
            .transpose()?,
    })
}

pub(super) fn encode_context(
    context: &ArtifactContext,
) -> Result<wire::ArtifactContext, IrWireError> {
    let frame = match context.frame() {
        ArtifactFrame::StaticLane {
            generated_path,
            source_root,
        } => wire::ArtifactFrame::StaticLane(Box::new(wire::ArtifactFrameStaticLane {
            generated_path: generated_path.clone(),
            source_root: source_root.clone(),
        })),
        ArtifactFrame::CompatibilityFragment => wire::ArtifactFrame::CompatibilityFragment(
            Box::new(wire::ArtifactFrameCompatibilityFragment {}),
        ),
    };
    let target = match context.target() {
        ArtifactTarget::StaticMarkdown => wire::ArtifactTarget::StaticMd,
        ArtifactTarget::StaticXml => wire::ArtifactTarget::StaticXml,
        target => wire::ArtifactTarget::Unknown(target.backend_id().to_string()),
    };
    Ok(wire::ArtifactContext {
        artifact: context.artifact().as_str().to_string(),
        target,
        frame,
        mode: super::encode_mode(context.mode()),
    })
}

fn encode_closure_document(value: &ClosureDocument) -> Result<wire::ClosureDocument, IrWireError> {
    let mut aliases = BTreeMap::new();
    for (name, address) in &value.aliases {
        aliases.insert(name.clone(), encode_spec_address(address));
    }
    Ok(wire::ClosureDocument {
        address: encode_document_address(&value.address),
        origin: value.origin.clone(),
        tree: encode_doc_tree(&value.tree)?,
        aliases,
    })
}

fn encode_closure_contribution(
    value: &ClosureContribution,
) -> Result<wire::ClosureContribution, IrWireError> {
    Ok(match value {
        ClosureContribution::Normal {
            meta,
            seed,
            seed_address,
            emission_order,
        } => {
            let mut order = Vec::with_capacity(emission_order.len());
            for occurrence in emission_order {
                order.push(wire::ClosureOccurrence {
                    node: widen("emission occurrence node", occurrence.node.0)?,
                    requested_address: encode_spec_address(&occurrence.requested_address),
                });
            }
            wire::ClosureContribution::Normal(Box::new(wire::ClosureContributionNormal {
                meta: encode_meta(meta),
                seed: widen("normal seed", seed.0)?,
                seed_address: encode_spec_address(seed_address),
                emission_order: order,
            }))
        }
        ClosureContribution::Simple { meta, document } => {
            wire::ClosureContribution::Simple(Box::new(wire::ClosureContributionSimple {
                meta: encode_meta(meta),
                document: encode_closure_document(document)?,
            }))
        }
        ClosureContribution::Elided { meta } => {
            wire::ClosureContribution::Elided(Box::new(wire::ClosureContributionElided {
                meta: encode_meta(meta),
            }))
        }
        ClosureContribution::Hoisted { meta, target } => {
            wire::ClosureContribution::Hoisted(Box::new(wire::ClosureContributionHoisted {
                meta: encode_meta(meta),
                target: encode_spec_address(target),
            }))
        }
    })
}

pub(super) fn encode_meta(value: &ContributionMeta) -> wire::ContributionMeta {
    wire::ContributionMeta {
        origin: value.origin.clone(),
        path: value.path.clone(),
    }
}

fn encode_origin_rename(value: &OriginRename) -> wire::OriginRename {
    wire::OriginRename {
        origin: value.origin.clone(),
        rename: wire::RenameEntry {
            original: value.rename.original.clone(),
            qualified: value.rename.qualified.clone(),
        },
    }
}
