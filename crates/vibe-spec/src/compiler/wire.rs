//! Strict, lossless conversion between the R3 domain IR and the generated
//! epoch-1 compiler-IR wire (PROP-054 `##WHOLE-IR-WIRE`).
//!
//! One compiler value has exactly one machine projection: both encoders build
//! the SAME generated `vibe_wire` value (pretty versus compact is a serializer
//! choice), and decode parses straight into that strict type before the named
//! conversion gates run in the architecture order — malformed indices never
//! reach an index, a slice, or an allocation sized from unchecked data. There
//! is no serde derive on the domain IR, no trace DTO, no JSON-value carrier,
//! and no second wire shape.
//!
//! The byte API stays crate-private in this atom; R3.4 trace and the R6.3
//! native ABI become its consumers later.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#WHOLE-IR-WIRE");

use super::ir::{Documents, StaticCompileMode};
use super::pass::AnyIr;
use super::verify::{IrVerifier, VerificationError};

mod address;
/// Visible to `compiler` so the R3.4 observer renders its diagnostics through
/// this ONE reviewed bounded sink rather than growing a second, unreviewed
/// formatter beside it.
pub(super) mod bounded;
mod closure;
mod emitted;
mod framing;
mod json;
mod lane;
mod preflight;
mod staged;
mod tree;

use vibe_wire::generated::compiler_ir::e1::ir as wire;

#[cfg(test)]
mod tests;

/// One named conversion gate, pinned to the schema entry it implements.
pub(crate) struct GateSpec {
    pub(crate) label: &'static str,
    /// A substring unique to the gate's own entry in the schema's
    /// `x-conversion-gates` list, so the registry and the schema cannot drift
    /// apart silently.
    pub(crate) probe: &'static str,
}

/// The implemented gate registry, in `schemas/compiler_ir/e1/ir.jtd.json`
/// `x-conversion-gates` order. A test pins this list against the schema: an
/// undocumented gate and an unimplemented named gate are both red.
pub(crate) const CONVERSION_GATES: [GateSpec; 15] = [
    GateSpec {
        label: "ir-schema",
        probe: "ir_schema == 1",
    },
    GateSpec {
        label: "scalar-ids",
        probe: "non-blank and free of newline/NUL",
    },
    GateSpec {
        label: "context-tuple",
        probe: "one `ArtifactContext::new` row",
    },
    GateSpec {
        label: "origin-package-relation",
        probe: "origin coordinate equals its target package coordinate",
    },
    GateSpec {
        label: "digest-base64-canonical",
        probe: "64 lowercase hex characters",
    },
    GateSpec {
        label: "address-reparse",
        probe: "`raw` re-parses to the authority/doc_path/anchor/pinned_r",
    },
    GateSpec {
        label: "arena-bounds",
        probe: "ARENA BOUNDS",
    },
    GateSpec {
        label: "forest",
        probe: "FOREST, checked iteratively",
    },
    GateSpec {
        label: "span-bounds",
        probe: "SPAN BOUNDS",
    },
    GateSpec {
        label: "anchor-coherence",
        probe: "names a node whose `id` is `a`",
    },
    GateSpec {
        label: "set-projection",
        probe: "SET PROJECTION",
    },
    GateSpec {
        label: "absorption-witness",
        probe: "qualification and absorption typestate align",
    },
    GateSpec {
        label: "link-witness-lane",
        probe: "and a lane brackets each normal occurrence",
    },
    GateSpec {
        label: "pass-snapshot",
        probe: "PASS/SNAPSHOT",
    },
    GateSpec {
        label: "emit-identity",
        probe: "EMIT IDENTITY",
    },
];

