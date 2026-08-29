//! The T6b/T6c transform schedule cell (R4-TRANSFORM-PLAN-ABI §6.2–6.3): the
//! one typed fault family for resolution, schedule, capability, behavior,
//! lane-admission and transform-attributed verifier faults; the opaque public
//! [`TransformCompileError`]; whole-plan resolution against one injected
//! registry; and the four level-preserving pass wrappers the built-in
//! schedule inserts at the frozen positions.
//!
//! The lane wrapper is the one position that already accepts a CHANGED
//! carrier: T6c retired the temporary full-equality detector, and the
//! manager-side [`lane_admission`] gate now decides admissibility. The
//! source/document selector and emitted-bytes gaps remain.
//!
//! Construction is a two-step transaction: every entry resolves (frame
//! refusal first, then name → epoch → stage, then the source/document
//! selector capability gap, all in dense plan order, first fault wins)
//! BEFORE anything is pushed or executed, and the resolved rows are then
//! stably partitioned by stage — never sorted by key, catalog or `BTreeMap`
//! iteration. Behavior objects live only here and in nothing the plan sees.
//! The cell names no builtin/driver type: converting a fault into the
//! artifact-level error family belongs to the attribution cells above.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY");

use std::fmt;
use std::sync::Arc;

use crate::compiler::assemble::LaneValidationError;
use crate::compiler::ir::{
    ArtifactFrame, ArtifactPlan, DocumentIr, EmittedArtifact, LaneIr, SourceIr,
};
use crate::compiler::pass::{Pass, PassName, PassNameError};
use crate::compiler::pipeline::{CompilerPipeline, CompilerPipelineError};
use crate::compiler::verify::{TransitionError, VerificationError};

use super::behavior::{TransformBehavior, TransformBehaviorError};
use super::lane_admission::{self, LaneAdmissionError};
use super::plan::{TransformConfig, TransformStage};
use super::plan_validate::BoundedPreview;
use super::plan_validate::bounded;
use super::registry::{TransformRegistry, TransformRegistryError};

/// Why one planned transform refused at construction or execution.
///
/// Typed end to end: the registry, behavior, pass-name and pipeline refusals
/// ride along as their exact types, and every entry fault names the bounded
/// key preview, the dense order and the stage — never a re-parsed pass name.
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

/// One typed interim capability gap of the T6b execution split: a seam a
/// later atom owns, refused now instead of silently approximated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum TransformCapabilityGap {
    #[error(
        "the selector subject arrives with T7/T8; a still-present source/document selector cannot execute yet"
    )]
    SelectorSubject,
    #[error(
        "emitted reconstruction arrives with T9; byte-equal output returns the original artifact, changed bytes refuse"
    )]
    EmittedChange,
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

/// The frozen schedule token of one stage inside a pass name.
fn stage_token(stage: &TransformStage) -> &'static str {
    match stage {
        TransformStage::Source => "source",
        TransformStage::Document => "document",
        TransformStage::Lane => "lane",
        TransformStage::Emitted => "emitted",
    }
}

/// One plan entry after successful resolution: everything a wrapper needs,
/// cloned off the seed, plus the identity the wrapper owns for attribution.
struct ResolvedTransform {
    name: PassName,
    stage: TransformStage,
    order: u32,
    preview: BoundedPreview,
    behavior: Arc<dyn TransformBehavior>,
    config: Option<TransformConfig>,
}

impl ResolvedTransform {
    /// The per-row entry fault for a pipeline-insertion refusal: the row's
    /// bounded preview, dense order and exact stage ride along.
    fn schedule_fault(&self, source: CompilerPipelineError) -> TransformError {
        TransformError::Schedule {
            preview: self.preview.clone(),
            order: self.order,
            stage: self.stage.clone(),
            source,
        }
    }
}

/// Every resolved row of one plan, still in dense plan order.
///
/// Pushing filters by stage — a stable partition — so within-stage authored
/// order survives without any sort.
pub(crate) struct TransformSchedule {
    rows: Vec<ResolvedTransform>,
}

