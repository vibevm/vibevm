//! One managed node/unit static compiler and its truthful outcome funnel.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#BOOTSTRAP-ORDER");

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use vibe_core::{layout, manifest::SpecFormat};
use vibe_spec::{
    ArtifactCompileError, ArtifactPlan, ArtifactTarget, CompileObserver,
    CompilerFinalizedPendingArtifact, CompilerInvocationReceipts, CompilerNativeOutcome,
    CompilerPendingArtifact, CompilerPendingFinalizeError, CompilerPendingSet, DocumentProvider,
    EmittedArtifact, FileResolver, FsSectionSource, SelfCoordinate, TransformPlan,
    compile_artifact, compile_artifact_native_managed, compile_artifact_native_managed_observed,
    compile_artifact_native_managed_traced, compile_artifact_observed, compile_artifact_traced,
    defer_emission, finalize_compiler_pending_artifact,
};

type PendingFinalizer =
    fn(
        CompilerPendingArtifact,
        &TransformPlan,
        &[u8; 32],
    ) -> Result<CompilerFinalizedPendingArtifact, CompilerPendingFinalizeError>;

use crate::boot::EffectiveBoot;
use crate::compile_trace::ScopeAcquisition;
use crate::compile_trace::TraceRun;
use crate::extension_world::{
    OwnerNativeCompileProvider, OwnerRuntimeId, OwnerRuntimeView, PendingArtifactEvidence,
    PendingArtifactTarget, build_pending_artifact_evidence,
};
use crate::{WorkspaceError, layout_paths};

use super::{
    INDEX_FILE, WrittenArtifacts, inputs, redirect, render_index_with_spec_format,
    stale_static_file, static_file, static_path, transaction,
};

#[path = "native_managed/replay.rs"]
mod replay;
pub(crate) use replay::compile_static_owner_managed_with_source;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OwnerNativeCompileStatus {
    Ready,
    Pending,
}

pub enum OwnerNativeCompileContinuation {
    Ready {
        receipts: CompilerInvocationReceipts,
    },
    Pending {
        evidence: PendingArtifactEvidence,
        pending: CompilerPendingSet,
    },
}

pub(crate) struct OwnerNativeCompileOutcome {
    artifact: EmittedArtifact,
    continuation: OwnerNativeCompileContinuation,
}

impl OwnerNativeCompileOutcome {
    #[must_use]
    pub const fn artifact(&self) -> &EmittedArtifact {
        &self.artifact
    }

    #[must_use]
    pub const fn status(&self) -> OwnerNativeCompileStatus {
        match self.continuation {
            OwnerNativeCompileContinuation::Ready { .. } => OwnerNativeCompileStatus::Ready,
            OwnerNativeCompileContinuation::Pending { .. } => OwnerNativeCompileStatus::Pending,
        }
    }

    #[must_use]
    pub fn pending(&self) -> Option<(&PendingArtifactEvidence, &CompilerPendingSet)> {
        match &self.continuation {
            OwnerNativeCompileContinuation::Pending { evidence, pending } => {
                Some((evidence, pending))
            }
            OwnerNativeCompileContinuation::Ready { .. } => None,
        }
    }

    #[must_use]
    pub fn into_parts(self) -> (EmittedArtifact, OwnerNativeCompileContinuation) {
        (self.artifact, self.continuation)
    }
}

impl fmt::Debug for OwnerNativeCompileOutcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OwnerNativeCompileOutcome")
            .field("status", &self.status())
            .finish_non_exhaustive()
    }
}

pub(crate) struct OwnerManagedStaticCompile {
    artifact: OwnerManagedArtifact,
    pub(crate) providers: Vec<Option<DocumentProvider>>,
}

enum OwnerManagedArtifact {
    Builtin(EmittedArtifact),
    Native(OwnerNativeCompileOutcome),
}

