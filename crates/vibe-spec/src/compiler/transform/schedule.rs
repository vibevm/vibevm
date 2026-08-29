//! The T6b/T6c/T8 transform schedule cell (R4-TRANSFORM-PLAN-ABI §6.2–6.3):
//! whole-plan resolution against one injected registry, and the four
//! level-preserving pass wrappers the built-in schedule inserts at the frozen
//! positions. The typed fault family the wrappers raise lives in
//! [`super::fault`]; this cell projects entry identity onto it and nothing
//! more.
//!
//! Two positions accept more than a pass-through. The lane wrapper accepts a
//! CHANGED carrier — T6c retired the temporary full-equality detector, and
//! the manager-side [`lane_admission`] gate decides admissibility. The
//! source and document wrappers consult the [`super::selector_admission`]
//! gate once per document and SKIP the behavior when the document is out of
//! scope; a skipped behavior is not an error and not a fault. The
//! emitted-bytes gap remains.
//!
//! Construction is a two-step transaction: every entry resolves (frame
//! refusal first, then name → epoch → stage, in dense plan order, first fault
//! wins) BEFORE anything is pushed or executed, and the resolved rows are
//! then stably partitioned by stage — never sorted by key, catalog or
//! `BTreeMap` iteration. T8 removed the construction-time selector refusal
//! rather than weakening that transaction: a selector verdict needs a
//! document subject, no document exists while a plan is being resolved, and
//! nothing that a subject is needed for was ever decidable here.
//!
//! Behavior objects live only here and in nothing the plan sees. The cell
//! names no builtin/driver type — converting a fault into the artifact-level
//! error family belongs to the attribution cells above — and it names no
//! kernel selector type either: it stores an opaque [`SelectorGate`], so the
//! one cell allowed to touch the kernel selector really is the only cell
//! that does.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY");

use std::sync::Arc;

use crate::compiler::ir::{
    ArtifactFrame, ArtifactPlan, DocumentIr, DocumentSubject, EmittedArtifact, LaneIr, SourceIr,
};
use crate::compiler::pass::{Pass, PassName};
use crate::compiler::pipeline::{CompilerPipeline, CompilerPipelineError};

