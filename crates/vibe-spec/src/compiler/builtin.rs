//! Built-in passes and the declared schedule prefix migrated so far.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR");

use crate::use_graph::UseGraphError;
use crate::{DocTree, SectionSource, SpecAddress};

#[cfg(test)]
use super::absorb::ABSORB_PASS_NAME;
use super::absorb::AbsorbPass;
use super::assemble::{ASSEMBLE_PASS_NAME, AssemblePass, AssemblePassError};
use super::backend::{BackendRegistry, BackendRegistryError, EmitBackend};
use super::close::{CLOSE_PASS_NAME, ClosePass, CloseState};
use super::embed::{EMBED_PASS_NAME, EmbedPass, EmbedPassError};
use super::emit::{EmitPass, EmitPassError};
use super::ir::{
    ArtifactPlan, ClosureIr, DocumentIr, EmittedArtifact, LaneIr, SourceIr, StaticCompileMode,
};
use super::link::{LINK_PASS_NAME, LinkPass, LinkPassError};
use super::merge::{MERGE_PASS_NAME, MergePass, MergePassError};
use super::pass::{Pass, PassName, PassSegmentError};
use super::pipeline::{CompilerPipeline, CompilerPipelineError};
#[cfg(test)]
use super::qualify::QUALIFY_PASS_NAME;
use super::qualify::QualifyPass;
use super::worklist;

const PARSE_PASS_NAME: &str = "parse";
const MARKDOWN_FORMAT: &str = "markdown";

mod driver;
#[cfg(test)]
pub(crate) use driver::compile_artifact_with_registry;
pub(crate) use driver::compile_compatibility_artifact;
pub use driver::{ArtifactCompileError, compile_artifact};
#[cfg(feature = "test-support")]
pub use driver::{
    compile_artifact_missing_backend_test_vehicle, compile_artifact_opaque_test_vehicle,
    compile_artifact_replacement_test_vehicle,
};

/// The built-in source-to-document lowering.
///
/// The shipping [`crate::SectionSource`] seam supplies canonical Markdown even
/// when the authored file is XML, so this pass has one built-in frontend today.
/// R6 may register additional frontend passes without widening this one.
struct ParsePass {
    name: PassName,
}

impl ParsePass {
    fn new() -> Self {
        Self {
            name: PassName::new(PARSE_PASS_NAME)
                .expect("the static built-in parse pass name is non-blank"),
        }
    }
}

impl Pass for ParsePass {
    type Input = SourceIr;
    type Output = DocumentIr;
    type Error = ParseError;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, input: SourceIr) -> Result<DocumentIr, ParseError> {
        #[cfg(test)]
        PARSE_INVOCATIONS.with(|count| count.set(count.get() + 1));
        if input.format().as_str() != MARKDOWN_FORMAT {
            return Err(ParseError {
                format: input.format().as_str().to_string(),
            });
        }

        let tree = DocTree::parse(input.text());
        Ok(DocumentIr::new(input, tree))
    }
}

