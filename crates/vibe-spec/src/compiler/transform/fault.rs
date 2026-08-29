//! The one typed transform fault family (R4-TRANSFORM-PLAN-ABI §6.3): every
//! resolution, schedule, capability, behavior, selector-admission,
//! lane-admission and verifier-attributed fault a planned transform can
//! raise, plus the opaque public [`TransformCompileError`] that carries one
//! out of the crate.
//!
//! Split out of the schedule cell when T8 landed selector evaluation. The
//! cell that WRAPS behaviors and the family that NAMES their faults are two
//! different jobs: the fault family is the only part of the pair the builtin
//! attribution cell above is allowed to name, and it is pure data — nothing
//! here resolves, executes, or holds a behavior.
//!
//! Every per-entry fault carries the same three identity members — the
//! bounded key preview, the dense plan order and the stage — so no failure
//! ever reconstructs an entry's identity by parsing a rendered pass name.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY");

use std::fmt;

use crate::compiler::assemble::LaneValidationError;
use crate::compiler::pass::{PassName, PassNameError};
use crate::compiler::pipeline::CompilerPipelineError;
use crate::compiler::verify::{TransitionError, VerificationError};

use super::behavior::TransformBehaviorError;
use super::config_lowering::ConfigLoweringGap;
use super::plan::TransformStage;
use super::plan_validate::{BoundedPreview, TransformPlanError};
use super::registry::TransformRegistryError;
use super::selector_admission::SelectorAdmissionError;

/// Why one planned transform refused at construction or execution.
///
/// Typed end to end: the registry, behavior, selector, pass-name and pipeline
/// refusals ride along as their exact types, and every entry fault names the
/// bounded key preview, the dense order and the stage — never a re-parsed
/// pass name.
#[derive(Debug, thiserror::Error)]
pub(crate) enum TransformError {
    #[error(
        "a nonempty transform plan ({entries} entries) cannot execute on a compatibility-fragment artifact"
    )]
    CompatibilityFragmentPlan { entries: u32 },
    #[error("transform entry {order} (`{preview}` at {stage:?}) did not resolve: {source}")]
    Resolution {
        preview: BoundedPreview,
        order: u32,
        stage: TransformStage,
        #[source]
        source: TransformRegistryError,
    },
    #[error("transform entry {order} (`{preview}` at {stage:?}) refused: {gap}")]
    Capability {
        preview: BoundedPreview,
        order: u32,
        stage: TransformStage,
        gap: TransformCapabilityGap,
    },
    #[error(
        "transform entry {order} (`{preview}` at {stage:?}) cannot judge one document against its selector: {source}"
    )]
    Selector {
        preview: BoundedPreview,
        order: u32,
        stage: TransformStage,
        #[source]
        source: SelectorAdmissionError,
    },
    #[error(
        "transform entry {order} (`{preview}` at {stage:?}) could not enter the compiler pipeline: {source}"
    )]
    Schedule {
        preview: BoundedPreview,
        order: u32,
        stage: TransformStage,
        #[source]
        source: CompilerPipelineError,
    },
    #[error("transform entry {order} (`{preview}` at {stage:?}) has no valid pass name: {source}")]
    Name {
        preview: BoundedPreview,
        order: u32,
        stage: TransformStage,
        #[source]
        source: PassNameError,
    },
    #[error("transform entry {order} (`{preview}` at {stage:?}) failed: {source}")]
    Behavior {
        preview: BoundedPreview,
        order: u32,
        stage: TransformStage,
        #[source]
        source: TransformBehaviorError,
    },
    #[error(
        "transform entry {order} (`{preview}` at {stage:?}) returned a lane violating its intrinsic contract: {source}"
    )]
    LaneIntrinsic {
        preview: BoundedPreview,
        order: u32,
        stage: TransformStage,
        #[source]
        source: Box<LaneValidationError>,
    },
    #[error("transform entry {order} (`{preview}` at {stage:?}) refused: {source}")]
    LaneTransition {
        preview: BoundedPreview,
        order: u32,
        stage: TransformStage,
        #[source]
        source: Box<TransitionError>,
    },
    #[error("inter-pass verification rejected transform pass `{pass}`: {source}")]
    Verification {
        pass: PassName,
        #[source]
        source: Box<VerificationError>,
    },
}