impl TransformSchedule {
    /// Resolve a whole plan against one registry, or refuse it.
    ///
    /// The frozen precedence: a nonempty plan on a compatibility-fragment
    /// frame refuses before any lookup; then each entry in dense plan order
    /// resolves name → epoch → stage and sheds a still-present source/
    /// document selector as the T7/T8 capability gap. The first fault wins
    /// and nothing is returned or pushed on a partial walk. Even the pass
    /// name refuses typed — no panic path exists in this cell.
    pub(crate) fn resolve(
        plan: &ArtifactPlan,
        registry: &TransformRegistry,
    ) -> Result<Self, TransformError> {
        let transforms = plan.transforms();
        let entries = transforms.entries();
        if entries.is_empty() {
            return Ok(Self { rows: Vec::new() });
        }
        if matches!(plan.context().frame(), ArtifactFrame::CompatibilityFragment) {
            return Err(TransformError::CompatibilityFragmentPlan {
                // Lossless: `TransformPlan::build` already refused any seed
                // count above `u32::MAX` through `plan_validate::
                // checked_entry_count`, so a validated plan's entry count
                // always fits — the dense `order` this cell reports rides on
                // the same guarantee.
                entries: entries.len() as u32,
            });
        }
        let mut rows = Vec::with_capacity(entries.len());
        for entry in entries {
            let seed = entry.seed();
            let stage = seed.stage().clone();
            let behavior = registry
                .resolve(seed.implementation(), &stage)
                .map_err(|source| TransformError::Resolution {
                    preview: bounded(seed.key().as_str()),
                    order: entry.order(),
                    stage: stage.clone(),
                    source,
                })?;
            if matches!(stage, TransformStage::Source | TransformStage::Document)
                && seed.selector().is_some()
            {
                return Err(TransformError::Capability {
                    preview: bounded(seed.key().as_str()),
                    order: entry.order(),
                    stage,
                    gap: TransformCapabilityGap::SelectorSubject,
                });
            }
            let name = PassName::new(format!(
                "transform:{}:{}",
                stage_token(&stage),
                seed.key().as_str()
            ))
            .map_err(|source| TransformError::Name {
                preview: bounded(seed.key().as_str()),
                order: entry.order(),
                stage: stage.clone(),
                source,
            })?;
            rows.push(ResolvedTransform {
                name,
                stage,
                order: entry.order(),
                preview: bounded(seed.key().as_str()),
                behavior,
                config: seed.config().cloned(),
            });
        }
        Ok(Self { rows })
    }

    /// The exact schedule-owned pass names, for verifier-fault attribution.
    /// An empty plan yields an empty set with no allocation.
    pub(crate) fn pass_names(&self) -> Vec<PassName> {
        self.rows.iter().map(|row| row.name.clone()).collect()
    }

    fn rows_at(&self, stage: TransformStage) -> impl Iterator<Item = &ResolvedTransform> {
        self.rows.iter().filter(move |row| row.stage == stage)
    }

    /// Insert the source wrappers before the built-in parse pass.
    pub(crate) fn push_source_before_parse(
        &self,
        pipeline: &mut CompilerPipeline,
    ) -> Result<(), TransformError> {
        for row in self.rows_at(TransformStage::Source) {
            pipeline
                .push_document(SourceTransformPass::from(row))
                .map_err(|source| row.schedule_fault(source))?;
        }
        Ok(())
    }

    /// Insert the document wrappers after the built-in parse pass.
    pub(crate) fn push_document_after_parse(
        &self,
        pipeline: &mut CompilerPipeline,
    ) -> Result<(), TransformError> {
        for row in self.rows_at(TransformStage::Document) {
            pipeline
                .push_document(DocumentTransformPass::from(row))
                .map_err(|source| row.schedule_fault(source))?;
        }
        Ok(())
    }

    /// Insert the lane wrappers after the built-in assemble pass.
    pub(crate) fn push_lane_after_assemble(
        &self,
        pipeline: &mut CompilerPipeline,
    ) -> Result<(), TransformError> {
        for row in self.rows_at(TransformStage::Lane) {
            pipeline
                .push_artifact(LaneTransformPass::from(row))
                .map_err(|source| row.schedule_fault(source))?;
        }
        Ok(())
    }

    /// Insert the emitted wrappers after the selected backend's emit pass.
    pub(crate) fn push_emitted_after_emit(
        &self,
        pipeline: &mut CompilerPipeline,
    ) -> Result<(), TransformError> {
        for row in self.rows_at(TransformStage::Emitted) {
            pipeline
                .push_artifact(EmittedTransformPass::from(row))
                .map_err(|source| row.schedule_fault(source))?;
        }
        Ok(())
    }
}

/// The shared wrapper fault projection: entry identity plus the typed source.
fn wrapper_fault(
    preview: &BoundedPreview,
    order: u32,
    stage: &TransformStage,
    source: TransformBehaviorError,
) -> TransformError {
    TransformError::Behavior {
        preview: preview.clone(),
        order,
        stage: stage.clone(),
        source,
    }
}

/// The source-position wrapper: one document's raw text before parsing.
struct SourceTransformPass {
    name: PassName,
    order: u32,
    preview: BoundedPreview,
    behavior: Arc<dyn TransformBehavior>,
    config: Option<TransformConfig>,
}

impl From<&ResolvedTransform> for SourceTransformPass {
    fn from(row: &ResolvedTransform) -> Self {
        Self {
            name: row.name.clone(),
            order: row.order,
            preview: row.preview.clone(),
            behavior: row.behavior.clone(),
            config: row.config.clone(),
        }
    }
}

impl Pass for SourceTransformPass {
    type Input = SourceIr;
    type Output = SourceIr;
    type Error = TransformError;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, input: SourceIr) -> Result<SourceIr, TransformError> {
        self.behavior
            .run_source(self.config.as_ref(), input)
            .map_err(|source| {
                wrapper_fault(&self.preview, self.order, &TransformStage::Source, source)
            })
    }
}

