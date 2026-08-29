//! Built-in passes and the declared schedule prefix migrated so far.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR");

use crate::{DocTree, SectionSource, SpecAddress};

#[cfg(test)]
use super::absorb::ABSORB_PASS_NAME;
use super::absorb::AbsorbPass;
#[cfg(test)]
use super::assemble::ASSEMBLE_PASS_NAME;
use super::assemble::AssemblePass;
use super::backend::{BackendRegistry, EmitBackend};
#[cfg(test)]
use super::close::CLOSE_PASS_NAME;
use super::close::{ClosePass, CloseState};
#[cfg(test)]
use super::embed::EMBED_PASS_NAME;
use super::embed::EmbedPass;
use super::emit::EmitPass;
use super::ir::{
    ArtifactPlan, ClosureIr, DocumentIr, EmittedArtifact, LaneIr, SourceIr, StaticCompileMode,
};
#[cfg(test)]
use super::link::LINK_PASS_NAME;
use super::link::LinkPass;
#[cfg(test)]
use super::merge::MERGE_PASS_NAME;
use super::merge::MergePass;
use super::pass::{Pass, PassName, PassSegmentError};
use super::pipeline::{CompilerPipeline, CompilerPipelineError};
#[cfg(test)]
use super::qualify::QUALIFY_PASS_NAME;
use super::qualify::QualifyPass;
use super::trace::CompileTraceSink;
use super::transform::fault::TransformError;
use super::transform::header as transform_header;
use super::transform::registry::TransformRegistry;
use super::transform::schedule::TransformSchedule;
use super::worklist;

mod attribution;
mod driver;
pub use super::transform::fault::TransformCompileError;
#[cfg(test)]
pub(crate) use driver::compile_artifact_traced_with_registries;
#[cfg(test)]
pub(crate) use driver::compile_artifact_with_registries;
#[cfg(test)]
pub(crate) use driver::compile_artifact_with_registry;
pub(crate) use driver::compile_compatibility_artifact;
pub use driver::{ArtifactCompileError, compile_artifact, compile_artifact_traced};
#[cfg(feature = "test-support")]
pub use driver::{
    compile_artifact_missing_backend_test_vehicle, compile_artifact_opaque_test_vehicle,
    compile_artifact_replacement_test_vehicle,
};

const PARSE_PASS_NAME: &str = "parse";
const MARKDOWN_FORMAT: &str = "markdown";

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

// Whether a newly constructed built-in schedule turns the R3.3 verify-each
// seam on. It defaults to ON, so every existing unit test keeps crossing the
// real verifier exactly as before; thread-local because the suite runs tests
// in parallel and one scope must not disarm another test's verifier.
#[cfg(test)]
std::thread_local! {
    static VERIFY_EACH: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}

#[cfg(test)]
fn verify_each_enabled() -> bool {
    VERIFY_EACH.with(std::cell::Cell::get)
}

/// Build and run built-in schedules exactly as PRODUCTION builds them — with
/// the inter-pass verifier hook absent — for the duration of `body`.
///
/// This exists so a guarantee that must not depend on the optional verifier
/// can be tested against the construction production actually uses. The guard
/// restores the previous state on drop, so a failing assertion inside `body`
/// cannot leak the off state into the rest of the thread's work. It is
/// `#[cfg(test)]` and deliberately not widened into `feature = "test-support"`.
#[cfg(test)]
pub(crate) fn without_verify_each<T>(body: impl FnOnce() -> T) -> T {
    struct Restore(bool);

    impl Drop for Restore {
        fn drop(&mut self) {
            VERIFY_EACH.with(|flag| flag.set(self.0));
        }
    }

    let _restore = Restore(VERIFY_EACH.with(|flag| flag.replace(false)));
    body()
}

/// The declared built-in schedule prefix currently used by production.
///
/// Keeping construction in one function makes the list executable rather than
/// a registry beside a separate call path. R3.2 appends later built-ins here as
/// each phase migrates.
pub(crate) struct BuiltinSchedule {
    pipeline: CompilerPipeline,
    close_state: CloseState,
    transforms: TransformSchedule,
    transform_names: Vec<PassName>,
}

