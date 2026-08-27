//! The ONE address inventory of a carrier, and the ONE tree inventory.
//!
//! Gate 2 (scalar identities) and gate 6 (raw reparse) must visit exactly the
//! same `SpecAddress`/`DocumentAddress` set, and gates 7–10 must visit exactly
//! the same `DocTree` set. Writing those walks twice is how two grammars
//! drift, so they are written ONCE here and each phase supplies what it does
//! at every site.

use crate::compiler::wire::{IrWireError, wire};

use super::{closure, documents, emitted, lane};

/// What a phase does at each address site. The walker decides WHICH addresses
/// exist; the visitor decides what the phase asks of one.
pub(super) trait AddressVisitor {
    fn spec(&mut self, value: &wire::SpecAddress) -> Result<(), IrWireError>;

    /// A document identity. Its `Spec` arm's inner address is ALSO handed to
    /// [`AddressVisitor::spec`], so a phase that only cares about `spec://`
    /// values can leave this empty.
    fn document(&mut self, _value: &wire::DocumentAddress) -> Result<(), IrWireError> {
        Ok(())
    }
}

/// Every `DocTree` the carrier holds, in carrier order: top-level document
/// batches, closure nodes, the whole lowered documents of `Simple`
/// contributions, and every resolved pending source/embed observation.
pub(super) fn trees(ir: &wire::Ir) -> Vec<&'_ wire::DocTree> {
    let mut out: Vec<&wire::DocTree> = documents(ir).iter().map(|doc| &doc.tree).collect();
    if let Some(closure) = closure(ir) {
        out.extend(closure.nodes.iter().map(|node| &node.tree));
        for contribution in &closure.contributions {
            if let wire::ClosureContribution::Simple(inner) = contribution {
                out.push(&inner.document.tree);
            }
        }
        if let Some(snapshot) = &closure.pending_sources {
            push_observed(&mut out, &snapshot.documents);
        }
        if let Some(snapshot) = &closure.pending_embeds {
            push_observed(&mut out, &snapshot.documents);
        }
    }
    out
}

fn push_observed<'a>(
    out: &mut Vec<&'a wire::DocTree>,
    documents: &'a std::collections::BTreeMap<String, wire::DocumentObservation>,
) {
    for observation in documents.values() {
        if let wire::DocumentObservation::Resolved(resolved) = observation {
            out.push(&resolved.document.tree);
        }
    }
}

/// Walk EVERY address in the carrier. Adding a wire arm that carries an
/// address means adding it here, once, for every phase at the same time.
pub(super) fn addresses(
    ir: &wire::Ir,
    visitor: &mut dyn AddressVisitor,
) -> Result<(), IrWireError> {
    if let wire::Ir::SourceDocument(arm) = ir {
        document(visitor, &arm.doc.address)?;
    }
    for doc in documents(ir) {
        document(visitor, &doc.source.address)?;
        tree(visitor, &doc.tree)?;
    }
    if let Some(value) = closure(ir) {
        closure_cell(visitor, value)?;
    }
    if let Some(value) = lane(ir) {
        lane_cell(visitor, value)?;
    }
    if let Some(value) = emitted(ir) {
        for witness in &value.provenance.contributions {
            match witness {
                wire::EmissionContributionWitness::Normal(inner) => {
                    visitor.spec(&inner.seed_address)?;
                }
                wire::EmissionContributionWitness::Simple(inner) => {
                    document(visitor, &inner.address)?;
                }
                wire::EmissionContributionWitness::Hoisted(inner) => visitor.spec(&inner.target)?,
                wire::EmissionContributionWitness::Elided(_) => {}
            }
        }
    }
    Ok(())
}

/// A document identity, plus the `spec://` value its `Spec` arm carries.
fn document(
    visitor: &mut dyn AddressVisitor,
    value: &wire::DocumentAddress,
) -> Result<(), IrWireError> {
    visitor.document(value)?;
    if let wire::DocumentAddress::Spec(spec) = value {
        visitor.spec(&spec.address)?;
    }
    Ok(())
}

fn tree(visitor: &mut dyn AddressVisitor, value: &wire::DocTree) -> Result<(), IrWireError> {
    for directive in &value.directives.directives {
        visitor.spec(&directive.address)?;
    }
    for entry in &value.directives.in_place_uses {
        visitor.spec(&entry.address)?;
    }
    for address in value.directives.aliases.values() {
        visitor.spec(address)?;
    }
    Ok(())
}

