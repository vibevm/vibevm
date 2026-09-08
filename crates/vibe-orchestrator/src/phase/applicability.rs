//! Pure host-OS projection for lifecycle package/deploy targets.

use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use vibe_core::manifest::{
    ArtifactPackageTarget, ArtifactsSection, DeployTarget, Manifest, TargetApplicability, TargetOs,
};

use crate::DeployAuthority;

pub(super) struct ProjectedTargets {
    pub(super) package_targets: Vec<ArtifactPackageTarget>,
    pub(super) deploy_targets: Vec<DeployTarget>,
    pub(super) deploy: Option<DeployAuthority>,
    pub(super) notices: Vec<String>,
}

pub(super) fn project(
    manifest: &Manifest,
    artifacts: Option<&ArtifactsSection>,
    deploy: Option<DeployAuthority>,
    os: TargetOs,
) -> Result<ProjectedTargets> {
    let package = artifacts
        .map(|section| section.project_package_targets(os))
        .transpose()
        .context("projecting [[artifacts.package]] host applicability")?;
    let package_targets = package.as_ref().map_or_else(Vec::new, |projection| {
        projection
            .active()
            .iter()
            .map(|target| (*target).clone())
            .collect()
    });
    let mut notices = package.as_ref().map_or_else(Vec::new, |projection| {
        skip_notices("[[artifacts.package]]", projection.decisions())
    });

    let Some(deploy) = deploy else {
        return Ok(ProjectedTargets {
            package_targets,
            deploy_targets: Vec::new(),
            deploy: None,
            notices,
        });
    };
    let section = manifest
        .deploy
        .as_ref()
        .context("resolved deploy selection has no [deploy] declaration")?;
    let projection = section
        .project_profile(&deploy.selection.profile, os)
        .context("projecting [[deploy.target]] host applicability")?;
    ensure!(
        deploy.selection.targets == projection.active_targets(),
        "resolved deploy selection differs from the injected host-OS projection"
    );
    notices.extend(skip_notices("[[deploy.target]]", projection.decisions()));
    let targets = section
        .targets
        .iter()
        .map(|target| (target.id.as_str(), target))
        .collect::<BTreeMap<_, _>>();
    let mut deploy_targets = Vec::with_capacity(projection.active_targets().len());
    for id in projection.active_targets() {
        let target = targets[id.as_str()];
        if let Some(package) = &package {
            package
                .require_active_artifact(&target.id, &target.artifact)
                .context("validating active deploy artifact applicability")?;
        }
        deploy_targets.push(target.clone());
    }
    Ok(ProjectedTargets {
        package_targets,
        deploy_targets,
        deploy: Some(deploy),
        notices,
    })
}