/// Wrap one internal transform fault as the public opaque artifact error.
/// The builtin cell owns this conversion (the transform cell names no
/// builtin type, and the builtin cell names no transform construction), and
/// it is the ONE conversion site: the attribution cell below reaches it
/// through `super` rather than restating it.
fn transform_public(inner: TransformError) -> ArtifactCompileError {
    ArtifactCompileError::Transform(TransformCompileError::new(inner))
}

impl BuiltinSchedule {
    fn linked(
        plan: &ArtifactPlan,
        registry: &TransformRegistry,
    ) -> Result<Self, ArtifactCompileError> {
        let transforms = TransformSchedule::resolve(plan, registry).map_err(transform_public)?;
        let close_state = CloseState::default();
        let mut pipeline = CompilerPipeline::default();
        transforms
            .push_source_before_parse(&mut pipeline)
            .map_err(transform_public)?;
        pipeline
            .push_document(ParsePass::new())
            .expect("the static built-in parse schedule is valid");
        transforms
            .push_document_after_parse(&mut pipeline)
            .map_err(transform_public)?;
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
        // The R3.3 test-only enabling seam: every built-in pass output crosses
        // the real verifier hook in unit tests. Production construction keeps
        // the verifier off, byte- and error-identical to before. One scoped
        // cfg-test switch ([`without_verify_each`]) turns it off so a test can
        // observe exactly the schedule production builds.
        #[cfg(test)]
        if verify_each_enabled() {
            pipeline.enable_verify_each_for_tests();
        }
        let transform_names = transforms.pass_names();
        Ok(Self {
            pipeline,
            close_state,
            transforms,
            transform_names,
        })
    }

    fn assembled(
        plan: &ArtifactPlan,
        registry: &TransformRegistry,
    ) -> Result<Self, ArtifactCompileError> {
        let mut schedule = Self::linked(plan, registry)?;
        schedule
            .pipeline
            .push_artifact(AssemblePass::new())
            .expect("the static built-in assemble schedule is valid");
        schedule
            .transforms
            .push_lane_after_assemble(&mut schedule.pipeline)
            .map_err(transform_public)?;
        Ok(schedule)
    }

    fn emitted(
        plan: &ArtifactPlan,
        transforms: &TransformRegistry,
        registry: &BackendRegistry,
    ) -> Result<Self, ArtifactCompileError> {
        // Transform resolution — including the compatibility-fragment frame
        // refusal — precedes the backend lookup, exactly as the frozen T6b
        // construction order demands.
        let schedule = Self::assembled(plan, transforms)?;
        let backend = registry
            .selected(&plan.context().target())
            .map_err(|error| ArtifactCompileError::Registry {
                reason: error.to_string(),
            })?;
        Self::append_emit(schedule, backend, plan)
    }

    /// The custom-backend construction path of the test-support vehicles;
    /// production always selects through [`Self::emitted`].
    #[cfg(feature = "test-support")]
    fn with_backend(
        plan: &ArtifactPlan,
        registry: &TransformRegistry,
        backend: std::sync::Arc<dyn EmitBackend>,
    ) -> Result<Self, ArtifactCompileError> {
        let schedule = Self::assembled(plan, registry)?;
        Self::append_emit(schedule, backend, plan)
    }

    /// Append the selected emit backend, and with it the artifact's ACTIVE
    /// transforms header (R4 architecture §7.1).
    ///
    /// The header payload is derived here, from the plan the artifact was
    /// compiled with, because this is the one place that holds both the
    /// artifact plan and the emit pass. It is engine framing — never plugin
    /// bytes — and an empty plan derives `None`, which is the exact
    /// historical byte stream.
    fn append_emit(
        mut schedule: Self,
        backend: std::sync::Arc<dyn EmitBackend>,
        plan: &ArtifactPlan,
    ) -> Result<Self, ArtifactCompileError> {
        let header = transform_header::transforms_header_payload(plan.transforms());
        schedule
            .pipeline
            .push_artifact(EmitPass::new(backend, header))
            .expect("the selected emit backend continues the built-in schedule");
        schedule
            .transforms
            .push_emitted_after_emit(&mut schedule.pipeline)
            .map_err(transform_public)?;
        Ok(schedule)
    }