#[cfg(test)]
std::thread_local! {
    static PARSE_INVOCATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_parse_invocations() {
    PARSE_INVOCATIONS.with(|count| count.set(0));
}

#[cfg(test)]
pub(crate) fn parse_invocations() -> usize {
    PARSE_INVOCATIONS.with(std::cell::Cell::get)
}

/// A parse failure remains the source of the manager's named-pass error.
#[derive(Debug, thiserror::Error)]
#[error("the built-in parser does not accept source format `{format}`")]
struct ParseError {
    format: String,
}

/// The declared built-in schedule prefix currently used by production.
///
/// Keeping construction in one function makes the list executable rather than
/// a registry beside a separate call path. R3.2 appends later built-ins here as
/// each phase migrates.
pub(crate) struct BuiltinSchedule {
    pipeline: CompilerPipeline,
    close_state: CloseState,
}

impl BuiltinSchedule {
    fn linked(plan: &ArtifactPlan) -> Self {
        let close_state = CloseState::default();
        let mut pipeline = CompilerPipeline::default();
        pipeline
            .push_document(ParsePass::new())
            .expect("the static built-in parse schedule is valid");
        pipeline
            .push_artifact(ClosePass::new(plan.clone(), close_state.clone()))
            .expect("the static built-in close schedule is valid");
        pipeline
            .push_artifact(MergePass::new())
            .expect("the static built-in merge schedule is valid");
        pipeline
            .push_artifact(EmbedPass::new())
            .expect("the static built-in embed schedule is valid");
        pipeline
            .push_artifact(QualifyPass::new())
            .expect("the static built-in qualify schedule is valid");
        pipeline
            .push_artifact(AbsorbPass::new())
            .expect("the static built-in absorb schedule is valid");
        pipeline
            .push_artifact(LinkPass::new())
            .expect("the static built-in link schedule is valid");
        Self {
            pipeline,
            close_state,
        }
    }

    fn assembled(plan: &ArtifactPlan) -> Self {
        let mut schedule = Self::linked(plan);
        schedule
            .pipeline
            .push_artifact(AssemblePass::new())
            .expect("the static built-in assemble schedule is valid");
        schedule
    }

    fn emitted(
        plan: &ArtifactPlan,
        registry: &BackendRegistry,
    ) -> Result<Self, BackendRegistryError> {
        let backend = registry.selected(plan.context().target())?;
        Ok(Self::with_backend(plan, backend))
    }

    fn with_backend(plan: &ArtifactPlan, backend: std::sync::Arc<dyn EmitBackend>) -> Self {
        let mut schedule = Self::assembled(plan);
        schedule
            .pipeline
            .push_artifact(EmitPass::new(backend))
            .expect("the selected emit backend continues the built-in schedule");
        schedule
    }

    fn parse_source(&self, source: SourceIr) -> DocumentIr {
        self.pipeline
            .run_document(source)
            .expect("the private parse segment accepts canonical Markdown sources")
    }

    fn record_failure(&self, address: &SpecAddress, reason: String) {
        self.close_state.record_failure(address, reason);
    }

    fn close(
        &self,
        documents: Vec<DocumentIr>,
    ) -> Result<ClosureIr, crate::pipeline::CompileError> {
        let documents = self.pipeline.gather_documents(documents);
        self.map_artifact_result(self.pipeline.run_to_closure(documents))
    }

    fn assemble(
        &self,
        documents: Vec<DocumentIr>,
    ) -> Result<LaneIr, crate::pipeline::CompileError> {
        let documents = self.pipeline.gather_documents(documents);
        self.map_artifact_result(self.pipeline.run_to_lane(documents))
    }

    fn emit(
        &self,
        documents: Vec<DocumentIr>,
        plan: &ArtifactPlan,
        owners: &worklist::ErrorOwners,
    ) -> Result<EmittedArtifact, ArtifactCompileError> {
        let documents = self.pipeline.gather_documents(documents);
        match self.pipeline.run_to_emitted(documents) {
            Ok(emitted) => Ok(emitted),
            Err(CompilerPipelineError::Segment(PassSegmentError::PassFailed { pass, source }))
                if pass.as_str() == CLOSE_PASS_NAME =>
            {
                source
                    .downcast::<UseGraphError>()
                    .map(|error| {
                        Err(driver::attribute_compile_error(
                            crate::pipeline::CompileError::UseGraph(*error),
                            plan,
                            owners,
                            None,
                        ))
                    })
                    .unwrap_or_else(|source| unexpected_pass_error(&pass, source))
            }
            Err(CompilerPipelineError::Segment(PassSegmentError::PassFailed { pass, source }))
                if pass.as_str() == MERGE_PASS_NAME =>
            {
                source
                    .downcast::<MergePassError>()
                    .map(|error| {
                        Err(driver::attribute_compile_error(
                            error.into_compile_error(),
                            plan,
                            owners,
                            None,
                        ))
                    })
                    .unwrap_or_else(|source| unexpected_pass_error(&pass, source))
            }
            Err(CompilerPipelineError::Segment(PassSegmentError::PassFailed { pass, source }))
                if pass.as_str() == EMBED_PASS_NAME =>
            {
                source
                    .downcast::<EmbedPassError>()
                    .map(|error| {
                        Err(driver::attribute_compile_error(
                            error.into_compile_error(),
                            plan,
                            owners,
                            None,
                        ))
                    })
                    .unwrap_or_else(|source| unexpected_pass_error(&pass, source))
            }
            Err(CompilerPipelineError::Segment(PassSegmentError::PassFailed { pass, source }))
                if pass.as_str() == LINK_PASS_NAME =>
            {
                source
                    .downcast::<LinkPassError>()
                    .map(|error| {
                        let input = match error.as_ref() {
                            LinkPassError::AmbiguousShortLink { contribution, .. } => {
                                Some(*contribution)
                            }
                            _ => None,
                        };
                        Err(driver::attribute_compile_error(
                            error.into_compile_error(),
                            plan,
                            owners,
                            input,
                        ))
                    })
                    .unwrap_or_else(|source| unexpected_pass_error(&pass, source))
            }
            Err(CompilerPipelineError::Segment(PassSegmentError::PassFailed { pass, source })) => {
                match source.downcast::<EmitPassError>() {
                    Ok(error) => Err(ArtifactCompileError::Backend {
                        pass: pass.as_str().to_string(),
                        reason: error.to_string(),
                    }),
                    Err(source) => Err(ArtifactCompileError::Pass {
                        pass: pass.as_str().to_string(),
                        reason: source.to_string(),
                    }),
                }
            }
            Err(error) => Err(ArtifactCompileError::Manager {
                reason: error.to_string(),
            }),
        }
    }

    fn map_artifact_result<T>(
        &self,
        result: Result<T, CompilerPipelineError>,
    ) -> Result<T, crate::pipeline::CompileError> {
        match result {
            Ok(closure) => Ok(closure),
            Err(CompilerPipelineError::Segment(PassSegmentError::PassFailed { pass, source }))
                if pass.as_str() == CLOSE_PASS_NAME =>
            {
                source
                    .downcast::<UseGraphError>()
                    .map(|error| Err(crate::pipeline::CompileError::UseGraph(*error)))
                    .unwrap_or_else(|source| {
                        panic!("the close pass returned an unexpected error type: {source}")
                    })
            }
            Err(CompilerPipelineError::Segment(PassSegmentError::PassFailed { pass, source }))
                if pass.as_str() == MERGE_PASS_NAME =>
            {
                source
                    .downcast::<MergePassError>()
                    .map(|error| Err(error.into_compile_error()))
                    .unwrap_or_else(|source| {
                        panic!("the merge pass returned an unexpected error type: {source}")
                    })
            }
            Err(CompilerPipelineError::Segment(PassSegmentError::PassFailed { pass, source }))
                if pass.as_str() == EMBED_PASS_NAME =>
            {
                source
                    .downcast::<EmbedPassError>()
                    .map(|error| Err(error.into_compile_error()))
                    .unwrap_or_else(|source| {
                        panic!("the embed pass returned an unexpected error type: {source}")
                    })
            }
            Err(CompilerPipelineError::Segment(PassSegmentError::PassFailed { pass, source }))
                if pass.as_str() == LINK_PASS_NAME =>
            {
                source
                    .downcast::<LinkPassError>()
                    .map(|error| Err(error.into_compile_error()))
                    .unwrap_or_else(|source| {
                        panic!("the link pass returned an unexpected error type: {source}")
                    })
            }
            Err(CompilerPipelineError::Segment(PassSegmentError::PassFailed { pass, source }))
                if pass.as_str() == ASSEMBLE_PASS_NAME =>
            {
                let error = source
                    .downcast::<AssemblePassError>()
                    .unwrap_or_else(|source| {
                        panic!("the assemble pass returned an unexpected error type: {source}")
                    });
                panic!("the built-in assemble pass rejected validated compiler state: {error}")
            }
            Err(error) => panic!("the private built-in artifact schedule is invalid: {error}"),
        }
    }
}

fn unexpected_pass_error<T>(
    pass: &PassName,
    source: Box<dyn std::error::Error + Send + Sync>,
) -> Result<T, ArtifactCompileError> {
    Err(ArtifactCompileError::Pass {
        pass: pass.as_str().to_string(),
        reason: source.to_string(),
    })
}

/// Compile one validated whole artifact plan through the artifact prefix.
///
/// Parse-dependent discovery invokes the document segment per honest document,
/// crosses one gather barrier, then the whole artifact segment runs once.
pub(crate) fn compile_artifact_prefix(
    plan: ArtifactPlan,
    source: &impl SectionSource,
) -> Result<ClosureIr, crate::pipeline::CompileError> {
    let schedule = BuiltinSchedule::linked(&plan);
    let worklist = worklist::discover(
        &plan,
        source,
        |input| schedule.parse_source(input),
        |address, reason| schedule.record_failure(address, reason),
    );
    schedule.close_state.set_pending_sources(worklist.sources);
    schedule.close_state.set_pending_embeds(worklist.embeds);
    schedule.close(worklist.documents)
}

/// Compile one validated whole artifact through the real Lane boundary.
///
/// This is crate-private until named emit and the workspace binder land. It is
/// production code, not a test-only hand-built IR path.
pub(crate) fn compile_artifact_lane(
    plan: ArtifactPlan,
    source: &impl SectionSource,
) -> Result<LaneIr, crate::pipeline::CompileError> {
    let schedule = BuiltinSchedule::assembled(&plan);
    let worklist = worklist::discover(
        &plan,
        source,
        |input| schedule.parse_source(input),
        |address, reason| schedule.record_failure(address, reason),
    );
    schedule.close_state.set_pending_sources(worklist.sources);
    schedule.close_state.set_pending_embeds(worklist.embeds);
    schedule.assemble(worklist.documents)
}

/// The one-normal compatibility adapter used by the public `compile_static*`
/// signatures. It is not the product's final whole-artifact driver.
pub(crate) fn compile_linked_closure(
    seed: &SpecAddress,
    source: &impl SectionSource,
    mode: StaticCompileMode,
) -> Result<ClosureIr, crate::pipeline::CompileError> {
    compile_artifact_prefix(ArtifactPlan::compatibility(seed.clone(), mode), source)
}

#[cfg(test)]
mod tests {
    use specmark::verifies;

    use super::*;
    use crate::SpecAddress;
    use crate::compiler::ir::{DocumentAddress, SourceFormatId};
    use crate::compiler::pass::{IrPayload, PassSegmentError};
    use crate::compiler::pipeline::{CompilerPipelineError, ScheduleItem};

    fn source(format: &str, text: &str) -> SourceIr {
        SourceIr::new(
            DocumentAddress::Spec(
                SpecAddress::parse("spec://org.demo/pkg/common/doc#root").unwrap(),
            ),
            SourceFormatId::new(format).unwrap(),
            text,
        )
    }

    fn seed() -> SpecAddress {
        SpecAddress::parse("spec://org.demo/pkg/common/doc#root").unwrap()
    }

    fn plan(mode: StaticCompileMode) -> ArtifactPlan {
        ArtifactPlan::compatibility(seed(), mode)
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
    fn production_lane_declares_parse_gather_close_merge_embed_qualify_absorb_link_assemble() {
        let pipeline = BuiltinSchedule::assembled(&plan(StaticCompileMode::Plain)).pipeline;
        let schedule = pipeline.schedule();

        assert!(matches!(
            schedule.as_slice(),
            [
                ScheduleItem::Pass(parse),
                ScheduleItem::GatherDocuments,
                ScheduleItem::Pass(close),
                ScheduleItem::Pass(merge),
                ScheduleItem::Pass(embed),
                ScheduleItem::Pass(qualify),
                ScheduleItem::Pass(absorb),
                ScheduleItem::Pass(link),
                ScheduleItem::Pass(assemble),
            ] if parse.name.as_str() == PARSE_PASS_NAME
                && parse.input == SourceIr::SHAPE
                && parse.output == DocumentIr::SHAPE
                && close.name.as_str() == CLOSE_PASS_NAME
                && close.input == super::super::ir::Documents::SHAPE
                && close.output == ClosureIr::SHAPE
                && merge.name.as_str() == MERGE_PASS_NAME
                && merge.input == ClosureIr::SHAPE
                && merge.output == ClosureIr::SHAPE
                && embed.name.as_str() == EMBED_PASS_NAME
                && embed.input == ClosureIr::SHAPE
                && embed.output == ClosureIr::SHAPE
                && qualify.name.as_str() == QUALIFY_PASS_NAME
                && qualify.input == ClosureIr::SHAPE
                && qualify.output == ClosureIr::SHAPE
                && absorb.name.as_str() == ABSORB_PASS_NAME
                && absorb.input == ClosureIr::SHAPE
                && absorb.output == ClosureIr::SHAPE
                && link.name.as_str() == LINK_PASS_NAME
                && link.input == ClosureIr::SHAPE
                && link.output == ClosureIr::SHAPE
                && assemble.name.as_str() == ASSEMBLE_PASS_NAME
                && assemble.input == ClosureIr::SHAPE
                && assemble.output == LaneIr::SHAPE
        ));
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-LEVELS")]
    fn parse_runs_once_for_each_addressed_document_then_gathers() {
        let schedule = BuiltinSchedule::linked(&plan(StaticCompileMode::Plain));
        let documents = schedule
            .pipeline
            .run_documents(vec![
                source(MARKDOWN_FORMAT, "# One {#one}\n"),
                source(MARKDOWN_FORMAT, "# Two {#two}\n"),
            ])
            .unwrap();

        assert_eq!(documents.len(), 2);
        assert!(
            documents
                .iter()
                .any(|document| document.tree().find_by_anchor("one").is_some())
        );
        assert!(
            documents
                .iter()
                .any(|document| document.tree().find_by_anchor("two").is_some())
        );
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
    fn removing_parse_makes_the_production_schedule_unrunnable() {
        let error = CompilerPipeline::default()
            .run_documents(vec![source(MARKDOWN_FORMAT, "# Doc {#root}\n")])
            .unwrap_err();

        assert!(matches!(
            error,
            CompilerPipelineError::ScheduleBoundary {
                boundary: "document segment input",
                ..
            }
        ));
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
    fn parse_failure_keeps_the_pass_name_and_concrete_source() {
        let error = BuiltinSchedule::linked(&plan(StaticCompileMode::Plain))
            .pipeline
            .run_documents(vec![source("unsupported", "body")])
            .unwrap_err();
        let CompilerPipelineError::Segment(PassSegmentError::PassFailed { pass, source }) = error
        else {
            panic!("expected the parse pass failure")
        };

        assert_eq!(pass.as_str(), PARSE_PASS_NAME);
        let parse = source
            .downcast_ref::<ParseError>()
            .expect("the concrete parse error must survive manager attribution");
        assert_eq!(parse.format, "unsupported");
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
    fn removing_close_makes_the_gathered_schedule_unrunnable() {
        let mut pipeline = CompilerPipeline::default();
        pipeline.push_document(ParsePass::new()).unwrap();
        let documents = pipeline
            .run_documents(vec![source(MARKDOWN_FORMAT, "# Doc {#root}\n")])
            .unwrap();

        let error = pipeline.run_to_closure(documents).unwrap_err();
        assert!(matches!(
            error,
            CompilerPipelineError::ScheduleBoundary {
                boundary: "artifact segment input",
                ..
            }
        ));
    }
}
