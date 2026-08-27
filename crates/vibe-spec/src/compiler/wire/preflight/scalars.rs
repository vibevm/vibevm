//! The scalar-identity phase (gate 2): EVERY identity in the carrier is
//! non-blank and free of CR/LF/NUL — ids, origins, paths, formats, anchors,
//! alias keys, renames, marker keys, frame paths, snapshot keys — and the
//! open `BackendId` vocabulary also matches its charset. Prose (bodies,
//! headings, reasons, directive options, raw text) is not an identity.

use super::super::bounded::preview;
use super::inventory::{AddressVisitor, addresses};
use super::{closure, documents, emitted, lane, source_docs};
use crate::compiler::wire::{G_SCALAR_IDS, IrWireError, gate, require_scalar, wire};

pub(super) fn run(ir: &wire::Ir) -> Result<(), IrWireError> {
    // Every address PIECE is an identity, and it is judged here — before any
    // digest, address-reparse, arena or construction gate. The walk is the
    // shared inventory gate 6 rides, so the two can never diverge.
    addresses(ir, &mut ScalarPieces)?;
    for doc in source_docs(ir) {
        source_doc(doc)?;
    }
    for document in documents(ir) {
        doc_tree(&document.tree)?;
    }
    if let Some(closure) = closure(ir) {
        closure_cell(closure)?;
    }
    if let Some(lane) = lane(ir) {
        lane_cell(lane)?;
    }
    if let Some(emitted) = emitted(ir) {
        emitted_cell(emitted)?;
    }
    Ok(())
}

/// Gate 2 over every address piece: authority coordinates, document path,
/// anchor components, the raw spelling itself, and a static entry's
/// origin/path pair. Prose is never an identity, and neither is a diagnostic
/// `reason` — those may legitimately be blank or multiline.
struct ScalarPieces;

impl AddressVisitor for ScalarPieces {
    fn spec(&mut self, value: &wire::SpecAddress) -> Result<(), IrWireError> {
        match &value.authority {
            wire::Authority::Host(host) => require_scalar("address authority host", &host.name)?,
            wire::Authority::Package(package) => {
                require_scalar("address authority group", &package.group)?;
                require_scalar("address authority name", &package.name)?;
                if let Some(version) = &package.version {
                    require_scalar("address authority version", version)?;
                }
            }
        }
        require_scalar("address doc path", &value.doc_path)?;
        for component in &value.anchor {
            require_scalar("address anchor component", component)?;
        }
        require_scalar("address raw spelling", &value.raw)
    }

    fn document(&mut self, value: &wire::DocumentAddress) -> Result<(), IrWireError> {
        if let wire::DocumentAddress::StaticEntry(entry) = value {
            require_scalar("static entry origin", &entry.origin)?;
            require_scalar("static entry path", &entry.path)?;
        }
        Ok(())
    }
}

fn source_doc(doc: &wire::SourceDoc) -> Result<(), IrWireError> {
    require_scalar("source format", doc.format.as_str())?;
    if let wire::DocumentAddress::StaticEntry(entry) = &doc.address {
        require_scalar("static entry origin", &entry.origin)?;
        require_scalar("static entry path", &entry.path)?;
    }
    Ok(())
}

fn doc_tree(tree: &wire::DocTree) -> Result<(), IrWireError> {
    for node in &tree.nodes {
        if let Some(id) = &node.id {
            require_scalar("node anchor id", id)?;
        }
    }
    for anchor in tree.anchors.keys() {
        require_scalar("anchor name", anchor)?;
    }
    for duplicate in &tree.duplicate_anchors {
        require_scalar("duplicate-anchor name", duplicate)?;
    }
    for name in tree.directives.aliases.keys() {
        require_scalar("directive alias key", name)?;
    }
    Ok(())
}

