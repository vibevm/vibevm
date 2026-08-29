//! The immutable inter-pass semantic verifier (PROP-054 `##INTER-PASS-VERIFIER`).
//!
//! One pure cell the pass manager consults after a pass has returned
//! successfully and its erased carrier has matched the declared output shape.
//! It never repairs: every entry point takes the IR by shared reference and
//! returns only a typed error naming the pass that produced the invalid
//! carrier (the manager adds that attribution). R3.3 enables it through a
//! `#[cfg(test)]` seam only; production construction stays verifier-off.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#INTER-PASS-VERIFIER");

use std::collections::HashMap;

use crate::doctree::DocTreeInvariantError;
use crate::gate::{DuplicateId, first_duplicate};

use super::assemble::LaneValidationError;
use super::assemble::{LaneShape, validate_shape};
use super::emit::emitted_bytes_digest;
use super::ir::{
    ClosureEdgeKind, ClosureNodeId, DocumentAddress, DocumentIr, DocumentSubject, Documents,
    EmittedIr, LaneChunk, LaneContribution, LaneIr, LaneNode, SourceIr,
};
use super::pass::AnyIr;
use super::qualify::QualifyPassError;
use super::worklist::{DocumentKey, document_key};

mod graph;
mod transition;

/// The moved-field discriminants of a provenance/identity refusal.
///
/// Production only ever renders them, through `TransitionError`; nothing in
/// the compiler classifies on them, and a crate-wide re-export would therefore
/// be dead in a non-test build. Tests match on them exactly.
#[cfg(test)]
pub(crate) use transition::{DocumentIdentityField, LaneProvenanceField};
pub(crate) use transition::{
    LaneWitness, TransitionError, VerificationWitness, lane_witness, verify_lane_transition,
};