    fn parse_source(
        &self,
        source: SourceIr,
        trace: Option<&dyn CompileTraceSink>,
    ) -> Result<DocumentIr, CompilerPipelineError> {
        self.pipeline.run_document_traced(source, trace)
    }

    fn record_failure(&self, address: &SpecAddress, reason: String) {
        self.close_state.record_failure(address, reason);
    }

    fn close(&self, documents: Vec<DocumentIr>) -> Result<ClosureIr, ArtifactCompileError> {
        let closure = self
            .pipeline
            .gather_documents(documents)
            .and_then(|documents| self.pipeline.run_to_closure(documents));
        self.map_artifact_result(closure)
    }

    fn assemble(&self, documents: Vec<DocumentIr>) -> Result<LaneIr, ArtifactCompileError> {
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
        trace: Option<&dyn CompileTraceSink>,
    ) -> Result<EmittedArtifact, ArtifactCompileError> {
        let documents = match self.pipeline.gather_documents(documents) {
            Ok(documents) => documents,
            Err(error) => {
                return Err(ArtifactCompileError::Manager {
                    reason: error.to_string(),
                });
            }
        };
        match self.pipeline.run_to_emitted(documents, trace) {
            Ok(emitted) => Ok(emitted),
            Err(error) => match self.transform_fault(error) {
                Ok(transform) => Err(transform_public(transform)),
                // The ONE shared classifier decided: not a transform fault.
                // The emitted path keeps its historical mapping below.
                Err(CompilerPipelineError::Segment(PassSegmentError::PassFailed {
                    pass,
                    source,
                })) => self.attribute_emit_pass_failure(pass, source, plan, owners),
                Err(CompilerPipelineError::Segment(PassSegmentError::VerificationFailed {
                    pass,
                    source,
                    ..
                })) => Err(ArtifactCompileError::Pass {
                    pass: pass.to_string(),
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
            },
        }
    }
}

/// Test-only construction of the REAL declared schedules, so a focused test
/// outside this module can observe the production pass list rather than a
/// hand-built imitation of it.
#[cfg(test)]
impl BuiltinSchedule {
    pub(crate) fn linked_for_test(
        plan: &ArtifactPlan,
        registry: &TransformRegistry,
    ) -> Result<Self, ArtifactCompileError> {
        Self::linked(plan, registry)
    }

    pub(crate) fn emitted_for_test(
        plan: &ArtifactPlan,
        registry: &TransformRegistry,
    ) -> Result<Self, ArtifactCompileError> {
        Self::emitted(plan, registry, &BackendRegistry::builtins())
    }

    pub(crate) fn pipeline_for_test(&self) -> &CompilerPipeline {
        &self.pipeline
    }
}

/// Compile one validated whole artifact plan through the artifact prefix.
///
/// Parse-dependent discovery invokes the document segment per honest document,
/// crosses one gather barrier, then the whole artifact segment runs once. A
/// document-segment failure — transform or otherwise — propagates through
/// T6a's fallible discovery with its typed fault intact.
pub(crate) fn compile_artifact_prefix(
    plan: ArtifactPlan,
    source: &impl SectionSource,
) -> Result<ClosureIr, ArtifactCompileError> {
    let schedule = BuiltinSchedule::linked(&plan, &TransformRegistry::builtins())?;
    let worklist = worklist::discover(
        &plan,
        source,
        |input| schedule.parse_source(input, None),
        |address, reason| schedule.record_failure(address, reason),
    )
    .map_err(|error| schedule.document_error(error))?;
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
) -> Result<LaneIr, ArtifactCompileError> {
    let schedule = BuiltinSchedule::assembled(&plan, &TransformRegistry::builtins())?;
    let worklist = worklist::discover(
        &plan,
        source,
        |input| schedule.parse_source(input, None),
        |address, reason| schedule.record_failure(address, reason),
    )
    .map_err(|error| schedule.document_error(error))?;
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
    match compile_artifact_prefix(ArtifactPlan::compatibility(seed.clone(), mode), source) {
        Ok(closure) => Ok(closure),
        Err(error) => Err(driver::into_compatibility_error(error)),
    }
}

#[cfg(test)]
#[path = "builtin/tests.rs"]
mod tests;