fn closure_cell(closure: &wire::ClosureIr) -> Result<(), IrWireError> {
    context(&closure.context)?;
    for node in &closure.nodes {
        require_scalar("closure node origin", &node.origin)?;
        if let wire::DocumentAddress::StaticEntry(entry) = &node.address {
            require_scalar("static entry origin", &entry.origin)?;
            require_scalar("static entry path", &entry.path)?;
        }
        doc_tree(&node.tree)?;
        for name in node.aliases.keys() {
            require_scalar("closure alias key", name)?;
        }
    }
    for entry in &closure.contributions {
        contribution(entry)?;
    }
    renames(&closure.renames)?;
    let plan = match &closure.absorption {
        wire::AbsorptionState::Planned(arm) => Some(&arm.plan),
        wire::AbsorptionState::Applied(arm) => Some(&arm.plan),
        wire::AbsorptionState::Unplanned(_) => None,
    };
    if let Some(plan) = plan {
        for entry in &plan.contributions {
            plan_contribution(entry)?;
            if let wire::ContributionAbsorption::Simple(inner) = entry {
                witness_address(&inner.address)?;
            }
        }
    }
    if let wire::LinkState::Linked(arm) = &closure.link {
        for witness in &arm.result.contributions {
            match witness {
                wire::LinkContributionWitness::Normal(inner) => {
                    meta(&inner.meta)?;
                }
                wire::LinkContributionWitness::Simple(inner) => {
                    meta(&inner.meta)?;
                    witness_address(&inner.address)?;
                }
                wire::LinkContributionWitness::Elided(inner) => {
                    meta(&inner.meta)?;
                }
                wire::LinkContributionWitness::Hoisted(inner) => {
                    meta(&inner.meta)?;
                }
            }
        }
        for occurrence in &arm.result.occurrences {
            if let wire::LinkOccurrence::Normal(inner) = occurrence {
                require_scalar("link marker key", inner.marker.as_str())?;
            }
        }
    }
    if let Some(snapshot) = &closure.pending_sources {
        source_snapshot(snapshot)?;
        for key in snapshot.expansions.keys() {
            require_scalar("source expansion key", key)?;
        }
        for observation in snapshot.documents.values() {
            observation_tree(observation)?;
        }
    }
    if let Some(snapshot) = &closure.pending_embeds {
        for key in snapshot.documents.keys() {
            require_scalar("snapshot document key", key)?;
        }
        for entry in &snapshot.discovery_order {
            require_scalar("discovery order entry", entry)?;
        }
        for key in &snapshot.explicit_use_keys {
            require_scalar("explicit use key", key)?;
        }
        for observation in snapshot.documents.values() {
            observation_tree(observation)?;
        }
    }
    Ok(())
}

/// A resolved observation carries a whole document: its tree's identities
/// walk the same scalar law as any carrier tree.
fn observation_tree(observation: &wire::DocumentObservation) -> Result<(), IrWireError> {
    if let wire::DocumentObservation::Resolved(resolved) = observation {
        doc_tree(&resolved.document.tree)?;
    }
    Ok(())
}

/// Every `Simple`-kind witness address: a static entry's origin/path is an
/// identity wherever a simple contribution appears.
fn witness_address(address: &wire::DocumentAddress) -> Result<(), IrWireError> {
    if let wire::DocumentAddress::StaticEntry(entry) = address {
        require_scalar("static entry origin", &entry.origin)?;
        require_scalar("static entry path", &entry.path)?;
    }
    Ok(())
}

fn source_snapshot(snapshot: &wire::SourceResolutionSnapshot) -> Result<(), IrWireError> {
    for key in snapshot.documents.keys() {
        require_scalar("snapshot document key", key)?;
    }
    for entry in &snapshot.discovery_order {
        require_scalar("discovery order entry", entry)?;
    }
    for key in &snapshot.explicit_use_keys {
        require_scalar("explicit use key", key)?;
    }
    Ok(())
}

fn contribution(contribution: &wire::ClosureContribution) -> Result<(), IrWireError> {
    match contribution {
        wire::ClosureContribution::Normal(inner) => meta(&inner.meta)?,
        wire::ClosureContribution::Simple(inner) => {
            meta(&inner.meta)?;
            doc_tree(&inner.document.tree)?;
            require_scalar("closure node origin", &inner.document.origin)?;
            for name in inner.document.aliases.keys() {
                require_scalar("closure alias key", name)?;
            }
        }
        wire::ClosureContribution::Elided(inner) => meta(&inner.meta)?,
        wire::ClosureContribution::Hoisted(inner) => meta(&inner.meta)?,
    }
    Ok(())
}

fn plan_contribution(contribution: &wire::ContributionAbsorption) -> Result<(), IrWireError> {
    match contribution {
        wire::ContributionAbsorption::Normal(inner) => meta(&inner.meta)?,
        wire::ContributionAbsorption::Simple(inner) => meta(&inner.meta)?,
        wire::ContributionAbsorption::Elided(inner) => meta(&inner.meta)?,
        wire::ContributionAbsorption::Hoisted(inner) => meta(&inner.meta)?,
    }
    Ok(())
}