impl OwnerManagedStaticCompile {
    pub(crate) fn artifact(&self) -> &EmittedArtifact {
        match &self.artifact {
            OwnerManagedArtifact::Builtin(artifact) => artifact,
            OwnerManagedArtifact::Native(outcome) => outcome.artifact(),
        }
    }

    pub(crate) fn native(&self) -> Option<&OwnerNativeCompileOutcome> {
        match &self.artifact {
            OwnerManagedArtifact::Builtin(_) => None,
            OwnerManagedArtifact::Native(outcome) => Some(outcome),
        }
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        EmittedArtifact,
        Option<OwnerNativeCompileContinuation>,
        Vec<Option<DocumentProvider>>,
    ) {
        let (artifact, native) = match self.artifact {
            OwnerManagedArtifact::Builtin(artifact) => (artifact, None),
            OwnerManagedArtifact::Native(outcome) => {
                let (artifact, continuation) = outcome.into_parts();
                (artifact, Some(continuation))
            }
        };
        (artifact, native, self.providers)
    }
}

impl fmt::Debug for OwnerManagedStaticCompile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OwnerManagedStaticCompile")
            .field(
                "native",
                &self.native().map(OwnerNativeCompileOutcome::status),
            )
            .field("providers", &self.providers)
            .finish_non_exhaustive()
    }
}

pub(crate) enum OwnerNativeCompileMode<'a> {
    Plain,
    Traced(&'a ScopeAcquisition<'a>),
    Observed(Arc<dyn CompileObserver>),
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn compile_static_owner_managed<P: OwnerNativeCompileProvider>(
    boot: &EffectiveBoot,
    workspace_root: &Path,
    self_coord: &SelfCoordinate,
    spec_format: SpecFormat,
    owner: OwnerRuntimeView<'_>,
    mode: OwnerNativeCompileMode<'_>,
    provider: Option<&mut P>,
) -> Result<Option<OwnerManagedStaticCompile>, WorkspaceError> {
    compile_static_owner_managed_using(
        boot,
        workspace_root,
        self_coord,
        spec_format,
        owner,
        mode,
        provider,
        None,
        None,
        finalize_compiler_pending_artifact,
    )
}

#[allow(clippy::too_many_arguments)]
fn compile_static_owner_managed_using<P: OwnerNativeCompileProvider>(
    boot: &EffectiveBoot,
    workspace_root: &Path,
    self_coord: &SelfCoordinate,
    spec_format: SpecFormat,
    owner: OwnerRuntimeView<'_>,
    mode: OwnerNativeCompileMode<'_>,
    provider: Option<&mut P>,
    injected_source: Option<&FsSectionSource>,
    injected_overlay: Option<&BTreeMap<PathBuf, Arc<[u8]>>>,
    finalizer: PendingFinalizer,
) -> Result<Option<OwnerManagedStaticCompile>, WorkspaceError> {
    let entries = boot.static_entries().collect::<Vec<_>>();
    if entries.is_empty() {
        return Ok(None);
    }
    let (inputs, providers) = match injected_overlay {
        Some(overlay) => inputs::build_with_providers_overlay(
            entries,
            workspace_root,
            self_coord,
            Some(overlay),
        )?,
        None => inputs::build_with_providers(entries, workspace_root, self_coord)?,
    };
    let target = if matches!(spec_format, SpecFormat::Xml) {
        ArtifactTarget::StaticXml
    } else {
        ArtifactTarget::StaticMarkdown
    };
    let transforms = owner.runtime().transform_plan().clone();
    let plan = ArtifactPlan::static_lane(
        target,
        static_path(spec_format),
        layout_paths::vibedeps(""),
        inputs,
    )
    .map_err(|error| WorkspaceError::InlineCompile {
        reason: error.to_string(),
    })?
    .with_transforms(transforms.clone());
    let owned_source;
    let source = match injected_source {
        Some(source) => source,
        None => {
            owned_source =
                FsSectionSource::new(FileResolver::new(workspace_root, self_coord.clone()));
            &owned_source
        }
    };
    let owner_id = owner.runtime().id().clone();
    let has_native = owner.runtime().has_compiler_native_intersection()?;

    if !has_native {
        let scope = acquire(&mode);
        let result = compile_builtin(plan, source, mode, scope.as_ref());
        finish_artifact_scope(scope.as_ref(), &result);
        return result.map(|artifact| {
            Some(OwnerManagedStaticCompile {
                artifact: OwnerManagedArtifact::Builtin(artifact),
                providers,
            })
        });
    }

    let provider = provider.ok_or_else(|| WorkspaceError::NativeCompileProvider {
        owner: owner_id.to_string(),
        reason: "no compiler-native binding provider was supplied".to_owned(),
    })?;
    let supplied = provider.bind(owner)?;
    let (binding, policy) = supplied.into_parts();
    let (mode, deferred) = match mode {
        OwnerNativeCompileMode::Observed(observer) => {
            let (observer, deferred) = defer_emission(observer);
            (OwnerNativeCompileMode::Observed(observer), Some(deferred))
        }
        mode => (mode, None),
    };
    let scope = acquire(&mode);
    let compiled = compile_native(plan, source, &binding, policy, &mode, scope.as_ref())
        .map_err(|source| native_compile_error(&owner_id, source));
    let result = compiled.and_then(|outcome| {
        join_managed(
            outcome,
            &binding,
            &owner_id,
            &transforms,
            spec_format,
            deferred,
            finalizer,
        )
    });
    finish_scope(scope.as_ref(), &result);
    result.map(|outcome| {
        Some(OwnerManagedStaticCompile {
            artifact: OwnerManagedArtifact::Native(outcome),
            providers,
        })
    })
}