/// The document-position wrapper: one document's parsed tree before gather.
struct DocumentTransformPass {
    name: PassName,
    order: u32,
    preview: BoundedPreview,
    behavior: Arc<dyn TransformBehavior>,
    config: Option<TransformConfig>,
}

impl From<&ResolvedTransform> for DocumentTransformPass {
    fn from(row: &ResolvedTransform) -> Self {
        Self {
            name: row.name.clone(),
            order: row.order,
            preview: row.preview.clone(),
            behavior: row.behavior.clone(),
            config: row.config.clone(),
        }
    }
}

impl Pass for DocumentTransformPass {
    type Input = DocumentIr;
    type Output = DocumentIr;
    type Error = TransformError;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, input: DocumentIr) -> Result<DocumentIr, TransformError> {
        self.behavior
            .run_document(self.config.as_ref(), input)
            .map_err(|source| {
                wrapper_fault(&self.preview, self.order, &TransformStage::Document, source)
            })
    }
}

/// The lane-position wrapper: the assembled lane, structured, once per
/// artifact.
///
/// T6c law: a changed lane is legal, and the MANAGER decides whether the
/// change is lawful. The immutable witness is taken from the input before the
/// behavior runs, and the output must then pass both halves of
/// [`lane_admission`] — the intrinsic lane contract and the provenance
/// transition — before it is returned. Both run unconditionally: the
/// inter-pass verifier hook is test-only, so routing this decision through it
/// would leave production unguarded.
struct LaneTransformPass {
    name: PassName,
    order: u32,
    preview: BoundedPreview,
    behavior: Arc<dyn TransformBehavior>,
    config: Option<TransformConfig>,
}

impl From<&ResolvedTransform> for LaneTransformPass {
    fn from(row: &ResolvedTransform) -> Self {
        Self {
            name: row.name.clone(),
            order: row.order,
            preview: row.preview.clone(),
            behavior: row.behavior.clone(),
            config: row.config.clone(),
        }
    }
}

impl Pass for LaneTransformPass {
    type Input = LaneIr;
    type Output = LaneIr;
    type Error = TransformError;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, input: LaneIr) -> Result<LaneIr, TransformError> {
        // Derived from the INPUT: evidence taken after the behavior ran would
        // only ever agree with itself.
        let witness = lane_admission::witness(&input);
        let output = self
            .behavior
            .run_lane(self.config.as_ref(), input)
            .map_err(|source| {
                wrapper_fault(&self.preview, self.order, &TransformStage::Lane, source)
            })?;
        lane_admission::admit(&witness, &output).map_err(|refusal| self.lane_fault(refusal))?;
        Ok(output)
    }
}

impl LaneTransformPass {
    /// Project one admission refusal onto this entry's identity: the bounded
    /// key preview, the dense plan order and the stage ride along, exactly as
    /// every other entry fault carries them.
    fn lane_fault(&self, refusal: LaneAdmissionError) -> TransformError {
        match refusal {
            LaneAdmissionError::Intrinsic(source) => TransformError::LaneIntrinsic {
                preview: self.preview.clone(),
                order: self.order,
                stage: TransformStage::Lane,
                source,
            },
            LaneAdmissionError::Transition(source) => TransformError::LaneTransition {
                preview: self.preview.clone(),
                order: self.order,
                stage: TransformStage::Lane,
                source,
            },
        }
    }
}

/// The emitted-position wrapper: owned artifact bytes in, new bytes out —
/// never a mutable artifact reference.
///
/// T6b interim law: unequal bytes refuse with the typed
/// [`TransformCapabilityGap::EmittedChange`] before any reconstruction;
/// byte-equal output returns the ORIGINAL [`EmittedArtifact`] untouched —
/// provenance, digest and fingerprint included. T9 owns reconstruction.
struct EmittedTransformPass {
    name: PassName,
    order: u32,
    preview: BoundedPreview,
    behavior: Arc<dyn TransformBehavior>,
    config: Option<TransformConfig>,
}

impl From<&ResolvedTransform> for EmittedTransformPass {
    fn from(row: &ResolvedTransform) -> Self {
        Self {
            name: row.name.clone(),
            order: row.order,
            preview: row.preview.clone(),
            behavior: row.behavior.clone(),
            config: row.config.clone(),
        }
    }
}

impl Pass for EmittedTransformPass {
    type Input = EmittedArtifact;
    type Output = EmittedArtifact;
    type Error = TransformError;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, input: EmittedArtifact) -> Result<EmittedArtifact, TransformError> {
        let bytes = input.bytes().to_vec();
        let output = self
            .behavior
            .run_emitted(self.config.as_ref(), bytes)
            .map_err(|source| {
                wrapper_fault(&self.preview, self.order, &TransformStage::Emitted, source)
            })?;
        if output.as_slice() != input.bytes() {
            return Err(TransformError::Capability {
                preview: self.preview.clone(),
                order: self.order,
                stage: TransformStage::Emitted,
                gap: TransformCapabilityGap::EmittedChange,
            });
        }
        Ok(input)
    }
}
