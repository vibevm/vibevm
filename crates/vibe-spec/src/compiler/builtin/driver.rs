use crate::{SectionSource, SpecAddress};

#[cfg(feature = "test-support")]
use super::super::backend::BackendId;
use super::super::backend::BackendRegistry;
use super::super::ir::{ArtifactInputWitness, ArtifactPlan, EmittedArtifact, StaticCompileMode};
use super::super::trace::CompileTraceSink;
use super::super::worklist::{self, ErrorOwners};
use super::BuiltinSchedule;

#[derive(Debug, thiserror::Error)]
pub enum ArtifactCompileError {
    #[error("artifact input {} ({}) failed: {source}", input.index, input.origin)]
    Input {
        input: ArtifactInputWitness,
        #[source]
        source: Box<ArtifactCompileError>,
    },
    #[error(transparent)]
    Compile(#[from] crate::pipeline::CompileError),
    #[error("compiler backend registry failed: {reason}")]
    Registry { reason: String },
    #[error("compiler pass `{pass}` failed: {reason}")]
    Pass { pass: String, reason: String },
    #[error("compiler backend pass `{pass}` failed: {reason}")]
    Backend { pass: String, reason: String },
    #[error("compiler manager failed: {reason}")]
    Manager { reason: String },
}

pub fn compile_artifact(
    plan: ArtifactPlan,
    source: &impl SectionSource,
) -> Result<EmittedArtifact, ArtifactCompileError> {
    compile_artifact_with_registry(plan, source, &BackendRegistry::builtins())
}

/// [`compile_artifact`] under one diagnostic observer (PROP-054 `##OBS-TRACE`).
///
/// The sink sees exactly one event per attempted pass of the schedule this
/// artifact really runs — every worklist `parse` included — and the exact
/// pretty `compiler_ir/e1` bytes of every accepted output. It is a witness,
/// never a veto: an encode refusal becomes a `snapshot-failed` event, and the
/// returned artifact and every error identity are those of
/// [`compile_artifact`] on the same inputs.
///
/// The sink is deliberately NOT part of [`ArtifactPlan`]: a plan is a
/// semantic, digested value, and an observer is neither.
pub fn compile_artifact_traced(
    plan: ArtifactPlan,
    source: &impl SectionSource,
    sink: &dyn CompileTraceSink,
) -> Result<EmittedArtifact, ArtifactCompileError> {
    compile_artifact_traced_with_registry(plan, source, &BackendRegistry::builtins(), Some(sink))
}

pub(crate) fn compile_artifact_with_registry(
    plan: ArtifactPlan,
    source: &impl SectionSource,
    registry: &BackendRegistry,
) -> Result<EmittedArtifact, ArtifactCompileError> {
    compile_artifact_traced_with_registry(plan, source, registry, None)
}

pub(crate) fn compile_artifact_traced_with_registry(
    plan: ArtifactPlan,
    source: &impl SectionSource,
    registry: &BackendRegistry,
    trace: Option<&dyn CompileTraceSink>,
) -> Result<EmittedArtifact, ArtifactCompileError> {
    let schedule = BuiltinSchedule::emitted(&plan, registry).map_err(|error| {
        ArtifactCompileError::Registry {
            reason: error.to_string(),
        }
    })?;
    run(plan, source, schedule, trace)
}

#[cfg(feature = "test-support")]
pub(crate) fn compile_artifact_with_backend_id(
    plan: ArtifactPlan,
    source: &impl SectionSource,
    registry: &BackendRegistry,
    backend: &BackendId,
) -> Result<EmittedArtifact, ArtifactCompileError> {
    let implementation = registry
        .get(backend)
        .map_err(|error| ArtifactCompileError::Registry {
            reason: error.to_string(),
        })?;
    let schedule = BuiltinSchedule::with_backend(&plan, implementation);
    run(plan, source, schedule, None)
}

/// Retarget one plan onto a custom test backend, forwarding the COMPLETE
/// artifact plan: the context is rebuilt for the custom target while the
/// carried transform plan crosses intact. Rebuilding from contributions
/// alone would silently drop a nonempty plan — the one carriage regression
/// T4 must make red (ABI §7.1).
#[cfg(any(test, feature = "test-support"))]
pub(super) fn retarget_custom_for_test(
    backend: &'static str,
    plan: ArtifactPlan,
) -> Result<ArtifactPlan, ArtifactCompileError> {
    let carried = plan.transforms().clone();
    ArtifactPlan::custom_for_test(backend, plan.contributions().to_vec())
        .map(|retargeted| retargeted.with_transforms(carried))
        .map_err(|error| ArtifactCompileError::Manager {
            reason: error.to_string(),
        })
}

#[cfg(feature = "test-support")]
#[doc(hidden)]
pub fn compile_artifact_opaque_test_vehicle(
    plan: ArtifactPlan,
    source: &impl SectionSource,
) -> Result<EmittedArtifact, ArtifactCompileError> {
    let plan = retarget_custom_for_test("opaque-test", plan)?;
    let mut registry = BackendRegistry::default();
    registry
        .register(std::sync::Arc::new(
            super::super::emit::opaque_test_vehicle::OpaqueTestBackend::new(),
        ))
        .map_err(|error| ArtifactCompileError::Registry {
            reason: error.to_string(),
        })?;
    compile_artifact_with_backend_id(
        plan,
        source,
        &registry,
        &BackendId::new("opaque-test").expect("test vehicle id is valid"),
    )
}

#[cfg(feature = "test-support")]
#[doc(hidden)]
pub fn compile_artifact_missing_backend_test_vehicle(
    plan: ArtifactPlan,
    source: &impl SectionSource,
) -> Result<EmittedArtifact, ArtifactCompileError> {
    compile_artifact_with_registry(plan, source, &BackendRegistry::default())
}

#[cfg(feature = "test-support")]
#[doc(hidden)]
pub fn compile_artifact_replacement_test_vehicle(
    plan: ArtifactPlan,
    source: &impl SectionSource,
) -> Result<EmittedArtifact, ArtifactCompileError> {
    let mut registry = BackendRegistry::builtins();
    registry
        .replace(std::sync::Arc::new(
            super::super::emit::opaque_test_vehicle::OpaqueTestBackend::replacement(),
        ))
        .map_err(|error| ArtifactCompileError::Registry {
            reason: error.to_string(),
        })?;
    compile_artifact_with_registry(plan, source, &registry)
}

fn run(
    plan: ArtifactPlan,
    source: &impl SectionSource,
    schedule: BuiltinSchedule,
    trace: Option<&dyn CompileTraceSink>,
) -> Result<EmittedArtifact, ArtifactCompileError> {
    let worklist = super::infallible_worklist(worklist::discover(
        &plan,
        source,
        |input| Ok(schedule.parse_source(input, trace)),
        |address, reason| schedule.record_failure(address, reason),
    ));
    schedule.close_state.set_pending_sources(worklist.sources);
    schedule.close_state.set_pending_embeds(worklist.embeds);
    schedule.emit(worklist.documents, &plan, &worklist.owners, trace)
}

pub(crate) fn compile_compatibility_artifact(
    seed: &SpecAddress,
    source: &impl SectionSource,
    mode: StaticCompileMode,
) -> Result<EmittedArtifact, crate::pipeline::CompileError> {
    match compile_artifact(ArtifactPlan::compatibility(seed.clone(), mode), source) {
        Ok(emitted) => Ok(emitted),
        Err(error) => Err(into_compatibility_error(error)),
    }
}

fn into_compatibility_error(error: ArtifactCompileError) -> crate::pipeline::CompileError {
    match error {
        ArtifactCompileError::Compile(error) => error,
        ArtifactCompileError::Input { source, .. } => into_compatibility_error(*source),
        error => panic!("the built-in compatibility backend failed: {error}"),
    }
}

pub(super) fn attribute_compile_error(
    error: crate::pipeline::CompileError,
    plan: &ArtifactPlan,
    owners: &ErrorOwners,
    explicit_input: Option<usize>,
) -> ArtifactCompileError {
    let owner = explicit_input.or_else(|| {
        error
            .attribution_address()
            .and_then(|address| owners.owner(address))
    });
    let base = ArtifactCompileError::Compile(error);
    match owner.and_then(|index| plan.input_witness(index)) {
        Some(input) => ArtifactCompileError::Input {
            input,
            source: Box::new(base),
        },
        None => base,
    }
}
