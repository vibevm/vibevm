use std::path::Path;

use vibe_spec::{
    ArtifactPlan, BackendId, CompilerNativeOutcome, EmittedArtifact, FileResolver, FsSectionSource,
    SelfCoordinate, compile_artifact_native_managed,
};

use crate::boot::EffectiveBoot;
use crate::extension_world::{
    CompilerNativeFactBinding, OwnerNativeCompileProvider, OwnerRuntimeView,
};
use crate::{WorkspaceError, layout_paths};

use super::{fact_error, inputs, native_compile_error};

pub(crate) fn compile_backend_owner_managed<P: OwnerNativeCompileProvider>(
    boot: &EffectiveBoot,
    workspace_root: &Path,
    self_coord: &SelfCoordinate,
    backend: BackendId,
    owner: OwnerRuntimeView<'_>,
    provider: &mut P,
) -> Result<Option<EmittedArtifact>, WorkspaceError> {
    let entries = boot.static_entries().collect::<Vec<_>>();
    if entries.is_empty() {
        return Ok(None);
    }
    let (inputs, _) = inputs::build_with_providers(entries, workspace_root, self_coord)?;
    let plans = owner.runtime().compile_plans().clone();
    let plan = plans.attach_to(
        ArtifactPlan::custom_backend(
            backend.clone(),
            format!("artifacts/{}", backend.as_str()),
            layout_paths::vibedeps(""),
            inputs,
        )
        .map_err(|error| WorkspaceError::InlineCompile {
            reason: error.to_string(),
        })?,
    );
    let source = FsSectionSource::new(FileResolver::new(workspace_root, self_coord.clone()));
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
