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
        #[cfg(test)]
        // The R3.3 test-only enabling seam: every built-in pass output crosses
        // the real verifier hook in unit tests. Production construction keeps
        // the verifier off, byte- and error-identical to before.
        pipeline.enable_verify_each_for_tests();
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
        let backend = registry.selected(&plan.context().target())?;
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
        let closure = self
            .pipeline
            .gather_documents(documents)
            .and_then(|documents| self.pipeline.run_to_closure(documents));
        self.map_artifact_result(closure)
    }

    fn assemble(
        &self,
        documents: Vec<DocumentIr>,
    ) -> Result<LaneIr, crate::pipeline::CompileError> {
        let lane = self
            .pipeline
            .gather_documents(documents)
            .and_then(|documents| self.pipeline.run_to_lane(documents));
        self.map_artifact_result(lane)
    }

    fn emit(
        &self,
        documents: Vec<DocumentIr>,
        plan: &ArtifactPlan,
        owners: &worklist::ErrorOwners,
    ) -> Result<EmittedArtifact, ArtifactCompileError> {
        let documents = match self.pipeline.gather_documents(documents) {
            Ok(documents) => documents,
            Err(error) => {
                return Err(ArtifactCompileError::Manager {
                    reason: error.to_string(),
                });
            }
        };
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
            Err(CompilerPipelineError::Segment(PassSegmentError::VerificationFailed {
                pass,
                source,
                ..
            })) => Err(ArtifactCompileError::Pass {
                pass: pass.as_str().to_string(),
                reason: format!("inter-pass verification rejected the output: {source}"),
            }),
            Err(CompilerPipelineError::Segment(
                error @ PassSegmentError::InputVerification { .. },
            )) => Err(ArtifactCompileError::Manager {
                reason: format!("inter-pass verification rejected the segment input: {error}"),
            }),
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
            // Verification failures keep their honest pass attribution instead
            // of dissolving into the generic schedule panic below. Reachable
            // only under the test-only enabling seam; production construction
            // never produces these variants.
            Err(CompilerPipelineError::Segment(PassSegmentError::VerificationFailed {
                pass,
                source,
                ..
            })) => panic!("inter-pass verification rejected `{pass}` output: {source}"),
            Err(CompilerPipelineError::Segment(PassSegmentError::InputVerification {
                input,
                source,
            })) => panic!("inter-pass verification rejected the {input:?} segment input: {source}"),
            Err(CompilerPipelineError::GatherVerification { source }) => {
                panic!("inter-pass verification rejected the gather-documents boundary: {source}")
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
#[path = "builtin/tests.rs"]
mod tests;
