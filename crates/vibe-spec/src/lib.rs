//! `vibe-spec` — `spec://` addressing, the document IR, and the deterministic
//! router.
//!
//! This crate implements the resolution layer of PROP-035 (the spec compiler):
//! a `spec://` address is parsed here (§6) and resolved into a node of a
//! document's hierarchical IR (§5). It is a **read-only** consumer of the spec
//! corpus — it never mutates authored files.
//!
//! It deliberately does **not** reuse the vendored `specmark-grammar` parser:
//! that parser rejects both the optional `@version` and the dotted tree-path
//! anchor this grammar introduces, and it is a sync-engines–gated snapshot that
//! must not be edited from the host tree. The flat-anchor kebab rule is
//! reproduced here segment-by-segment so a plain `spec://pkg/doc#anchor` parses
//! byte-identically to the legacy engine.
//!
//! The crate now carries the full router — the address grammar, the document
//! IR, and file resolution — plus the directive scanner (§7). The compilation
//! pipeline (§8) and link tables (§10) build on top of it next.

mod address;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "later R3 phase carriers stay dormant until their built-in passes migrate"
    )
)]
mod compiler;
mod directives;
mod doctree;
mod embed;
mod facts;
mod gate;
mod link_table;
mod markers;
mod merge;
mod pipeline;
mod qualify;
mod resolver;
mod transforms;
mod use_graph;

pub use address::{Authority, SpecAddress, SpecAddressError};
pub use compiler::builtin::{
    ArtifactCompileError, TransformCompileError, compile_artifact, compile_artifact_native,
    compile_artifact_native_observed, compile_artifact_native_traced, compile_artifact_observed,
    compile_artifact_traced,
};
#[cfg(feature = "test-support")]
pub use compiler::builtin::{
    compile_artifact_missing_backend_test_vehicle, compile_artifact_opaque_test_vehicle,
    compile_artifact_replacement_test_vehicle,
};
pub use compiler::ir::{
    ArtifactContext, ArtifactInput, ArtifactInputType, ArtifactInputWitness, ArtifactPlan,
    ArtifactPlanError, ArtifactTarget, DocumentProvider, EmissionProvenance, EmittedArtifact,
    emitted_output_fingerprint,
};
// The R4.1 T10B adapter seam (PROP-054 `#TRANSFORM-PLAN-IDENTITY`): the
// workspace lowers one lane owner's effective compile rows into an
// owner-scoped plan and attaches it to an `ArtifactPlan`. Exactly three names
// cross — the plan VALUE, its refusal, and nothing that can author either.
// The seed, entry, provider, implementation and config values, and every
// digest, stay inside the crate.
pub use compiler::transform::fault::TransformLoweringError;
pub use compiler::transform::native_identity::{
    CompilerNativeImplementationDigest, CompilerNativeImplementationDigestError,
    compiler_native_implementation_digest,
};
pub use compiler::transform::native_manager::{
    CompilerNativeCall, CompilerNativeInvoker, CompilerNativeInvokerError,
    CompilerNativeInvokerErrorKind,
};
pub use compiler::transform::native_policy::{
    CompilerInvocationReceipts, CompilerNativePolicy, CompilerNativePolicyError,
    CompilerPendingRef, CompilerPendingSet,
};
pub use compiler::transform::plan::TransformPlan;
// The observation vocabulary itself is NOT re-exported: status, level,
// cardinality, shape and duration are the generated
// `vibe_wire::generated::compiler_trace_index::e1::index` types, and a second
// spelling of them here is exactly the drift the trace epoch exists to prevent.
pub use compiler::trace::{CompileTraceSink, PassTraceEvent, SnapshotDecision};
// The analyzer observer seam (R4.3): the evidence vocabulary this crate
// owns — witness-derived contribution bytes, occurrence counts, frame
// bytes and the two stage-labelled byte deltas. Every type here is
// `vibe-spec`'s own (the T10 boundary law: no kernel manifest type
// crosses), which is exactly why it MAY cross where the trace epoch's
// generated vocabulary may not.
pub use compiler::observer::{
    CompileObserver, DeltaStage, EmissionContribution, EmissionEvent, EmissionKind, StageDeltaEvent,
};
pub use directives::{Directive, DirectiveError, DirectiveKind, Directives, InPlaceUse};
pub use doctree::{DocTree, Node, NodeId, NodeKind};
pub use embed::{EmbedError, FsSectionSource, SectionSource, expand_embeds};
pub use gate::{DuplicateId, first_duplicate};
pub use link_table::{LinkTable, LinkTableError, build_link_table};
pub use markers::{Block, close, decompile, open};
pub use merge::{
    MergeMode, MergedSection, SectionOrigin, fold_source, fold_sources, merge_contract_source,
};
pub use pipeline::{CompileError, compile_static, compile_static_qualified};
pub use qualify::{RenameEntry, origin_slug, qualify_contribution};
pub use resolver::{
    FileResolver, ResolveError, SelectedPackage, SelfCoordinate, canonical_doc_path, is_pattern,
};
pub use transforms::{XmlMinifyError, minify_emitted_xml};
pub use use_graph::{UseGraphError, source_fold_order, topo_order_from};
