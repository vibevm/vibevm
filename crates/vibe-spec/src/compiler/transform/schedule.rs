//! Whole-plan transform resolution and schedule insertion.
//!
//! Every entry resolves transactionally in dense plan order before the stage
//! wrappers in `native_passes` are inserted. Selectors remain wrapper-local,
//! where a real document subject exists.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY");

use crate::compiler::ir::{ArtifactFrame, ArtifactPlan};
use crate::compiler::observer::Observing;
use crate::compiler::pass::PassName;
use crate::compiler::pipeline::{CompilerPipeline, CompilerPipelineError};

use super::fault::TransformError;
use super::native_manager::CompilerNativeInvoker;
use super::native_policy::session::NativePolicySession;
use super::native_schedule::TransformExecution;
use super::plan::{ImplementationComponents, TransformConfig, TransformStage};
use super::plan_validate::{BoundedPreview, bounded};
use super::registry::TransformRegistry;
use super::selector_admission::SelectorGate;

#[path = "schedule/native_passes.rs"]
mod native_passes;
use native_passes::{
    DocumentTransformPass, EmittedTransformPass, LaneTransformPass, SourceTransformPass,
};

fn stage_token(stage: &TransformStage) -> &'static str {
    match stage {
        TransformStage::Source => "source",
        TransformStage::Document => "document",
        TransformStage::Lane => "lane",
        TransformStage::Emitted => "emitted",
    }
}

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
pub(crate) struct TransformSchedule<'invoke> {
    rows: Vec<ResolvedTransform<'invoke>>,
    observer: Observing,
}

impl<'invoke> TransformSchedule<'invoke> {
    pub(crate) fn resolve_with_invoker(
        plan: &ArtifactPlan,
        registry: &TransformRegistry,
        observer: Observing,
        invoker: Option<&'invoke dyn CompilerNativeInvoker>,
        policy: Option<&'invoke NativePolicySession>,
    ) -> Result<Self, TransformError> {
        let entries = plan.transforms().entries();
        if entries.is_empty() {
            return Ok(Self {
                rows: Vec::new(),
                observer,
            });
        }
        if matches!(plan.context().frame(), ArtifactFrame::CompatibilityFragment) {
            return Err(TransformError::CompatibilityFragmentPlan {
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
                    policy,
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

    pub(crate) fn pass_names(&self) -> Vec<PassName> {
        self.rows.iter().map(|row| row.name.clone()).collect()
    }

    fn rows_at(&self, stage: TransformStage) -> impl Iterator<Item = &ResolvedTransform<'invoke>> {
        self.rows.iter().filter(move |row| row.stage == stage)
    }

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
        Self::resolve_with_invoker(plan, registry, observer, None, None)
    }
}