/// Why a carrier violates its level's semantic contract.
///
/// A closed typed enum carrying indices, addresses and expected/actual values
/// as data; no caller ever classifies a rendered string. Planned/applied
/// absorption and link/lane failures keep their existing typed sources.
#[derive(Debug, thiserror::Error)]
pub(crate) enum VerificationError {
    #[error("source identity field `{field}` must not be blank")]
    BlankSourceIdentity { field: &'static str },
    #[error(
        "source identity field `{field}` is `{value}`, which is not forward-slashed: a `paths` selector dimension compiles its globs with a literal separator, so a backslashed path matches nothing at all"
    )]
    BackslashedSourcePath { field: &'static str, value: String },
    #[error("document {address} carries a malformed tree: {source}")]
    DocTree {
        address: String,
        #[source]
        source: DocTreeInvariantError,
    },
    #[error("document {address}: {duplicate}")]
    DuplicateId {
        address: String,
        duplicate: DuplicateId,
    },
    #[error(
        "canonical document key {key} appears twice in the gather batch (positions {first} and {second})"
    )]
    DuplicateDocument {
        first: usize,
        second: usize,
        key: String,
    },
    #[error("closure {site} names node {index} outside the graph of {len} nodes")]
    InvalidNodeId {
        site: &'static str,
        index: usize,
        len: usize,
    },
    #[error(
        "closure node {index} is not spec-addressed; static entries live only in simple contributions"
    )]
    NodeAddressKind { index: usize },
    #[error("closure node {second} repeats the canonical key of node {first} ({key})")]
    DuplicateNodeAddress {
        first: usize,
        second: usize,
        key: String,
    },
    #[error(
        "closure node {index} origin `{actual}` disagrees with its address authority `{expected}`"
    )]
    NodeOriginMismatch {
        index: usize,
        expected: String,
        actual: String,
    },
    #[error("closure edge {edge} requests `{expected}` but names a node holding `{actual}`")]
    EdgeTargetMismatch {
        edge: usize,
        expected: String,
        actual: String,
    },
    #[error(
        "normal contribution {contribution} seeds `{expected}` but names a node holding `{actual}`"
    )]
    SeedAddressMismatch {
        contribution: usize,
        expected: String,
        actual: String,
    },
    #[error(
        "contribution {contribution} occurrence {occurrence} requests `{expected}` but names a node holding `{actual}`"
    )]
    OccurrenceAddressMismatch {
        contribution: usize,
        occurrence: usize,
        expected: String,
        actual: String,
    },
    #[error("closure node {index} is not reachable from any normal seed over retained edges")]
    UnreachableNode { index: usize },
    #[error("simple contribution {contribution} does not carry a static entry address")]
    SimpleAddressKind { contribution: usize },
    #[error(
        "simple contribution {contribution} document origin `{actual}` disagrees with `{expected}`"
    )]
    SimpleOriginMismatch {
        contribution: usize,
        expected: String,
        actual: String,
    },
    #[error("illegal {kind:?} cycle in the closure: {}", path.join(" -> "))]
    IllegalCycle {
        kind: ClosureEdgeKind,
        path: Vec<String>,
    },
    #[error("pending qualification requires no produced renames (found {count})")]
    PendingRenames { count: usize },
    #[error(
        "closure typestate is misaligned: {qualification} qualification with {absorption} absorption"
    )]
    MisalignedState {
        qualification: &'static str,
        absorption: &'static str,
    },
    #[error("{kind} snapshots must be consumed before an absorption plan exists")]
    PendingSnapshotsLive { kind: &'static str },
    #[error("planned absorption state is invalid: {source}")]
    AbsorptionPlanned {
        #[source]
        source: Box<QualifyPassError>,
    },
    #[error("applied absorption state is invalid: {source}")]
    AbsorptionApplied {
        #[source]
        source: Box<super::absorb::AbsorbPassError>,
    },
    #[error("linked state is invalid: {source}")]
    LinkReplay {
        #[source]
        source: Box<super::link::LinkPassError>,
    },
    #[error("the pre-pass absorption analysis failed on a verified carrier: {source}")]
    AbsorptionAnalyze {
        #[source]
        source: Box<QualifyPassError>,
    },
    #[error("lane violates its intrinsic contract: {source}")]
    Lane {
        #[source]
        source: Box<LaneValidationError>,
    },
    #[error(
        "lane contribution {contribution} occurrence {occurrence} carries a counterfeit `vibe:begin`/`vibe:end` control line"
    )]
    CounterfeitControlLine {
        contribution: usize,
        occurrence: usize,
    },
    #[error("two distinct lane documents both claim the reversible block key {key}")]
    LaneKeyCollision { key: String },
    #[error(
        "Markdown lane contribution {contribution} ends inside a `{delimiter}` fence of {run}; a fence may carry between occurrences of one contribution, never past its end"
    )]
    ContributionFenceOpen {
        contribution: usize,
        delimiter: char,
        run: usize,
    },
    #[error("emitted provenance identity must not be blank (field: {field})")]
    EmittedIdentityBlank { field: &'static str },
    #[error("emitted bytes do not match their provenance digest")]
    EmittedBytesDigest,
    #[error(transparent)]
    Transition(#[from] TransitionError),
}

/// The verifier itself: stateless and immutable, so a copy is the policy.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct IrVerifier;

impl IrVerifier {
    /// Check the level invariants of one carrier, immutable borrow only.
    pub(crate) fn verify(&self, ir: &AnyIr) -> Result<(), VerificationError> {
        match ir {
            AnyIr::Source(source) => verify_source(source),
            AnyIr::Document(document) => verify_document(document),
            AnyIr::Documents(documents) => verify_documents(documents),
            AnyIr::Closure(closure) => graph::verify_closure(closure),
            AnyIr::Lane(lane) => verify_lane(lane),
            AnyIr::Emitted(emitted) => verify_emitted(emitted),
        }
    }

    /// Derive the immutable transition evidence of a valid pass input.
    pub(crate) fn witness(&self, ir: &AnyIr) -> Result<VerificationWitness, VerificationError> {
        transition::witness(ir)
    }

    /// Authenticate the just-produced value against the pre-pass witness.
    pub(crate) fn verify_transition(
        &self,
        before: &VerificationWitness,
        ir: &AnyIr,
    ) -> Result<(), VerificationError> {
        transition::verify(before, ir)
    }
}

pub(super) fn address_label(address: &DocumentAddress) -> String {
    document_key(address).label()
}

/// Prove one tree's arena shape, then the anchor-namespace gate, in that order.
pub(super) fn verify_tree(address: String, tree: &crate::DocTree) -> Result<(), VerificationError> {
    tree.verify_structure()
        .map_err(|source| VerificationError::DocTree {
            address: address.clone(),
            source,
        })?;
    if let Some(duplicate) = first_duplicate(tree) {
        return Err(VerificationError::DuplicateId { address, duplicate });
    }
    Ok(())
}

fn verify_source(source: &SourceIr) -> Result<(), VerificationError> {
    if source.format().as_str().trim().is_empty() {
        return Err(VerificationError::BlankSourceIdentity { field: "format" });
    }
    // The subject is selector identity, so it obeys the same non-blank law
    // every other identity does — and the wire's scalar gate spells exactly
    // this rule, so a carrier and a live value are held to one contract.
    let declared_path = source.subject().declared_path();
    if declared_path.trim().is_empty() {
        return Err(VerificationError::BlankSourceIdentity {
            field: "subject declared path",
        });
    }
    // The same parity, one law further: the wire refuses a backslashed
    // declared path, so a live one is refused here. This is the boundary a
    // REACHED subject crosses — its path comes from an address rather than
    // from a contribution row, so the artifact plan never saw it.
    if !DocumentSubject::path_is_forward_slashed(declared_path) {
        return Err(VerificationError::BackslashedSourcePath {
            field: "subject declared path",
            value: declared_path.to_string(),
        });
    }
    if let DocumentAddress::StaticEntry { origin, path } = source.address() {
        if origin.trim().is_empty() {
            return Err(VerificationError::BlankSourceIdentity {
                field: "static origin",
            });
        }
        if path.trim().is_empty() {
            return Err(VerificationError::BlankSourceIdentity {
                field: "static path",
            });
        }
    }
    Ok(())
}

fn verify_document(document: &DocumentIr) -> Result<(), VerificationError> {
    verify_source(document.source())?;
    verify_tree(address_label(document.source().address()), document.tree())
}

/// The gather boundary's own law: every document valid, and no canonical key
/// repeated, in vector order. Vector order is authoritative — nothing is
/// sorted, deduplicated, or repaired.
///
/// The key is [`document_key`] itself — the very key `close` and discovery use
/// to index documents — so the guard's collision set is exactly the set of
/// batches those maps would collapse, neither wider nor narrower.
fn verify_documents(documents: &Documents) -> Result<(), VerificationError> {
    let mut first_position: HashMap<DocumentKey, usize> = HashMap::new();
    for (position, document) in documents.iter().enumerate() {
        verify_document(document)?;
        let key = document_key(document.source().address());
        if let Some(first) = first_position.insert(key.clone(), position) {
            return Err(VerificationError::DuplicateDocument {
                first,
                second: position,
                key: key.label(),
            });
        }
    }
    Ok(())
}

/// Marker policy is structured at Lane, so that is where the generic marker
/// law lives; the emitted level carries arbitrary bytes instead (a backend may
/// emit JSON or binary, so no UTF-8 or marker parse is legal there).
fn verify_lane(lane: &LaneIr) -> Result<(), VerificationError> {
    let shape = validate_shape(lane).map_err(|source| VerificationError::Lane {
        source: Box::new(source),
    })?;
    verify_contribution_boundaries(lane, &shape)?;
    let mut namespace: HashMap<String, LaneOwner> = HashMap::new();
    for (contribution, entry) in lane.contributions.iter().enumerate() {
        let chunks = match entry {
            LaneContribution::Normal { chunks, .. } | LaneContribution::Simple { chunks, .. } => {
                chunks
            }
            LaneContribution::Elided { .. } | LaneContribution::Hoisted { .. } => continue,
        };
        for chunk in chunks {
            let LaneChunk::Node(node) = chunk else {
                continue;
            };
            match node.as_ref() {
                LaneNode::Normal {
                    occurrence,
                    node,
                    requested_address,
                    body,
                    fence_before,
                    ..
                } => {
                    let key = requested_address.without_pin();
                    reject_counterfeit(contribution, *occurrence, body, *fence_before, Some(&key))?;
                    claim_key(&mut namespace, key.clone(), LaneOwner::Graph(*node))?;
                    verify_tree(key, &occurrence_tree(body, *fence_before))?;
                }
                LaneNode::Simple {
                    occurrence,
                    address,
                    body,
                    fence_before,
                    ..
                } => {
                    reject_counterfeit(contribution, *occurrence, body, *fence_before, None)?;
                    claim_key(
                        &mut namespace,
                        document_key(address).label(),
                        LaneOwner::Static(document_key(address)),
                    )?;
                    verify_tree(
                        address_label(address),
                        &occurrence_tree(body, *fence_before),
                    )?;
                }
            }
        }
    }
    Ok(())
}

/// Re-read one occurrence body as the document tree it actually is.
///
/// The body is a *fragment* of the emitted lane, and an occurrence may begin
/// inside a fence its predecessor opened. Parsing from the closed state would
/// mint headings and `##<ID>` facts out of fenced code — which the anchor gate
/// would then report as duplicates the document never had. The fence snapshot
/// the lane already carries is the parser's starting state.
fn occurrence_tree(body: &str, fence_before: super::ir::LinkFenceSnapshot) -> crate::DocTree {
    crate::DocTree::parse_from_fence(body, fence_before.markdown())
}

/// Every top-level Markdown contribution boundary is fence-closed.
///
/// Markdown concatenates contribution bodies, so an unbalanced fence is not
/// local: it turns the next contribution, the generated framing and the markers
/// themselves into code. A fence may still carry between the *occurrences* of
/// one normal contribution — the intrinsic walk tracks that per contribution, so
/// only what escapes the contribution is judged here.
///
/// Structured backends (StaticXml, registered custom ones) render lane nodes
/// instead of splicing text, so the same open state is legal for them; the check
/// belongs to the target. And it belongs to the *verifier*, not to
/// `validate_lane`: R3.3's seam is test-only, so production keeps accepting
/// exactly the bytes and errors it accepted before. R6 turns the same seam on
/// unconditionally.
fn verify_contribution_boundaries(
    lane: &LaneIr,
    shape: &LaneShape,
) -> Result<(), VerificationError> {
    if !lane.context().target().is_static_markdown() {
        return Ok(());
    }
    for (contribution, closing) in shape.closing_fences.iter().enumerate() {
        if let super::ir::LinkFenceSnapshot::Open { delimiter, run } = *closing {
            return Err(VerificationError::ContributionFenceOpen {
                contribution,
                delimiter,
                run,
            });
        }
    }
    Ok(())
}

/// Who a reversible block key belongs to. Graph nodes are identified by arena
/// id, static entries by their typed document key.
#[derive(Debug, Clone, PartialEq, Eq)]
enum LaneOwner {
    Graph(ClosureNodeId),
    Static(DocumentKey),
}

/// Reversible-block namespace integrity: one marker key names one document.
///
/// `decompile` recovers blocks by key, so two *distinct* documents emitting the
/// same key make the reverse trip ambiguous — the second block would be read as
/// another slice of the first. Repeated occurrences of the *same* document are
/// exactly the graph-node-versus-emission-occurrence cardinality the closure
/// preserves, so they re-claim their own key and stay legal.
fn claim_key(
    namespace: &mut HashMap<String, LaneOwner>,
    key: String,
    owner: LaneOwner,
) -> Result<(), VerificationError> {
    match namespace.get(&key) {
        Some(established) if *established == owner => Ok(()),
        Some(_) => Err(VerificationError::LaneKeyCollision { key }),
        None => {
            namespace.insert(key, owner);
            Ok(())
        }
    }
}

/// The assembler's framing lines belong to the assembler: a node body carrying
/// a control line of the reversible grammar would be split out as its own block
/// on the reverse trip.
///
/// The reader is [`crate::markers::ControlScanner`] — the exact one `decompile`
/// runs — resumed from this occurrence's own `fence_before`. Two things are
/// refused: any control line no fence hides (the assembler owns that shape), and
/// the one control `decompile` reads *through* a carried fence — the enclosing
/// block's own `close` for a normal occurrence, an `open` for a simple body,
/// which sits between blocks. Prose quoting a marker prefix, and a fenced sample
/// naming somebody else, stay content: `decompile` keeps them in the body, so
/// the verifier may not refuse them.
fn reject_counterfeit(
    contribution: usize,
    occurrence: usize,
    body: &str,
    fence_before: super::ir::LinkFenceSnapshot,
    own_key: Option<&str>,
) -> Result<(), VerificationError> {
    let inside_block = own_key.is_some();
    let mut scanner = crate::markers::ControlScanner::resume(fence_before.markdown(), inside_block);
    // `split('\n')`, exactly the assembler's own fence bookkeeping over a body.
    for line in body.split('\n') {
        let position = scanner.step(line, inside_block);
        let Some(control) = &position.control else {
            continue;
        };
        let structural = match own_key {
            Some(key) => {
                matches!(control, crate::markers::ControlLine::Close(closed) if closed == key)
            }
            None => matches!(control, crate::markers::ControlLine::Open(_)),
        };
        if position.readable() && (structural || !position.fenced()) {
            return Err(VerificationError::CounterfeitControlLine {
                contribution,
                occurrence,
            });
        }
    }
    Ok(())
}

/// Generic emitted law: identity present, bytes authenticated by their
/// provenance digest. The bytes themselves are arbitrary — non-UTF-8 is legal.
fn verify_emitted(emitted: &EmittedIr) -> Result<(), VerificationError> {
    let provenance = emitted.provenance();
    for (field, value) in [
        ("artifact id", provenance.context().artifact().as_str()),
        ("backend id", provenance.backend_id()),
        ("producer pass", provenance.producer()),
    ] {
        if value.trim().is_empty() {
            return Err(VerificationError::EmittedIdentityBlank { field });
        }
    }
    if emitted_bytes_digest(emitted.bytes()) != provenance.bytes_digest {
        return Err(VerificationError::EmittedBytesDigest);
    }
    Ok(())
}

#[cfg(test)]
mod closure_tests;
#[cfg(test)]
mod cycle_tests;
#[cfg(test)]
mod lane_tests;
#[cfg(test)]
mod manager_tests;
#[cfg(test)]
mod markdown_boundary;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod transition_tests;