/// Registry-tied labels, so a gate error can only name an implemented gate.
pub(super) const G_SCALAR_IDS: &str = CONVERSION_GATES[1].label;
pub(super) const G_CONTEXT_TUPLE: &str = CONVERSION_GATES[2].label;
pub(super) const G_ORIGIN_PACKAGE: &str = CONVERSION_GATES[3].label;
pub(super) const G_DIGEST_BASE64: &str = CONVERSION_GATES[4].label;
pub(super) const G_ADDRESS_REPARSE: &str = CONVERSION_GATES[5].label;
pub(super) const G_ARENA_BOUNDS: &str = CONVERSION_GATES[6].label;
pub(super) const G_FOREST: &str = CONVERSION_GATES[7].label;
pub(super) const G_SPAN_BOUNDS: &str = CONVERSION_GATES[8].label;
pub(super) const G_ANCHOR_COHERENCE: &str = CONVERSION_GATES[9].label;
pub(super) const G_SET_PROJECTION: &str = CONVERSION_GATES[10].label;
pub(super) const G_ABSORPTION_WITNESS: &str = CONVERSION_GATES[11].label;
pub(super) const G_LINK_WITNESS_LANE: &str = CONVERSION_GATES[12].label;
pub(super) const G_PASS_SNAPSHOT: &str = CONVERSION_GATES[13].label;
pub(super) const G_EMIT_IDENTITY: &str = CONVERSION_GATES[14].label;

