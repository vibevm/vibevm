//! The T6b/T6c/T8 transform schedule cell (R4-TRANSFORM-PLAN-ABI §6.2–6.3):
//! whole-plan resolution against one injected registry, and the four
//! level-preserving pass wrappers the built-in schedule inserts at the frozen
//! positions. The typed fault family the wrappers raise lives in
//! [`super::fault`]; this cell projects entry identity onto it and nothing
//! more.
//!
//! Three positions accept more than a pass-through. The lane wrapper accepts
//! a CHANGED carrier — T6c retired the temporary full-equality detector, and
//! the manager-side [`lane_admission`] gate decides admissibility. The
//! emitted wrapper accepts CHANGED bytes — T9 retired the interim refusal,
//! and the manager-side [`emitted_reconstruction`] cell rebuilds the artifact
//! around them. The source and document wrappers consult the
//! [`super::selector_admission`] gate once per document and SKIP the behavior
//! when the document is out of scope; a skipped behavior is not an error and
//! not a fault.
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

use crate::compiler::emit::attribution;
use crate::compiler::ir::{
    ArtifactFrame, ArtifactPlan, DocumentIr, DocumentSubject, EmittedArtifact, LaneIr, SourceIr,
};
use crate::compiler::observer::{self as analyze_observer, DeltaStage, Observing, StageDeltaEvent};
use crate::compiler::pass::{Pass, PassName};
use crate::compiler::pipeline::{CompilerPipeline, CompilerPipelineError};

use super::fault::{TransformCapabilityGap, TransformError};
use super::native_manager::CompilerNativeInvoker;
use super::native_schedule::TransformExecution;
use super::plan::{ImplementationComponents, TransformConfig, TransformStage};
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
struct ResolvedTransform<'invoke> {
    name: PassName,
    stage: TransformStage,
    order: u32,
    preview: BoundedPreview,
    execution: TransformExecution<'invoke>,
    config: Option<TransformConfig>,
    selector: Option<SelectorGate>,
}

impl ResolvedTransform<'_> {
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
pub(crate) struct TransformSchedule<'invoke> {
    rows: Vec<ResolvedTransform<'invoke>>,
    /// The analyzer observer the lane/emitted wrappers report through
    /// (R4.3). `None` is the unobserved path: no byte is counted, no
    /// event is built.
    observer: Observing,
}

impl<'invoke> TransformSchedule<'invoke> {
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
    pub(crate) fn resolve_with_invoker(
        plan: &ArtifactPlan,
        registry: &TransformRegistry,
        observer: Observing,
        invoker: Option<&'invoke dyn CompilerNativeInvoker>,
    ) -> Result<Self, TransformError> {
        let transforms = plan.transforms();
        let entries = transforms.entries();
        if entries.is_empty() {
            return Ok(Self {
                rows: Vec::new(),
                observer,
            });
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
            let execution = match seed.implementation().components() {
                ImplementationComponents::Builtin { .. } => TransformExecution::from_behavior(
                    registry
                        .resolve(seed.implementation(), &stage)
                        .map_err(|source| TransformError::Resolution {
                            preview: bounded(seed.key().as_str()),
                            order: entry.order(),
                            stage: stage.clone(),
                            source,
                        })?,
                ),
                ImplementationComponents::Native { digest, .. } => TransformExecution::native(
                    seed.key().clone(),
                    invoker.ok_or_else(|| TransformError::NativeInvokerUnavailable {
                        preview: bounded(seed.key().as_str()),
                        order: entry.order(),
                        stage: stage.clone(),
                    })?,
                    digest,
                ),
            };
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
                execution,
                config: seed.config().cloned(),
                selector: seed.selector().map(SelectorGate::new),
            });
        }
        Ok(Self { rows, observer })
    }

    /// The exact schedule-owned pass names, for verifier-fault attribution.
    /// An empty plan yields an empty set with no allocation.
    pub(crate) fn pass_names(&self) -> Vec<PassName> {
        self.rows.iter().map(|row| row.name.clone()).collect()
    }

    fn rows_at(&self, stage: TransformStage) -> impl Iterator<Item = &ResolvedTransform<'invoke>> {
        self.rows.iter().filter(move |row| row.stage == stage)
    }

    /// Insert the source wrappers before the built-in parse pass.
    pub(crate) fn push_source_before_parse(
        &self,
        pipeline: &mut CompilerPipeline<'invoke>,
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
        pipeline: &mut CompilerPipeline<'invoke>,
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
        pipeline: &mut CompilerPipeline<'invoke>,
    ) -> Result<(), TransformError> {
        for row in self.rows_at(TransformStage::Lane) {
            let mut pass = LaneTransformPass::from(row);
            pass.observer = self.observer.clone();
            pipeline
                .push_artifact(pass)
                .map_err(|source| row.schedule_fault(source))?;
        }
        Ok(())
    }

    /// Insert the emitted wrappers after the selected backend's emit pass.
    pub(crate) fn push_emitted_after_emit(
        &self,
        pipeline: &mut CompilerPipeline<'invoke>,
    ) -> Result<(), TransformError> {
        for row in self.rows_at(TransformStage::Emitted) {
            let mut pass = EmittedTransformPass::from(row);
            pass.observer = self.observer.clone();
            pipeline
                .push_artifact(pass)
                .map_err(|source| row.schedule_fault(source))?;
        }
        Ok(())
    }
}