fn meta(value: &wire::ContributionMeta) -> Result<(), IrWireError> {
    require_scalar("contribution origin", &value.origin)?;
    require_scalar("contribution path", &value.path)?;
    Ok(())
}

pub(super) fn renames(values: &[wire::OriginRename]) -> Result<(), IrWireError> {
    for rename in values {
        require_scalar("rename origin", rename.origin.as_str())?;
        require_scalar("rename original", rename.rename.original.as_str())?;
        require_scalar("rename qualified", rename.rename.qualified.as_str())?;
    }
    Ok(())
}

pub(super) fn context(context: &wire::ArtifactContext) -> Result<(), IrWireError> {
    require_scalar("artifact id", context.artifact.as_str())?;
    if let wire::ArtifactTarget::Unknown(id) = &context.target {
        require_scalar("custom target id", id.as_str())?;
        backend_id_charset(id)?;
    }
    if let wire::ArtifactFrame::StaticLane(frame) = &context.frame {
        require_scalar("generated artifact path", &frame.generated_path)?;
        require_scalar("spec source root", &frame.source_root)?;
    }
    Ok(())
}

/// The open target vocabulary's one charset law, same as `BackendId::new`.
fn backend_id_charset(id: &str) -> Result<(), IrWireError> {
    let bytes = id.as_bytes();
    let valid = (1..=64).contains(&bytes.len())
        && id_byte(bytes[0])
        && bytes
            .iter()
            .skip(1)
            .all(|byte| id_byte(*byte) || b"._-".contains(byte));
    if valid {
        Ok(())
    } else {
        Err(gate(
            G_SCALAR_IDS,
            format!("backend id ({}) is refused by the id charset", preview(id)),
        ))
    }
}

fn id_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit()
}

fn lane_cell(lane: &wire::LaneIr) -> Result<(), IrWireError> {
    context(&lane.context)?;
    if let Some(path) = &lane.frame.generated_path {
        require_scalar("lane generated path", path)?;
    }
    if let Some(root) = &lane.frame.source_root {
        require_scalar("lane source root", root)?;
    }
    renames(&lane.frame.renames)?;
    for contribution in &lane.contributions {
        match contribution {
            wire::LaneContribution::Normal(inner) => {
                meta(&inner.meta)?;
                for entry in &inner.chunks {
                    chunk(entry)?;
                }
            }
            wire::LaneContribution::Simple(inner) => {
                meta(&inner.meta)?;
                for entry in &inner.chunks {
                    chunk(entry)?;
                }
            }
            wire::LaneContribution::Elided(inner) => meta(&inner.meta)?,
            wire::LaneContribution::Hoisted(inner) => meta(&inner.meta)?,
        }
    }
    Ok(())
}

fn chunk(chunk: &wire::LaneChunk) -> Result<(), IrWireError> {
    match chunk {
        wire::LaneChunk::NormalOpen(inner) => {
            require_scalar("lane marker key", inner.marker.as_str())
        }
        wire::LaneChunk::NormalClose(inner) => {
            require_scalar("lane marker key", inner.marker.as_str())
        }
        wire::LaneChunk::Node(inner) => lane_node(&inner.node),
        wire::LaneChunk::ForcedNewline(_) => Ok(()),
    }
}

fn lane_node(node: &wire::LaneNode) -> Result<(), IrWireError> {
    match node {
        wire::LaneNode::Normal(inner) => {
            require_scalar("lane node origin", &inner.origin)?;
            require_scalar("lane marker key", inner.marker.as_str())
        }
        wire::LaneNode::Simple(inner) => require_scalar("lane node origin", &inner.origin),
    }
}

fn emitted_cell(emitted: &wire::EmittedArtifact) -> Result<(), IrWireError> {
    let provenance = &emitted.provenance;
    context(&provenance.context)?;
    require_scalar("emit backend id", provenance.backend.as_str())?;
    backend_id_charset(provenance.backend.as_str())?;
    require_scalar("emit producer pass", provenance.producer.as_str())?;
    renames(&provenance.renames)?;
    for witness in &provenance.contributions {
        match witness {
            wire::EmissionContributionWitness::Normal(inner) => meta(&inner.meta)?,
            wire::EmissionContributionWitness::Simple(inner) => {
                meta(&inner.meta)?;
                witness_address(&inner.address)?;
            }
            wire::EmissionContributionWitness::Elided(inner) => meta(&inner.meta)?,
            wire::EmissionContributionWitness::Hoisted(inner) => meta(&inner.meta)?,
        }
    }
    Ok(())
}
