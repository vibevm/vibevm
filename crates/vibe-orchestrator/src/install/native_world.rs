//! Existing ordinary native-world regeneration, separated from install execution.
specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");
use super::*;

/// The run-scoped facts one native-world regeneration reads: the trace, the
/// invocation's identity, the mutation lease and the cancellable progress.
pub(super) struct NativeRunScope<'a> {
    pub(super) trace: Option<&'a TraceRun>,
    pub(super) metadata: &'a RunMetadata,
    pub(super) lease: &'a LifecycleLease,
    pub(super) progress: &'a vibe_core::progress::Progress,
}

pub(super) fn regenerate_native_world(
    project_root: &std::path::Path,
    workspace: &vibe_workspace::Workspace,
    resolution: &[vibe_workspace::install::ResolvedDep],
    spec_format: vibe_core::manifest::SpecFormat,
    scope: NativeRunScope<'_>,
) -> Result<(Vec<String>, outcome::NativeInstallContext)> {
    let NativeRunScope {
        trace,
        metadata,
        lease,
        progress,
    } = scope;
    progress.checkpoint()?;
    vibe_install::configuration_admission::resolved_packages(resolution)?;
    let (world, lowering, sidecar) =
        crate::world::prepare_owner_runtime_inputs(project_root, workspace, resolution)?;
    let platform = vibe_lifecycle::native::NativePlatform::current()?;
    let run = vibe_workspace::extension_world::OwnerRuntimeRunFacts {
        run_id: metadata.run_id.clone(),
        state_root: lease.root().join(".vibe"),
        platform: platform.key().to_owned(),
        offline: metadata.offline,
        created_at: metadata.started.clone(),
    };
    let mut make_provider = |policies| {
        Ok(
            vibe_lifecycle::native::ArtifactCompilerNativeProvider::new(platform, policies)
                .with_cancellation(progress.cancellation().clone()),
        )
    };
    let (nodes, carriage) = vibe_workspace::install::regenerate_boot_from_traced_native(
        workspace,
        resolution,
        world,
        spec_format,
        trace,
        lowering,
        run,
        &mut make_provider,
    )?;

    Ok((nodes, outcome::NativeInstallContext::new(carriage, sidecar)))
}