/// Why a carrier cannot cross the wire boundary.
#[derive(Debug, thiserror::Error)]
pub(crate) enum IrWireError {
    /// serde's own `unknown field` / type messages quote the offending input,
    /// so the detail is BOUNDED at construction and the unbounded source is
    /// not retained — a refusal must never grow with the carrier that caused
    /// it, however it is later rendered or chained.
    #[error("the strict generated reader rejected the carrier: {detail}")]
    Reader { detail: String },
    #[error("the strict byte reader rejected the carrier: {detail}")]
    StrictReader { detail: String },
    #[error("ir_schema must be 1 in epoch 1, got {0}")]
    Schema(u32),
    #[error("conversion gate `{gate}` refused the carrier: {detail}")]
    Gate { gate: &'static str, detail: String },
    #[error("the domain constructor refused the carrier: {0}")]
    Construction(String),
    #[error("the immutable verifier refused the reconstructed carrier: {0}")]
    Verification(String),
    #[error("the domain value cannot ride the epoch-1 wire: {0}")]
    Encode(String),
}

pub(super) fn gate(label: &'static str, detail: impl Into<String>) -> IrWireError {
    IrWireError::Gate {
        gate: label,
        detail: detail.into(),
    }
}

pub(super) fn construction(detail: impl Into<String>) -> IrWireError {
    IrWireError::Construction(detail.into())
}

/// A scalar id law: non-blank, and free of CR/LF/NUL (the gate the schema
/// names for ids, origins, paths, formats and pass names).
pub(super) fn require_scalar(field: &'static str, value: &str) -> Result<(), IrWireError> {
    if value.trim().is_empty() {
        return Err(gate(G_SCALAR_IDS, format!("{field} must not be blank")));
    }
    if value.contains(['\n', '\r', '\0']) {
        return Err(gate(
            G_SCALAR_IDS,
            format!("{field} must not contain a newline or NUL"),
        ));
    }
    Ok(())
}

/// The compatibility/static-lane policy crossing both directions.
pub(super) fn decode_mode(value: &wire::CompileMode) -> StaticCompileMode {
    match value {
        wire::CompileMode::Plain => StaticCompileMode::Plain,
        wire::CompileMode::QualifyPerNode => StaticCompileMode::QualifyPerNode,
    }
}

pub(super) fn encode_mode(value: StaticCompileMode) -> wire::CompileMode {
    match value {
        StaticCompileMode::Plain => wire::CompileMode::Plain,
        StaticCompileMode::QualifyPerNode => wire::CompileMode::QualifyPerNode,
    }
}

/// Checked wire→domain index narrowing; total, never truncating.
pub(super) fn narrow(field: &'static str, value: u32) -> Result<usize, IrWireError> {
    usize::try_from(value).map_err(|_| gate(G_ARENA_BOUNDS, format!("{field} overflows usize")))
}

/// Checked domain→wire index widening; a domain value that cannot fit epoch 1
/// refuses rather than truncating.
pub(super) fn widen(field: &'static str, value: usize) -> Result<u32, IrWireError> {
    u32::try_from(value)
        .map_err(|_| IrWireError::Encode(format!("{field} {value} does not fit a u32 index")))
}

/// Decode wire bytes into one verified domain carrier.
pub(crate) fn decode(bytes: &[u8]) -> Result<AnyIr, IrWireError> {
    let ir = convert(bytes)?;
    IrVerifier
        .verify(&ir)
        .map_err(|source| verification(&source))?;
    Ok(ir)
}

/// The typed conversion without the post-construction verifier pass, so tests
/// can separate "the conversion is faithful" from "the carrier is
/// verifier-valid" (the byte API always runs both).
#[cfg(test)]
pub(crate) fn decode_unverified(bytes: &[u8]) -> Result<AnyIr, IrWireError> {
    convert(bytes)
}

/// Every named conversion gate, in schema order, and nothing else: the strict
/// reader, the ordered phases (1–11), construction (which carries gate 15 at
/// the emitted arm), then the staged replays (12, 13, 14).
fn convert(bytes: &[u8]) -> Result<AnyIr, IrWireError> {
    let value: wire::Ir = json::from_strict_slice(bytes)?;
    preflight::run(&value)?;
    let ir = decode_carrier(&value)?;
    staged::run(&value, &ir)?;
    Ok(ir)
}

/// Encode one domain carrier as compact wire JSON (the native ABI spelling).
pub(crate) fn encode_compact(ir: &AnyIr) -> Result<Vec<u8>, IrWireError> {
    let value = encode_carrier(ir)?;
    serde_json::to_vec(&value).map_err(|source| IrWireError::Encode(source.to_string()))
}

/// Encode the same generated value as pretty wire JSON (the trace spelling).
pub(crate) fn encode_pretty(ir: &AnyIr) -> Result<Vec<u8>, IrWireError> {
    let value = encode_carrier(ir)?;
    serde_json::to_vec_pretty(&value).map_err(|source| IrWireError::Encode(source.to_string()))
}

/// A verifier refusal, rendered through the BOUNDED `Debug` sink.
///
/// Verifier variants carry hostile strings — a node origin, a duplicate id, a
/// whole cycle path — and some of their `Display` impls build the text before
/// the formatter sees it (`path.join(" -> ")`). Rendering the typed value's
/// derived `Debug` through the sink keeps the variant family and a bounded
/// preview without ever asking `Display` for the full string, and the
/// unbounded `VerificationError` is not retained inside `IrWireError`.
fn verification(source: &VerificationError) -> IrWireError {
    IrWireError::Verification(bounded::debug(source))
}

fn require_schema(value: u32) -> Result<(), IrWireError> {
    if value == 1 {
        Ok(())
    } else {
        Err(IrWireError::Schema(value))
    }
}

fn decode_carrier(value: &wire::Ir) -> Result<AnyIr, IrWireError> {
    match value {
        wire::Ir::SourceDocument(arm) => {
            require_schema(arm.ir_schema)?;
            belt(
                matches!(arm.level, wire::LevelSource::Source),
                matches!(arm.cardinality, wire::CardinalityDocument::Document),
            );
            Ok(AnyIr::Source(address::decode_source_doc(&arm.doc)?))
        }
        wire::Ir::DocumentDocument(arm) => {
            require_schema(arm.ir_schema)?;
            belt(
                matches!(arm.level, wire::LevelDocument::Document),
                matches!(arm.cardinality, wire::CardinalityDocument::Document),
            );
            Ok(AnyIr::Document(tree::decode_document_ir(&arm.doc)?))
        }
        wire::Ir::DocumentsArtifact(arm) => {
            require_schema(arm.ir_schema)?;
            belt(
                matches!(arm.level, wire::LevelDocument::Document),
                matches!(arm.cardinality, wire::CardinalityArtifact::Artifact),
            );
            let mut documents = Vec::with_capacity(arm.documents.len());
            for document in &arm.documents {
                documents.push(tree::decode_document_ir(document)?);
            }
            Ok(AnyIr::Documents(Documents::new(documents)))
        }
        wire::Ir::ClosureArtifact(arm) => {
            require_schema(arm.ir_schema)?;
            belt(
                matches!(arm.level, wire::LevelClosure::Closure),
                matches!(arm.cardinality, wire::CardinalityArtifact::Artifact),
            );
            Ok(AnyIr::Closure(closure::decode_closure(&arm.closure)?))
        }
        wire::Ir::LaneArtifact(arm) => {
            require_schema(arm.ir_schema)?;
            belt(
                matches!(arm.level, wire::LevelLane::Lane),
                matches!(arm.cardinality, wire::CardinalityArtifact::Artifact),
            );
            Ok(AnyIr::Lane(lane::decode_lane(&arm.lane)?))
        }
        wire::Ir::EmittedArtifact(arm) => {
            require_schema(arm.ir_schema)?;
            belt(
                matches!(arm.level, wire::LevelEmitted::Emitted),
                matches!(arm.cardinality, wire::CardinalityArtifact::Artifact),
            );
            Ok(AnyIr::Emitted(emitted::decode_emitted(&arm.emitted)?))
        }
    }
}

/// The level/cardinality redundancy is single-value enums, so the reader is
/// already red on a mismatch; the belt keeps a swapped arm a typed fault.
fn belt(level: bool, cardinality: bool) {
    debug_assert!(
        level && cardinality,
        "the strict reader guarantees the level/cardinality belt"
    );
}

fn encode_carrier(ir: &AnyIr) -> Result<wire::Ir, IrWireError> {
    let value = match ir {
        AnyIr::Source(source) => wire::Ir::SourceDocument(Box::new(wire::IrSourceDocument {
            ir_schema: 1,
            level: wire::LevelSource::Source,
            cardinality: wire::CardinalityDocument::Document,
            doc: address::encode_source_doc(source),
        })),
        AnyIr::Document(document) => {
            wire::Ir::DocumentDocument(Box::new(wire::IrDocumentDocument {
                ir_schema: 1,
                level: wire::LevelDocument::Document,
                cardinality: wire::CardinalityDocument::Document,
                doc: tree::encode_document_ir(document)?,
            }))
        }
        AnyIr::Documents(documents) => {
            let mut batch = Vec::with_capacity(documents.len());
            for document in documents {
                batch.push(tree::encode_document_ir(document)?);
            }
            wire::Ir::DocumentsArtifact(Box::new(wire::IrDocumentsArtifact {
                ir_schema: 1,
                level: wire::LevelDocument::Document,
                cardinality: wire::CardinalityArtifact::Artifact,
                documents: batch,
            }))
        }
        AnyIr::Closure(closure) => wire::Ir::ClosureArtifact(Box::new(wire::IrClosureArtifact {
            ir_schema: 1,
            level: wire::LevelClosure::Closure,
            cardinality: wire::CardinalityArtifact::Artifact,
            closure: closure::encode_closure(closure)?,
        })),
        AnyIr::Lane(lane) => wire::Ir::LaneArtifact(Box::new(wire::IrLaneArtifact {
            ir_schema: 1,
            level: wire::LevelLane::Lane,
            cardinality: wire::CardinalityArtifact::Artifact,
            lane: lane::encode_lane(lane)?,
        })),
        AnyIr::Emitted(emitted) => wire::Ir::EmittedArtifact(Box::new(wire::IrEmittedArtifact {
            ir_schema: 1,
            level: wire::LevelEmitted::Emitted,
            cardinality: wire::CardinalityArtifact::Artifact,
            emitted: emitted::encode_emitted(emitted)?,
        })),
    };
    Ok(value)
}