fn acquire(mode: &OwnerNativeCompileMode<'_>) -> Option<crate::compile_trace::TraceScope> {
    match mode {
        OwnerNativeCompileMode::Traced(acquisition) => acquisition.acquire(),
        OwnerNativeCompileMode::Plain | OwnerNativeCompileMode::Observed(_) => None,
    }
}

fn compile_builtin(
    plan: ArtifactPlan,
    source: &FsSectionSource,
    mode: OwnerNativeCompileMode<'_>,
    scope: Option<&crate::compile_trace::TraceScope>,
) -> Result<EmittedArtifact, WorkspaceError> {
    let result = match mode {
        OwnerNativeCompileMode::Plain => compile_artifact(plan, source),
        OwnerNativeCompileMode::Traced(_) => match scope {
            Some(scope) => compile_artifact_traced(plan, source, scope),
            None => compile_artifact(plan, source),
        },
        OwnerNativeCompileMode::Observed(observer) => {
            compile_artifact_observed(plan, source, observer)
        }
    };
    result.map_err(legacy_compile_error)
}

fn compile_native<B: crate::extension_world::CompilerNativeFactBinding>(
    plan: ArtifactPlan,
    source: &FsSectionSource,
    binding: &B,
    policy: vibe_spec::CompilerNativePolicy,
    mode: &OwnerNativeCompileMode<'_>,
    scope: Option<&crate::compile_trace::TraceScope>,
) -> Result<CompilerNativeOutcome, ArtifactCompileError> {
    match mode {
        OwnerNativeCompileMode::Plain => {
            compile_artifact_native_managed(plan, source, binding.invoker(), policy)
        }
        OwnerNativeCompileMode::Traced(_) => match scope {
            Some(scope) => compile_artifact_native_managed_traced(
                plan,
                source,
                binding.invoker(),
                policy,
                scope,
            ),
            None => compile_artifact_native_managed(plan, source, binding.invoker(), policy),
        },
        OwnerNativeCompileMode::Observed(observer) => compile_artifact_native_managed_observed(
            plan,
            source,
            binding.invoker(),
            policy,
            Arc::clone(observer),
        ),
    }
}