fn skip_notices(kind: &str, decisions: &[TargetApplicability]) -> Vec<String> {
    decisions
        .iter()
        .filter(|decision| !decision.is_active())
        .map(|decision| {
            format!(
                "skipped {kind} target `{}`: {}",
                decision.target(),
                decision.reason()
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use vibe_core::manifest::Manifest;
    use vibe_lifecycle::{ClientExecutable, ClientExecutables, DeploySelection, Phase};

    use super::*;
    use crate::PhaseOutcome;
    use crate::phase::tests::mechanism_wiring::run_deploying_on;
    use crate::phase::tests::validate_only_gate::manifested;

    const MATRIX: &str = "[project]\nname='demo'\nversion='0.1.0'\n\
        [[artifacts.build]]\nid='build'\nmechanism='build:cargo'\noutputs=[{id='tool',kind='executable'}]\n\
        [[artifacts.package]]\nid='win-package'\nmechanism='package:static-skill'\nwhen={os=['windows']}\noutputs=[{id='win-bundle',kind='file'}]\nconfig={source='win'}\n\
        [[artifacts.package]]\nid='posix-package'\nmechanism='package:static-skill'\nwhen={os=['linux','macos']}\noutputs=[{id='posix-bundle',kind='file'}]\nconfig={source='posix'}\n\
        [[deploy.target]]\nid='win-deploy'\nartifact='tool'\nmechanism='deploy:vibe-bin'\nwhen={os=['windows']}\n\
        [[deploy.target]]\nid='posix-deploy'\nartifact='tool'\nmechanism='deploy:vibe-bin'\nwhen={os=['linux','macos']}\n\
        [deploy]\ndefault_profile='local'\n[deploy.profiles.local]\ntargets=['win-deploy','posix-deploy']\n";

    fn authority(target: &str) -> DeployAuthority {
        let missing = |command: &str| ClientExecutable::Missing {
            command: command.to_owned(),
        };
        DeployAuthority {
            selection: DeploySelection {
                profile: "local".to_owned(),
                targets: vec![target.to_owned()],
            },
            user_home: PathBuf::from("/unreachable/b2-home"),
            clients: ClientExecutables {
                claude: missing("claude"),
                codex: missing("codex"),
                opencode: missing("opencode"),
            },
        }
    }

    #[test]
    fn one_default_profile_projects_windows_and_posix_in_authored_order() {
        let manifest = Manifest::parse_str(MATRIX).unwrap();
        for (os, package, deploy, skipped) in [
            (TargetOs::Windows, "win-package", "win-deploy", "posix"),
            (TargetOs::Linux, "posix-package", "posix-deploy", "win"),
        ] {
            let projected = project(
                &manifest,
                manifest.artifacts.as_ref(),
                Some(authority(deploy)),
                os,
            )
            .unwrap();
            assert_eq!(projected.package_targets[0].id, package);
            assert_eq!(projected.deploy_targets[0].id, deploy);
            assert_eq!(projected.notices.len(), 2);
            assert!(projected.notices[0].contains(&format!("{skipped}-package")));
            assert!(projected.notices[1].contains(&format!("{skipped}-deploy")));
        }
    }

    #[test]
    fn inactive_package_target_surfaces_skip_and_moves_zero_bytes() {
        let dir = manifested(
            "[project]\nname='demo'\nversion='0.1.0'\n\
             [[artifacts.package]]\nid='win'\nmechanism='package:static-skill'\n\
             when={os=['windows']}\noutputs=[{id='win-file',kind='file'}]\n\
             config={source='never-read'}\n",
        );
        let outcome = run_deploying_on(dir.path(), vec![Phase::Package], None, TargetOs::Linux);
        let PhaseOutcome::Completed(values) = outcome else {
            panic!("inactive package completes without dispatch: {outcome:?}")
        };
        assert!(
            values
                .notices
                .iter()
                .any(|notice| notice.contains("skipped") && notice.contains("win"))
        );
        assert!(!dir.path().join("target").exists());
        assert!(!dir.path().join(".vibe/state/artifacts").exists());
    }

    #[test]
    fn active_deploy_to_inactive_package_refuses_before_dispatch() {
        let text = "[project]\nname='demo'\nversion='0.1.0'\n\
            [[artifacts.package]]\nid='win-package'\nmechanism='package:static-skill'\n\
            when={os=['windows']}\noutputs=[{id='win-file',kind='executable'}]\n\
            config={source='never-read'}\n\
            [[deploy.target]]\nid='linux-deploy'\nartifact='win-file'\nmechanism='deploy:vibe-bin'\n\
            when={os=['linux']}\n\
            [deploy]\ndefault_profile='local'\n[deploy.profiles.local]\ntargets=['linux-deploy']\n";
        let dir = manifested(text);
        let outcome = run_deploying_on(
            dir.path(),
            vec![Phase::Package, Phase::Deploy],
            Some(authority("linux-deploy")),
            TargetOs::Linux,
        );
        let PhaseOutcome::Failed { original, .. } = outcome else {
            panic!("cross-OS artifact dependency refuses: {outcome:?}")
        };
        let error = format!("{original:#}");
        for expected in ["linux-deploy", "win-package", "win-file", "linux"] {
            assert!(error.contains(expected), "{error}");
        }
        assert!(!dir.path().join("target").exists());
        assert!(!dir.path().join(".vibe/state/artifacts").exists());
    }

    #[test]
    fn lifecycle_observes_target_os_once_and_lower_cells_never_probe() {
        let command = include_str!("../../../vibe-cli/src/commands/lifecycle.rs");
        assert_eq!(command.matches("TargetOs::current()").count(), 1);
        let mcp = include_str!("../../../vibe-mcp/src/tools/lifecycle_run.rs");
        assert_eq!(mcp.matches("TargetOs::current()").count(), 1);
        for lower in [
            include_str!("../phase.rs"),
            include_str!("../dispatch/mechanism.rs"),
        ] {
            assert!(!lower.contains("TargetOs::current()"));
        }
    }
}