impl TransformSchedule<'static> {
    pub(crate) fn resolve(
        plan: &ArtifactPlan,
        registry: &TransformRegistry,
        observer: Observing,
    ) -> Result<Self, TransformError> {
        Self::resolve_with_invoker(plan, registry, observer, None)
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
struct SourceTransformPass<'invoke> {
    name: PassName,
    order: u32,
    preview: BoundedPreview,
    execution: TransformExecution<'invoke>,
    config: Option<TransformConfig>,
    selector: Option<SelectorGate>,
}

impl<'invoke> From<&ResolvedTransform<'invoke>> for SourceTransformPass<'invoke> {
    fn from(row: &ResolvedTransform<'invoke>) -> Self {
        Self {
            name: row.name.clone(),
            order: row.order,
            preview: row.preview.clone(),
            execution: row.execution.clone(),
            config: row.config.clone(),
            selector: row.selector.clone(),
        }
    }
}

impl Pass for SourceTransformPass<'_> {
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
        self.execution.run_source(
            &self.name,
            &self.preview,
            self.order,
            self.config.as_ref(),
            input,
        )
    }
}

/// The document-position wrapper: one document's parsed tree before gather.
///
/// The subject is reached through the paired source, which is the same value
/// the source position judged, so both positions of one document answer to
/// one subject and parse mints no second one.
struct DocumentTransformPass<'invoke> {
    name: PassName,
    order: u32,
    preview: BoundedPreview,
    execution: TransformExecution<'invoke>,
    config: Option<TransformConfig>,
    selector: Option<SelectorGate>,
}

impl<'invoke> From<&ResolvedTransform<'invoke>> for DocumentTransformPass<'invoke> {
    fn from(row: &ResolvedTransform<'invoke>) -> Self {
        Self {
            name: row.name.clone(),
            order: row.order,
            preview: row.preview.clone(),
            execution: row.execution.clone(),
            config: row.config.clone(),
            selector: row.selector.clone(),
        }
    }
}

impl Pass for DocumentTransformPass<'_> {
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
        self.execution.run_document(
            &self.name,
            &self.preview,
            self.order,
            self.config.as_ref(),
            input,
        )
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
///
/// R4.3: under an analyzer observer the wrapper also reports the lane's
/// chunk-stream byte count before and after the behavior — the lane-byte
/// delta §9 names apart from the artifact-byte delta. Measured on the
/// INPUT before the behavior runs and on the ADMITTED output after, so
/// the reported pair is exactly the transition the manager accepted.
struct LaneTransformPass<'invoke> {
    name: PassName,
    order: u32,
    preview: BoundedPreview,
    execution: TransformExecution<'invoke>,
    config: Option<TransformConfig>,
    observer: Observing,
}