fn join_managed<B: crate::extension_world::CompilerNativeFactBinding>(
    outcome: CompilerNativeOutcome,
    binding: &B,
    owner: &OwnerRuntimeId,
    plan: &vibe_spec::TransformPlan,
    spec_format: SpecFormat,
    deferred: Option<vibe_spec::DeferredEmission>,
    finalizer: PendingFinalizer,
) -> Result<OwnerNativeCompileOutcome, WorkspaceError> {
    let outcome =
        match outcome {
            CompilerNativeOutcome::Ready(ready) => {
                let (artifact, receipts) = ready.into_parts();
                binding
                    .finish_ready()
                    .map_err(|source| fact_error(owner, source))?;
                OwnerNativeCompileOutcome {
                    artifact,
                    continuation: OwnerNativeCompileContinuation::Ready { receipts },
                }
            }
            CompilerNativeOutcome::Pending(pending) => {
                let facts = binding
                    .take_pending_build_facts(pending.pending())
                    .map_err(|source| fact_error(owner, source))?;
                let evidence = build_pending_artifact_evidence(
                    pending.pending(),
                    owner.clone(),
                    PendingArtifactTarget::BootStatic,
                    spec_format,
                    facts,
                )
                .map_err(|source| WorkspaceError::NativePendingEvidence {
                    owner: owner.to_string(),
                    source,
                })?
                .ok_or_else(|| WorkspaceError::NativeCompileProvider {
                    owner: owner.to_string(),
                    reason: "a Pending compiler outcome produced empty evidence".to_owned(),
                })?;
                let finalized = finalizer(pending, plan, evidence.fingerprint().as_bytes())
                    .map_err(|source| WorkspaceError::NativePendingFinalize {
                        owner: owner.to_string(),
                        source,
                    })?;
                let (artifact, pending) = finalized.into_parts();
                OwnerNativeCompileOutcome {
                    artifact,
                    continuation: OwnerNativeCompileContinuation::Pending { evidence, pending },
                }
            }
        };
    if let Some(deferred) = deferred {
        deferred.deliver(outcome.artifact());
    }
    Ok(outcome)
}

fn fact_error(
    owner: &OwnerRuntimeId,
    source: crate::extension_world::CompilerNativeFactError,
) -> WorkspaceError {
    WorkspaceError::NativeCompileFacts {
        owner: owner.to_string(),
        source,
    }
}

fn native_compile_error(owner: &OwnerRuntimeId, source: ArtifactCompileError) -> WorkspaceError {
    WorkspaceError::NativeCompile {
        owner: owner.to_string(),
        source: Box::new(source),
    }
}

fn legacy_compile_error(error: ArtifactCompileError) -> WorkspaceError {
    let reason = match error {
        ArtifactCompileError::Input { input, source }
            if input.kind == vibe_spec::ArtifactInputType::Normal =>
        {
            format!(
                "compiling the normal package `{}` closure (PROP-035 §8): {source}",
                input.origin
            )
        }
        error => error.to_string(),
    };
    WorkspaceError::InlineCompile { reason }
}

fn finish_scope(
    scope: Option<&crate::compile_trace::TraceScope>,
    result: &Result<OwnerNativeCompileOutcome, WorkspaceError>,
) {
    if let Some(scope) = scope {
        match result {
            Ok(outcome) => scope.complete_lossy(&outcome.artifact().output_fingerprint()),
            Err(error) => scope.fail_lossy(&error.to_string()),
        }
    }
}

