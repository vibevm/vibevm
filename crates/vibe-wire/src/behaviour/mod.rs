//! The hand-written behaviour layer that lives beside the generated
//! form.
//!
//! The tree under [`crate::generated`] owns the SHAPES — fields,
//! derives, serde — and this module owns what a shape cannot say
//! about itself: the wire strings of a vocabulary and their laws,
//! emptiness, zero values, the schema constant, the fixture builder,
//! and the `finalise` passes that make a record byte-deterministic.
//! It lives in this crate because the orphan rule leaves no
//! alternative: an inherent `impl` belongs in the crate that defines
//! the type, and the consumers re-export these types instead of
//! duplicating them, so that crate is this one. The layer is split by
//! the same seam the generated side already has: vocabularies,
//! per-record projections, the records and aggregates themselves, and
//! the serde boundary helpers whose policy the generated shape cannot
//! express.
//!
//! Nothing here is generated, and nothing here edits the generated
//! files.

pub mod artifact_record;
pub mod compile_trace_report;
pub mod compiler_trace_index;
pub mod deploy_records;
pub mod extensions_analyze;
pub mod projections;
pub mod records;
pub(crate) mod required_nullable;
pub mod requirements_report;
pub(crate) mod scalars;
pub mod verification_evidence;
pub mod vocabularies;