/// One typed interim capability gap: a seam a later atom owns, refused now
/// instead of silently approximated.
///
/// [`TransformCapabilityGap::SelectorSubject`] narrowed at T8 rather than
/// disappearing. It used to refuse EVERY selector-bearing source/document
/// entry at construction, because no subject existed to judge one against;
/// the subject now exists, so the gap shrank to the one subject that still
/// cannot be judged — a document whose declaring provider is
/// `Undetermined` met by an authored `packages` dimension. That is a
/// per-document, match-time fact, so the refusal moved with it.
///
/// The family had a second arm until T9: an emitted behavior that returned
/// different bytes refused, because no cell yet owned recomputing the digest
/// and recording which pass had rewritten the tape.
/// [`super::emitted_reconstruction`] now owns exactly that, so the arm was
/// deleted rather than left as a dead spelling. A gap that has been closed is
/// not a variant with no constructor — it is gone, and the tests that proved
/// the refusal now state the law that replaced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum TransformCapabilityGap {
    #[error(
        "this document's declaring provider is undetermined, so an authored `applies_to.packages` dimension cannot be judged; the owner-view adapter supplies the typed coordinate"
    )]
    SelectorSubject,
}

/// The public, opaque transform-fault value one artifact compile returns.
///
/// It names the family, renders the exact internal message and keeps the
/// standard source chain alive, but exposes no plan, stage, registry,
/// behavior or fault taxonomy: the internal enum stays private, and only
/// crate tests read it through the crate-only accessor. No
/// `#[non_exhaustive]` anticipations — T10 freezes external inspection when
/// a real consumer exists.
#[derive(Debug)]
pub struct TransformCompileError {
    inner: Box<TransformError>,
}

impl TransformCompileError {
    pub(crate) fn new(inner: TransformError) -> Self {
        Self {
            inner: Box::new(inner),
        }
    }

    /// The exact internal fault, for crate tests only.
    pub(crate) fn inner(&self) -> &TransformError {
        &self.inner
    }
}

impl From<TransformError> for TransformCompileError {
    fn from(inner: TransformError) -> Self {
        Self::new(inner)
    }
}

impl fmt::Display for TransformCompileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.inner, formatter)
    }
}

impl std::error::Error for TransformCompileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // The standard chain continues through the private fault: an
        // unnamed `dyn Error` source is not a public taxonomy, and silently
        // terminating the chain would contradict the typed-source design.
        Some(self.inner.as_ref())
    }
}

/// Wrap one internal lowering fault as the public opaque lowering error.
pub(super) fn lowering_fault(inner: LoweringFault) -> TransformLoweringError {
    TransformLoweringError {
        inner: Box::new(inner),
    }
}

/// Why one owner's effective rows could not become a plan.
///
/// Typed by fault and by row, never echoing a payload: a declaration key can
/// be attacker-sized, so every arm carries at most the fixed-size preview
/// plus the true length, exactly as the plan refusal law does.
#[derive(Debug, thiserror::Error)]
pub(crate) enum LoweringFault {
    #[error(
        "compile row {row} (`{preview}`) is declared at `{point}`, which is not a compile point; the caller must supply the compile family in effective order"
    )]
    NonCompilePoint {
        row: usize,
        preview: BoundedPreview,
        point: String,
    },
    #[error(
        "compile row {row} (`{preview}`) is a `compile:pass` declaration; the pass tier declares its own placement and is not one of the four staged transform tiers"
    )]
    PassTier { row: usize, preview: BoundedPreview },
    #[error(
        "compile row {row} (`{preview}`) declares a `{kind}` handler; a staged compiler transform is a builtin handler"
    )]
    UnsupportedHandler {
        row: usize,
        preview: BoundedPreview,
        kind: &'static str,
    },
    #[error("compile row {row} (`{preview}`) has no usable implementation: {source}")]
    Implementation {
        row: usize,
        preview: BoundedPreview,
        #[source]
        source: TransformRegistryError,
    },
    #[error("compile row {row} (`{preview}`) has no usable configuration: {source}")]
    Config {
        row: usize,
        preview: BoundedPreview,
        #[source]
        source: ConfigLoweringGap,
    },
    #[error("the lowered rows do not form a plan: {source}")]
    Plan {
        #[source]
        source: TransformPlanError,
    },
}

/// The public, opaque refusal one lowering returns.
///
/// It names the family, renders the exact internal message and keeps the
/// standard source chain alive, but exposes no row, stage, registry or fault
/// taxonomy — the same shape [`super::fault::TransformCompileError`] already
/// established for the execution family, so the crate has one idiom for
/// "typed inside, opaque outside". Crate tests read the exact fault through
/// the crate-only accessor.
#[derive(Debug)]
pub struct TransformLoweringError {
    inner: Box<LoweringFault>,
}

impl TransformLoweringError {
    /// The exact internal fault, for crate tests only.
    #[cfg(test)]
    pub(crate) fn inner(&self) -> &LoweringFault {
        &self.inner
    }
}

impl fmt::Display for TransformLoweringError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.inner, formatter)
    }
}

impl std::error::Error for TransformLoweringError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // The standard chain continues through the private fault, exactly as
        // the execution family's opaque error does: an unnamed `dyn Error`
        // source is not a public taxonomy, and terminating the chain here
        // would silently drop the registry/config/plan cause.
        Some(self.inner.as_ref())
    }
}