impl<'invoke> From<&ResolvedTransform<'invoke>> for LaneTransformPass<'invoke> {
    fn from(row: &ResolvedTransform<'invoke>) -> Self {
        Self {
            name: row.name.clone(),
            order: row.order,
            preview: row.preview.clone(),
            execution: row.execution.clone(),
            config: row.config.clone(),
            observer: None,
        }
    }
}

impl Pass for LaneTransformPass<'_> {
    type Input = LaneIr;
    type Output = LaneIr;
    type Error = TransformError;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, input: LaneIr) -> Result<LaneIr, TransformError> {
        let before = self
            .observer
            .as_ref()
            .map(|_| attribution::lane_content_bytes(&input));
        let output = self.execution.run_lane(
            &self.name,
            &self.preview,
            self.order,
            self.config.as_ref(),
            input,
        )?;
        if let (Some(observer), Some(before)) = (self.observer.as_deref(), before) {
            let event = StageDeltaEvent::new(
                self.name.as_str(),
                DeltaStage::Lane,
                before,
                attribution::lane_content_bytes(&output),
            );
            analyze_observer::deliver_stage_delta(observer, &event);
        }
        Ok(output)
    }
}

/// The emitted-position wrapper: owned artifact bytes in, new bytes out —
/// never a mutable artifact reference.
///
/// T9 law: a changed tape is legal, and the MANAGER alone rebuilds the
/// artifact around it. The wrapper hands the original artifact, the behavior's
/// bytes and its own exact pass name to [`emitted_reconstruction`] and returns
/// what that cell answers — the ORIGINAL value untouched when the bytes did
/// not move, a wholly rebuilt one (recomputed digest, appended transform name,
/// every other provenance member copied) when they did. The wrapper owns
/// nothing else: it does not compare the tapes, recompute a digest or touch
/// provenance, so there is exactly one writer of a post-backend artifact.
///
/// R4.3: under an analyzer observer the wrapper also reports the
/// artifact's byte count before and after — the artifact-byte delta §9
/// names apart from the lane-byte delta. The `after` side is the
/// RECONSTRUCTED artifact's length, so a behavior whose bytes did not
/// move reports an honest no-op pair.
struct EmittedTransformPass<'invoke> {
    name: PassName,
    order: u32,
    preview: BoundedPreview,
    execution: TransformExecution<'invoke>,
    config: Option<TransformConfig>,
    observer: Observing,
}

impl<'invoke> From<&ResolvedTransform<'invoke>> for EmittedTransformPass<'invoke> {
    fn from(row: &ResolvedTransform<'invoke>) -> Self {
        Self {
            name: row.name.clone(),
            order: row.order,
            preview: row.preview.clone(),
            execution: row.execution.clone(),
            config: row.config.clone(),
            observer: None,
        }
    }
}

impl Pass for EmittedTransformPass<'_> {
    type Input = EmittedArtifact;
    type Output = EmittedArtifact;
    type Error = TransformError;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, input: EmittedArtifact) -> Result<EmittedArtifact, TransformError> {
        let observer = self.observer.as_deref();
        let before = observer.map(|_| input.bytes().len());
        let reconstructed = self.execution.run_emitted(
            &self.name,
            &self.preview,
            self.order,
            self.config.as_ref(),
            input,
        )?;
        if let (Some(observer), Some(before)) = (observer, before) {
            let event = StageDeltaEvent::new(
                self.name.as_str(),
                DeltaStage::Emitted,
                before,
                reconstructed.bytes().len(),
            );
            analyze_observer::deliver_stage_delta(observer, &event);
        }
        Ok(reconstructed)
    }
}
