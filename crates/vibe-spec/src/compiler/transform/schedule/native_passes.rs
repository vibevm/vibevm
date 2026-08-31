//! Four stage-preserving transform pass wrappers.

use crate::compiler::emit::attribution;
use crate::compiler::ir::{DocumentIr, DocumentSubject, EmittedArtifact, LaneIr, SourceIr};
use crate::compiler::observer::{self as analyze_observer, DeltaStage, Observing, StageDeltaEvent};
use crate::compiler::pass::{Pass, PassName};

use super::ResolvedTransform;
use crate::compiler::transform::fault::{TransformCapabilityGap, TransformError};
use crate::compiler::transform::native_schedule::TransformExecution;
use crate::compiler::transform::plan::{TransformConfig, TransformStage};
use crate::compiler::transform::plan_validate::BoundedPreview;
use crate::compiler::transform::selector_admission::{
    SelectorAdmissionError, SelectorGate, SelectorVerdict,
};

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

pub(super) struct SourceTransformPass<'invoke> {
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
        if selector_decision(
            self.selector.as_ref(),
            input.subject(),
            &self.preview,
            self.order,
            &TransformStage::Source,
        )? == SelectorVerdict::Skipped
        {
            return Ok(input);
        }
        self.execution
            .run_source(
                &self.name,
                &self.preview,
                self.order,
                self.config.as_ref(),
                input,
            )
            .map(|run| run.output)
    }
}

pub(super) struct DocumentTransformPass<'invoke> {
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
        if selector_decision(
            self.selector.as_ref(),
            input.source().subject(),
            &self.preview,
            self.order,
            &TransformStage::Document,
        )? == SelectorVerdict::Skipped
        {
            return Ok(input);
        }
        self.execution
            .run_document(
                &self.name,
                &self.preview,
                self.order,
                self.config.as_ref(),
                input,
            )
            .map(|run| run.output)
    }
}

pub(super) struct LaneTransformPass<'invoke> {
    name: PassName,
    order: u32,
    preview: BoundedPreview,
    execution: TransformExecution<'invoke>,
    config: Option<TransformConfig>,
    pub(super) observer: Observing,
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
        let run = self.execution.run_lane(
            &self.name,
            &self.preview,
            self.order,
            self.config.as_ref(),
            input,
        )?;
        if run.executed
            && let (Some(observer), Some(before)) = (self.observer.as_deref(), before)
        {
            let event = StageDeltaEvent::new(
                self.name.as_str(),
                DeltaStage::Lane,
                before,
                attribution::lane_content_bytes(&run.output),
            );
            analyze_observer::deliver_stage_delta(observer, &event);
        }
        Ok(run.output)
    }
}

pub(super) struct EmittedTransformPass<'invoke> {
    name: PassName,
    order: u32,
    preview: BoundedPreview,
    execution: TransformExecution<'invoke>,
    config: Option<TransformConfig>,
    pub(super) observer: Observing,
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
        let run = self.execution.run_emitted(
            &self.name,
            &self.preview,
            self.order,
            self.config.as_ref(),
            input,
        )?;
        if run.executed
            && let (Some(observer), Some(before)) = (observer, before)
        {
            let event = StageDeltaEvent::new(
                self.name.as_str(),
                DeltaStage::Emitted,
                before,
                run.output.bytes().len(),
            );
            analyze_observer::deliver_stage_delta(observer, &event);
        }
        Ok(run.output)
    }
}