use super::behavior::{TransformBehavior, TransformBehaviorError};
use super::fault::{TransformCapabilityGap, TransformError};
use super::lane_admission::{self, LaneAdmissionError};
use super::plan::{TransformConfig, TransformStage};
use super::plan_validate::BoundedPreview;
use super::plan_validate::bounded;
use super::registry::TransformRegistry;
use super::selector_admission::{SelectorAdmissionError, SelectorGate, SelectorVerdict};

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
///
/// The selector is cloned here exactly once, as an opaque
/// [`SelectorGate`], so matching never reaches back into the plan and the
/// kernel selector type never has to be named at this level.
struct ResolvedTransform {
    name: PassName,
    stage: TransformStage,
    order: u32,
    preview: BoundedPreview,
    behavior: Arc<dyn TransformBehavior>,
    config: Option<TransformConfig>,
    selector: Option<SelectorGate>,
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
    /// resolves name → epoch → stage. The first fault wins and nothing is
    /// returned or pushed on a partial walk. Even the pass name refuses
    /// typed — no panic path exists in this cell.
    ///
    /// A present source/document selector is no longer a refusal here. It
    /// used to be the T7/T8 capability gap, because judging one needed a
    /// document subject that construction cannot have; the subject now
    /// exists per document, so the verdict — including its one remaining
    /// refusal — moved to the wrappers, where a document does exist. The
    /// grammar law that lane/emitted carry no selector at all is enforced a
    /// layer earlier, by `plan_validate::validate_selector_stage`, so a
    /// selector reaching this walk is always a source/document one.
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
                selector: seed.selector().map(SelectorGate::new),
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

/// Decide whether one wrapper runs its behavior on one document.
///
/// An entry with no selector applies to every document, which is why absence
/// answers [`SelectorVerdict::Matched`] rather than being a third state: the
/// wrapper asks one question and gets one of two answers, or a refusal.
fn selector_decision(
    selector: Option<&SelectorGate>,
    subject: &DocumentSubject,
    preview: &BoundedPreview,
    order: u32,
    stage: &TransformStage,
) -> Result<SelectorVerdict, TransformError> {
    let Some(gate) = selector else {
        return Ok(SelectorVerdict::Matched);
    };
    gate.admit(subject)
        .map_err(|refusal| selector_fault(preview, order, stage, refusal))
}

/// Project one admission refusal onto the entry's identity: the bounded key
/// preview, the dense plan order and the stage ride along, exactly as every
/// other entry fault carries them.
///
/// The two arms land in different families on purpose. An undetermined
/// provider is the surviving, narrowed T7/T8→T10 capability gap — a seam a
/// later atom owns — so it keeps
/// [`TransformCapabilityGap::SelectorSubject`]. A backslashed declared path
/// is not a gap at all: it is a stated contract, violated, and it carries its
/// own typed source.
fn selector_fault(
    preview: &BoundedPreview,
    order: u32,
    stage: &TransformStage,
    refusal: SelectorAdmissionError,
) -> TransformError {
    match refusal {
        SelectorAdmissionError::UndeterminedProvider => TransformError::Capability {
            preview: preview.clone(),
            order,
            stage: stage.clone(),
            gap: TransformCapabilityGap::SelectorSubject,
        },
        source @ SelectorAdmissionError::BackslashedDeclaredPath { .. } => {
            TransformError::Selector {
                preview: preview.clone(),
                order,
                stage: stage.clone(),
                source,
            }
        }
    }
}

/// The source-position wrapper: one document's raw text before parsing.
///
/// The selector is consulted against the document's own carried subject
/// before the behavior is invoked, so a document out of scope is returned
/// untouched and the behavior never observes it.
struct SourceTransformPass {
    name: PassName,
    order: u32,
    preview: BoundedPreview,
    behavior: Arc<dyn TransformBehavior>,
    config: Option<TransformConfig>,
    selector: Option<SelectorGate>,
}

impl From<&ResolvedTransform> for SourceTransformPass {
    fn from(row: &ResolvedTransform) -> Self {
        Self {
            name: row.name.clone(),
            order: row.order,
            preview: row.preview.clone(),
            behavior: row.behavior.clone(),
            config: row.config.clone(),
            selector: row.selector.clone(),
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
        let verdict = selector_decision(
            self.selector.as_ref(),
            input.subject(),
            &self.preview,
            self.order,
            &TransformStage::Source,
        )?;
        if verdict == SelectorVerdict::Skipped {
            return Ok(input);
        }
        self.behavior
            .run_source(self.config.as_ref(), input)
            .map_err(|source| {
                wrapper_fault(&self.preview, self.order, &TransformStage::Source, source)
            })
    }
}

/// The document-position wrapper: one document's parsed tree before gather.
///
/// The subject is reached through the paired source, which is the same value
/// the source position judged, so both positions of one document answer to
/// one subject and parse mints no second one.
struct DocumentTransformPass {
    name: PassName,
    order: u32,
    preview: BoundedPreview,
    behavior: Arc<dyn TransformBehavior>,
    config: Option<TransformConfig>,
    selector: Option<SelectorGate>,
}

impl From<&ResolvedTransform> for DocumentTransformPass {
    fn from(row: &ResolvedTransform) -> Self {
        Self {
            name: row.name.clone(),
            order: row.order,
            preview: row.preview.clone(),
            behavior: row.behavior.clone(),
            config: row.config.clone(),
            selector: row.selector.clone(),
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
        let verdict = selector_decision(
            self.selector.as_ref(),
            input.source().subject(),
            &self.preview,
            self.order,
            &TransformStage::Document,
        )?;
        if verdict == SelectorVerdict::Skipped {
            return Ok(input);
        }
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