fn finish_artifact_scope(
    scope: Option<&crate::compile_trace::TraceScope>,
    result: &Result<EmittedArtifact, WorkspaceError>,
) {
    if let Some(scope) = scope {
        match result {
            Ok(artifact) => scope.complete_lossy(&artifact.output_fingerprint()),
            Err(error) => scope.fail_lossy(&error.to_string()),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn write_boot_artifacts_owner_managed<P: OwnerNativeCompileProvider>(
    node_dir: &Path,
    node_rel: &str,
    workspace_root: &Path,
    self_coord: &SelfCoordinate,
    boot: &EffectiveBoot,
    spec_format: SpecFormat,
    trace: Option<&TraceRun>,
    owner: OwnerRuntimeView<'_>,
    provider: Option<&mut P>,
) -> Result<(WrittenArtifacts, Option<OwnerNativeCompileContinuation>), WorkspaceError> {
    write_boot_artifacts_owner_managed_using(
        node_dir,
        node_rel,
        workspace_root,
        self_coord,
        boot,
        spec_format,
        trace,
        owner,
        provider,
        finalize_compiler_pending_artifact,
    )
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn write_boot_artifacts_owner_managed_with_finalizer<P: OwnerNativeCompileProvider>(
    node_dir: &Path,
    node_rel: &str,
    workspace_root: &Path,
    self_coord: &SelfCoordinate,
    boot: &EffectiveBoot,
    spec_format: SpecFormat,
    trace: Option<&TraceRun>,
    owner: OwnerRuntimeView<'_>,
    provider: Option<&mut P>,
    finalizer: PendingFinalizer,
) -> Result<(WrittenArtifacts, Option<OwnerNativeCompileContinuation>), WorkspaceError> {
    write_boot_artifacts_owner_managed_using(
        node_dir,
        node_rel,
        workspace_root,
        self_coord,
        boot,
        spec_format,
        trace,
        owner,
        provider,
        finalizer,
    )
}

#[allow(clippy::too_many_arguments)]
fn write_boot_artifacts_owner_managed_using<P: OwnerNativeCompileProvider>(
    node_dir: &Path,
    node_rel: &str,
    workspace_root: &Path,
    self_coord: &SelfCoordinate,
    boot: &EffectiveBoot,
    spec_format: SpecFormat,
    trace: Option<&TraceRun>,
    owner: OwnerRuntimeView<'_>,
    provider: Option<&mut P>,
    finalizer: PendingFinalizer,
) -> Result<(WrittenArtifacts, Option<OwnerNativeCompileContinuation>), WorkspaceError> {
    let has_static = boot.static_entries().next().is_some();
    let acquisition = trace
        .filter(|_| has_static)
        .map(|run| ScopeAcquisition::node(run, node_rel, spec_format));
    let mode = match acquisition.as_ref() {
        Some(acquisition) => OwnerNativeCompileMode::Traced(acquisition),
        None => OwnerNativeCompileMode::Plain,
    };
    let index_text = render_index_with_spec_format(boot, None, spec_format)?;
    let compiled = compile_static_owner_managed_using(
        boot,
        workspace_root,
        self_coord,
        spec_format,
        owner,
        mode,
        provider,
        None,
        None,
        finalizer,
    )?;
    let boot_dir = node_dir.join(layout::current_boot_dir());
    let index = boot_dir.join(INDEX_FILE);
    let static_path = boot_dir.join(static_file(spec_format));
    let stale_path = boot_dir.join(stale_static_file(spec_format));
    let static_bytes = compiled
        .as_ref()
        .map(|compiled| compiled.artifact().bytes());
    let redirects = transaction::write_production_with_selectors(
        transaction::ArtifactWrite {
            index_path: &index,
            index_bytes: index_text.as_bytes(),
            static_path: &static_path,
            static_bytes,
            stale_path: &stale_path,
        },
        |transaction| {
            redirect::write_redirect_blocks_with_transaction(node_dir, spec_format, transaction)
        },
    )?;
    let static_lane = compiled.as_ref().map(|_| static_path);
    let native = compiled
        .map(OwnerManagedStaticCompile::into_parts)
        .and_then(|(_, native, _)| native);
    Ok((
        WrittenArtifacts {
            index,
            static_lane,
            redirects,
        },
        native,
    ))
}
