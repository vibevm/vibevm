//! Build-fence preparation of selected deploy-native mechanism artifacts.

use std::path::Path;

use anyhow::{Context, Result, ensure};
use vibe_core::manifest::{ExtensionHandler, MechanismRoutes};
use vibe_lifecycle::native::{
    NativeBuildExecution, NativeMechanismArtifactClaim, NativeMechanismPlan, NativePlatform,
    PreparedNativeMechanisms, build_native_sources, preflight_native_mechanisms,
    project_native_source_groups,
};
use vibe_lifecycle::{ExtensionRegistryRow, MechanismRegistry};

use crate::install::NativeInstallContext;

#[allow(clippy::too_many_arguments)]
pub(super) fn prepare(
    native: Option<&NativeInstallContext>,
    selected_candidates: &[ExtensionRegistryRow],
    plan: NativeMechanismPlan,
    project_root: &Path,
    registry: &MechanismRegistry,
    routes: &MechanismRoutes,
    platform: NativePlatform,
    offline: bool,
    created_at: &str,
) -> Result<PreparedNativeMechanisms> {
    let no_candidates = [];
    let mechanism_execution = NativeBuildExecution {
        candidates: &no_candidates,
        selected_project_root: project_root,
        registry,
        routes,
        platform,
        offline,
        created_at,
    };
    let mechanism = preflight_native_mechanisms(plan, &mechanism_execution)
        .context("preflighting selected native mechanism artifacts")?;
    match native {
        Some(native) => {
            let groups = super::mechanism::preflight_all_owner_native_sources(
                native,
                project_root,
                platform,
                offline,
                created_at,
            )?;
            preflight_cross_family(&groups, mechanism.claims())?;
            preflight_retained_prebuilts(native, mechanism.claims(), platform)?;
            for group in groups {
                build_native_sources(&NativeBuildExecution {
                    candidates: &group.candidates,
                    selected_project_root: project_root,
                    registry: group.registry,
                    routes: group.routes,
                    platform,
                    offline,
                    created_at,
                })
                .with_context(|| {
                    format!(
                        "building native source group {}/{}",
                        group.source.0, group.source.3
                    )
                })?;
            }
        }
        None => {
            let candidates = selected_candidates.iter().collect::<Vec<_>>();
            let execution = NativeBuildExecution {
                candidates: &candidates,
                selected_project_root: project_root,
                registry,
                routes,
                platform,
                offline,
                created_at,
            };
            let groups = project_native_source_groups(&candidates, platform)?;
            if !groups.is_empty() {
                platform.resolved_build_provider_pin(&execution)?;
                platform.admit_build_provider(&execution)?;
            }
            for group in &groups {
                ensure!(
                    !mechanism.claims().iter().any(|claim| matches!(claim,
                        NativeMechanismArtifactClaim::Source { provider, provider_root, crate_dir }
                        if provider == &group.provider
                            && provider_root == &group.provider_root
                            && crate_dir == &group.crate_dir)),
                    "one provider source group declares both extension and mechanism ABI families"
                );
            }
            preflight_candidate_prebuilts(&candidates, mechanism.claims(), platform)?;
            build_native_sources(&execution)
                .context("building enabled native source extensions at the build fence")?;
        }
    }
    mechanism
        .prepare(&mechanism_execution)
        .context("preparing selected native mechanism load images")
}

fn preflight_cross_family(
    groups: &[super::mechanism::PreflightBuildGroup<'_>],
    claims: &[NativeMechanismArtifactClaim],
) -> Result<()> {
    for group in groups {
        ensure!(
            !claims.iter().any(|claim| matches!(claim,
                NativeMechanismArtifactClaim::Source { provider, provider_root, crate_dir }
                if provider == &group.source.0
                    && provider_root == &group.source.1
                    && crate_dir == &group.source.3)),
            "one provider source group declares both extension and mechanism ABI families"
        );
    }
    Ok(())
}

fn preflight_retained_prebuilts(
    native: &NativeInstallContext,
    claims: &[NativeMechanismArtifactClaim],
    platform: NativePlatform,
) -> Result<()> {
    let (epoch, owners) = native.build_parts();
    for owner in owners {
        let view = match owner {
            vibe_workspace::extension_world::OwnerRuntimeId::Node { rel } => epoch.node(rel)?,
            vibe_workspace::extension_world::OwnerRuntimeId::Unit { provider } => {
                epoch.unit(provider)?
            }
        };
        let rows = view.runtime().rows()?;
        preflight_candidate_prebuilts(rows.native(), claims, platform)?;
    }
    Ok(())
}

fn preflight_candidate_prebuilts(
    candidates: &[&ExtensionRegistryRow],
    claims: &[NativeMechanismArtifactClaim],
    platform: NativePlatform,
) -> Result<()> {
    for row in candidates {
        let ExtensionHandler::Native {
            prebuilt: Some(paths),
            ..
        } = &row.declaration().handler
        else {
            continue;
        };
        let Some(path) = paths.get(platform.key()) else {
            continue;
        };
        let provider = row.provider().to_string();
        let relative = path.display().to_string().replace('\\', "/");
        ensure!(
            !claims.iter().any(|claim| matches!(claim,
                NativeMechanismArtifactClaim::Prebuilt { provider: other, relative: other_path }
                if other == &provider && other_path == &relative)),
            "one provider prebuilt declares both extension and mechanism ABI families"
        );
    }
    Ok(())
}
