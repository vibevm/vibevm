use std::path::Path;

use vibe_spec::{
    ArtifactPlan, BackendId, CompilerNativeOutcome, EmittedArtifact, SelfCoordinate,
    compile_artifact_native_managed,
};

use crate::boot::EffectiveBoot;
use crate::extension_world::{
    CompilerNativeFactBinding, OwnerNativeCompileProvider, OwnerRuntimeView,
};
use crate::{WorkspaceError, layout_paths};

use super::super::declared_sources::{BootSectionSource, DeclaredBootSources};
use super::{fact_error, inputs, native_compile_error};

pub(crate) struct BackendReadContext<'a> {
    pub render_format: vibe_core::manifest::SpecFormat,
    pub sources: Option<&'a DeclaredBootSources>,
}

pub(crate) fn compile_backend_owner_managed<P: OwnerNativeCompileProvider>(
    boot: &EffectiveBoot,
    workspace_root: &Path,
    self_coord: &SelfCoordinate,
    backend: BackendId,
    reads: BackendReadContext<'_>,
    owner: OwnerRuntimeView<'_>,
    provider: &mut P,
) -> Result<Option<EmittedArtifact>, WorkspaceError> {
    let BackendReadContext {
        render_format,
        sources,
    } = reads;
    let entries = boot.static_entries().collect::<Vec<_>>();
    if entries.is_empty() {
        return Ok(None);
    }
    let lane = format!("artifacts/{}", backend.as_str());
    let source = BootSectionSource::new(
        workspace_root,
        self_coord,
        boot,
        render_format,
        sources,
        None,
    )?;
    let (inputs, _) = inputs::build_with_providers_bound(
        entries,
        workspace_root,
        self_coord,
        &lane,
        sources,
        None,
        &source,
    )?;
    let plans = owner.runtime().compile_plans().clone();
    let plan = plans.attach_to(
        ArtifactPlan::custom_backend(
            backend.clone(),
            lane.clone(),
            layout_paths::vibedeps(""),
            inputs,
        )
        .map_err(|error| WorkspaceError::InlineCompile {
            lane,
            reason: error.to_string(),
        })?,
    );
    let owner_id = owner.runtime().id().clone();
    let supplied = provider.bind(owner)?;
    let (binding, policy) = supplied.into_parts();
    let outcome = compile_artifact_native_managed(plan, &source, binding.invoker(), policy)
        .map_err(|source| native_compile_error(&owner_id, source))?;
    match outcome {
        CompilerNativeOutcome::Ready(ready) => {
            binding
                .finish_ready()
                .map_err(|source| fact_error(&owner_id, source))?;
            Ok(Some(ready.into_artifact()))
        }
        CompilerNativeOutcome::Pending(_) => Err(WorkspaceError::NativeCompileProvider {
            owner: owner_id.to_string(),
            reason: "write-free custom compilation cannot publish a Pending artifact".to_owned(),
        }),
    }
}