/// One closed document payload: a graph node, or the lowered document a
/// `Simple` contribution carries outside the graph.
fn closure_document(
    visitor: &mut dyn AddressVisitor,
    value: &wire::ClosureDocument,
) -> Result<(), IrWireError> {
    document(visitor, &value.address)?;
    tree(visitor, &value.tree)?;
    for address in value.aliases.values() {
        visitor.spec(address)?;
    }
    Ok(())
}

fn closure_cell(
    visitor: &mut dyn AddressVisitor,
    value: &wire::ClosureIr,
) -> Result<(), IrWireError> {
    for node in &value.nodes {
        closure_document(visitor, node)?;
    }
    for edge in &value.edges {
        visitor.spec(&edge.requested_target)?;
    }
    for contribution in &value.contributions {
        match contribution {
            wire::ClosureContribution::Normal(inner) => {
                visitor.spec(&inner.seed_address)?;
                for occurrence in &inner.emission_order {
                    visitor.spec(&occurrence.requested_address)?;
                }
            }
            wire::ClosureContribution::Simple(inner) => closure_document(visitor, &inner.document)?,
            wire::ClosureContribution::Hoisted(inner) => visitor.spec(&inner.target)?,
            wire::ClosureContribution::Elided(_) => {}
        }
    }
    if let Some(plan) = super::plan_of(value) {
        for contribution in &plan.contributions {
            match contribution {
                wire::ContributionAbsorption::Normal(inner) => {
                    visitor.spec(&inner.seed_address)?;
                    for occurrence in &inner.occurrences {
                        visitor.spec(&occurrence.requested_address)?;
                    }
                }
                wire::ContributionAbsorption::Simple(inner) => document(visitor, &inner.address)?,
                wire::ContributionAbsorption::Hoisted(inner) => visitor.spec(&inner.target)?,
                wire::ContributionAbsorption::Elided(_) => {}
            }
        }
    }
    if let wire::LinkState::Linked(arm) = &value.link {
        for witness in &arm.result.contributions {
            match witness {
                wire::LinkContributionWitness::Normal(inner) => {
                    visitor.spec(&inner.seed_address)?
                }
                wire::LinkContributionWitness::Simple(inner) => document(visitor, &inner.address)?,
                wire::LinkContributionWitness::Hoisted(inner) => visitor.spec(&inner.target)?,
                wire::LinkContributionWitness::Elided(_) => {}
            }
        }
        for occurrence in &arm.result.occurrences {
            match occurrence {
                wire::LinkOccurrence::Normal(inner) => visitor.spec(&inner.address)?,
                wire::LinkOccurrence::Simple(inner) => document(visitor, &inner.address)?,
            }
        }
    }
    for snapshot in [&value.pending_sources] {
        let Some(snapshot) = snapshot else { continue };
        observed(visitor, &snapshot.documents)?;
        for expansion in snapshot.expansions.values() {
            match expansion {
                wire::ExpansionObservation::Resolved(resolved) => {
                    visitor.spec(&resolved.requested)?;
                    for target in &resolved.targets {
                        visitor.spec(target)?;
                    }
                }
                wire::ExpansionObservation::Failed(failed) => visitor.spec(&failed.requested)?,
            }
        }
    }
    if let Some(snapshot) = &value.pending_embeds {
        observed(visitor, &snapshot.documents)?;
    }
    Ok(())
}

fn observed(
    visitor: &mut dyn AddressVisitor,
    documents: &std::collections::BTreeMap<String, wire::DocumentObservation>,
) -> Result<(), IrWireError> {
    for observation in documents.values() {
        match observation {
            wire::DocumentObservation::Resolved(resolved) => {
                document(visitor, &resolved.document.source.address)?;
                tree(visitor, &resolved.document.tree)?;
            }
            wire::DocumentObservation::Failed(failed) => visitor.spec(&failed.requested)?,
        }
    }
    Ok(())
}

fn lane_cell(visitor: &mut dyn AddressVisitor, value: &wire::LaneIr) -> Result<(), IrWireError> {
    for contribution in &value.contributions {
        let chunks = match contribution {
            wire::LaneContribution::Normal(inner) => {
                visitor.spec(&inner.seed_address)?;
                &inner.chunks
            }
            wire::LaneContribution::Simple(inner) => {
                document(visitor, &inner.address)?;
                &inner.chunks
            }
            wire::LaneContribution::Hoisted(inner) => {
                visitor.spec(&inner.target)?;
                continue;
            }
            wire::LaneContribution::Elided(_) => continue,
        };
        for chunk in chunks {
            if let wire::LaneChunk::Node(inner) = chunk {
                match &inner.node {
                    wire::LaneNode::Normal(node) => visitor.spec(&node.requested_address)?,
                    wire::LaneNode::Simple(node) => document(visitor, &node.address)?,
                }
            }
        }
    }
    Ok(())
}
