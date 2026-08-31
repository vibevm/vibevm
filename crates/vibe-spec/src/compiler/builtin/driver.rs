use crate::{CompileObserver, SectionSource, SpecAddress};

#[cfg(feature = "test-support")]
use super::super::backend::BackendId;
use super::super::backend::BackendRegistry;
use super::super::ir::{ArtifactInputWitness, ArtifactPlan, EmittedArtifact, StaticCompileMode};
use super::super::observer::Observing;
use super::super::trace::CompileTraceSink;
use super::super::transform::native_manager::CompilerNativeInvoker;
use super::super::transform::native_policy::session::{NativePolicyResult, NativePolicySession};
use super::super::transform::native_policy::{
    CompilerInvocationReceipts, CompilerNativeOutcome, CompilerNativePolicy,
    CompilerNativePolicyError,
};
use super::super::transform::registry::TransformRegistry;
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
    #[error(transparent)]
    Transform(#[from] super::TransformCompileError),
    #[error(transparent)]
    NativePolicy(#[from] CompilerNativePolicyError),
}

pub fn compile_artifact(
    plan: ArtifactPlan,
    source: &impl SectionSource,
) -> Result<EmittedArtifact, ArtifactCompileError> {
    run_with_registries(
        plan,
        source,
        &BackendRegistry::builtins(),
        &TransformRegistry::builtins(),
        None,
        None,
        None,
    )
}

/// [`compile_artifact`] under one analyzer observer (R4.3, the
/// packages-2026-09 architecture §9): the observer receives one emission
/// event per accepted artifact and one stage-delta event per lane- or
/// emitted-position transform that ran.
///
/// The same law the traced sibling carries, transposed to this seam: the
/// observer is a witness, never a veto — its evidence cannot alter the
/// artifact or any error identity, and the bytes are the bytes
/// [`compile_artifact`] would have produced with no observer at all.
/// Nothing is persisted (the frozen §9.1 ruling): the events are values
/// handed to one in-process observer for the process's lifetime.
///
/// The observer is deliberately NOT part of [`ArtifactPlan`], for the
/// same reason the trace sink is not: a plan is a semantic, digested
/// value, and an observer is neither.
pub fn compile_artifact_observed(
    plan: ArtifactPlan,
    source: &impl SectionSource,
    observer: std::sync::Arc<dyn CompileObserver>,
) -> Result<EmittedArtifact, ArtifactCompileError> {
    run_with_registries(
        plan,
        source,
        &BackendRegistry::builtins(),
        &TransformRegistry::builtins(),
        None,
        Some(observer),
        None,
    )
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
    run_with_registries(
        plan,
        source,
        &BackendRegistry::builtins(),
        &TransformRegistry::builtins(),
        Some(sink),
        None,
        None,
    )
}

/// Compile through the same plain schedule with a stack-borrowed native
/// invoker available to native plan rows.
pub fn compile_artifact_native(
    plan: ArtifactPlan,
    source: &impl SectionSource,
    invoker: &dyn CompilerNativeInvoker,
) -> Result<EmittedArtifact, ArtifactCompileError> {
    run_with_registries(
        plan,
        source,
        &BackendRegistry::builtins(),
        &TransformRegistry::builtins(),
        None,
        None,
        Some(invoker),
    )
}

/// Native-aware compile under the existing diagnostic trace observer.
pub fn compile_artifact_native_traced(
    plan: ArtifactPlan,
    source: &impl SectionSource,
    invoker: &dyn CompilerNativeInvoker,
    sink: &dyn CompileTraceSink,
) -> Result<EmittedArtifact, ArtifactCompileError> {
    run_with_registries(
        plan,
        source,
        &BackendRegistry::builtins(),
        &TransformRegistry::builtins(),
        Some(sink),
        None,
        Some(invoker),
    )
}

/// Native-aware compile under the existing analyzer observer.
pub fn compile_artifact_native_observed(
    plan: ArtifactPlan,
    source: &impl SectionSource,
    invoker: &dyn CompilerNativeInvoker,
    observer: std::sync::Arc<dyn CompileObserver>,
) -> Result<EmittedArtifact, ArtifactCompileError> {
    run_with_registries(
        plan,
        source,
        &BackendRegistry::builtins(),
        &TransformRegistry::builtins(),
        None,
        Some(observer),
        Some(invoker),
    )
}

/// Native-aware compile under an explicit pending-state policy.
pub fn compile_artifact_native_managed(
    plan: ArtifactPlan,
    source: &impl SectionSource,
    invoker: &dyn CompilerNativeInvoker,
    policy: CompilerNativePolicy,
) -> Result<CompilerNativeOutcome, ArtifactCompileError> {
    run_managed_with_registries(
        plan,
        source,
        &BackendRegistry::builtins(),
        &TransformRegistry::builtins(),
        None,
        None,
        ManagedNative { invoker, policy },
    )
}

/// Managed native compile under the existing diagnostic trace observer.
pub fn compile_artifact_native_managed_traced(
    plan: ArtifactPlan,
    source: &impl SectionSource,
    invoker: &dyn CompilerNativeInvoker,
    policy: CompilerNativePolicy,
    sink: &dyn CompileTraceSink,
) -> Result<CompilerNativeOutcome, ArtifactCompileError> {
    run_managed_with_registries(
        plan,
        source,
        &BackendRegistry::builtins(),
        &TransformRegistry::builtins(),
        Some(sink),
        None,
        ManagedNative { invoker, policy },
    )
}

/// Managed native compile under the existing analyzer observer.
pub fn compile_artifact_native_managed_observed(
    plan: ArtifactPlan,
    source: &impl SectionSource,
    invoker: &dyn CompilerNativeInvoker,
    policy: CompilerNativePolicy,
    observer: std::sync::Arc<dyn CompileObserver>,
) -> Result<CompilerNativeOutcome, ArtifactCompileError> {
    run_managed_with_registries(
        plan,
        source,
        &BackendRegistry::builtins(),
        &TransformRegistry::builtins(),
        None,
        Some(observer),
        ManagedNative { invoker, policy },
    )
}

pub(crate) fn compile_artifact_with_registry(
    plan: ArtifactPlan,
    source: &impl SectionSource,
    registry: &BackendRegistry,
) -> Result<EmittedArtifact, ArtifactCompileError> {
    run_with_registries(
        plan,
        source,
        registry,
        &TransformRegistry::builtins(),
        None,
        None,
        None,
    )
}

/// The cfg-test dual-registry seam: the one way a test injects T5's identity
/// catalog into a whole-artifact compile. Production always pins
/// [`TransformRegistry::builtins`]; this never widens into
/// `feature = "test-support"`.
#[cfg(test)]
pub(crate) fn compile_artifact_with_registries(
    plan: ArtifactPlan,
    source: &impl SectionSource,
    backends: &BackendRegistry,
    transforms: &TransformRegistry,
) -> Result<EmittedArtifact, ArtifactCompileError> {
    run_with_registries(plan, source, backends, transforms, None, None, None)
}

/// The cfg-test traced dual-registry seam, so a transform pass name can be
/// proven to survive trace identity end to end.
#[cfg(test)]
pub(crate) fn compile_artifact_traced_with_registries(
    plan: ArtifactPlan,
    source: &impl SectionSource,
    backends: &BackendRegistry,
    transforms: &TransformRegistry,
    trace: Option<&dyn CompileTraceSink>,
) -> Result<EmittedArtifact, ArtifactCompileError> {
    run_with_registries(plan, source, backends, transforms, trace, None, None)
}

/// The cfg-test observed dual-registry seam, so the analyzer's evidence —
/// attribution counts and stage deltas — can be exercised over the same
/// injected identity/mutating catalogs the execution tests use, before
/// any real behavior enters the production catalog.
#[cfg(test)]
pub(crate) fn compile_artifact_observed_with_registries(
    plan: ArtifactPlan,
    source: &impl SectionSource,
    backends: &BackendRegistry,
    transforms: &TransformRegistry,
    observer: std::sync::Arc<dyn CompileObserver>,
) -> Result<EmittedArtifact, ArtifactCompileError> {
    run_with_registries(
        plan,
        source,
        backends,
        transforms,
        None,
        Some(observer),
        None,
    )
}

/// The cfg-test native-aware dual-registry seam: mixed builtin/native plans
/// execute against one borrowed invoker without widening registry injection
/// into the production API.
#[cfg(test)]
pub(crate) fn compile_artifact_native_with_registries(
    plan: ArtifactPlan,
    source: &impl SectionSource,
    backends: &BackendRegistry,
    transforms: &TransformRegistry,
    invoker: &dyn CompilerNativeInvoker,
) -> Result<EmittedArtifact, ArtifactCompileError> {
    run_with_registries(
        plan,
        source,
        backends,
        transforms,
        None,
        None,
        Some(invoker),
    )
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
    let schedule =
        BuiltinSchedule::with_backend(&plan, &TransformRegistry::builtins(), implementation)?;
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

fn run_with_registries(
    plan: ArtifactPlan,
    source: &impl SectionSource,
    backends: &BackendRegistry,
    transforms: &TransformRegistry,
    trace: Option<&dyn CompileTraceSink>,
    observer: Observing,
    invoker: Option<&dyn CompilerNativeInvoker>,
) -> Result<EmittedArtifact, ArtifactCompileError> {
    let schedule = BuiltinSchedule::emitted_with_invoker(
        &plan, transforms, backends, &observer, invoker, None,
    )?;
    run(plan, source, schedule, trace)
}

struct ManagedNative<'invoke> {
    invoker: &'invoke dyn CompilerNativeInvoker,
    policy: CompilerNativePolicy,
}

fn run_managed_with_registries(
    plan: ArtifactPlan,
    source: &impl SectionSource,
    backends: &BackendRegistry,
    transforms: &TransformRegistry,
    trace: Option<&dyn CompileTraceSink>,
    observer: Observing,
    managed: ManagedNative<'_>,
) -> Result<CompilerNativeOutcome, ArtifactCompileError> {
    let session = NativePolicySession::new(plan.transforms(), managed.policy)?;
    let schedule = BuiltinSchedule::emitted_with_invoker(
        &plan,
        transforms,
        backends,
        &observer,
        Some(managed.invoker),
        Some(&session),
    )?;
    let artifact = run(plan, source, schedule, trace)?;
    match session.finish()? {
        NativePolicyResult::Fail => Ok(CompilerNativeOutcome::ready(
            artifact,
            CompilerInvocationReceipts::empty(),
        )),
        NativePolicyResult::Collected(pending) if pending.is_empty() => Ok(
            CompilerNativeOutcome::ready(artifact, CompilerInvocationReceipts::empty()),
        ),
        NativePolicyResult::Collected(pending) => {
            Ok(CompilerNativeOutcome::pending(artifact, pending))
        }
        NativePolicyResult::Resolved(receipts) => {
            Ok(CompilerNativeOutcome::ready(artifact, receipts))
        }
    }
}

fn run(
    plan: ArtifactPlan,
    source: &impl SectionSource,
    schedule: BuiltinSchedule<'_>,
    trace: Option<&dyn CompileTraceSink>,
) -> Result<EmittedArtifact, ArtifactCompileError> {
    let worklist = worklist::discover(
        &plan,
        source,
        |input| schedule.parse_source(input, trace),
        |address, reason| schedule.record_failure(address, reason),
    )
    .map_err(|error| schedule.document_error(error))?;
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

/// Fold one artifact-level error into the legacy public compile error.
///
/// The compatibility path always compiles an EMPTY transform plan, so the
/// transform family is unreachable here by construction — the panic arm
/// keeps exactly the reachability it had before the family existed.
pub(super) fn into_compatibility_error(
    error: ArtifactCompileError,
) -> crate::pipeline::CompileError {
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
