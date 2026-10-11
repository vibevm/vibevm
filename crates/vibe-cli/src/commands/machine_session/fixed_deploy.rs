//! One project-local fixed Deploy grant composition for CLI and MCP.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-FIXED-DEPLOY-REATTACH");

use anyhow::{Context, Result, bail};
use std::{path::Path, sync::Arc};
use vibe_core::manifest::{DeployTarget, Manifest, TargetOs};
use vibe_lifecycle::{SubmissionScope, process::SelectedFileWitness};
use vibe_mcp::ServerContext;
use vibe_orchestrator::{
    machine_operation::fixed_deploy::FixedDeployRoute,
    machine_submission::{
        TrustedPolicySource,
        deploy_grant::{DeployGrantAttachment, DeployGrantService},
    },
};
use vibe_safefs::Project;

use crate::commands::deploy::{
    self,
    profile::{ProfileMode, resolve_profile},
};

#[derive(Clone, Copy)]
pub(crate) enum FixedDeployHostMode {
    Issue(u64),
    Reattach,
}

/// Both process surfaces reconstruct the same facts independently of wire
/// contents. This first mode is project-local and has no selected VVM
/// generation; a package or installed-generation host needs its own observer.
pub(crate) fn context(
    context: ServerContext,
    root: &Path,
    profile: &str,
    additional_profiles: &[String],
    mode: FixedDeployHostMode,
) -> Result<ServerContext> {
    let os = TargetOs::current()
        .ok_or_else(|| anyhow::anyhow!("fixed Deploy requires a supported target platform"))?;
    let manifest = Manifest::read(root.join(Manifest::FILENAME))
        .with_context(|| format!("reading authored Deploy manifest at {}", root.display()))?;
    if manifest.project.is_none() || manifest.package.is_some() {
        bail!("fixed Deploy project-local mode requires one authored [project] node");
    }
    if additional_profiles.len() > 8 {
        bail!("fixed Deploy accepts at most eight additional profiles");
    }
    let mut allowed_names = additional_profiles.to_vec();
    allowed_names.sort();
    if allowed_names
        .iter()
        .any(|name| name == profile || name.is_empty())
        || allowed_names.windows(2).any(|pair| pair[0] == pair[1])
    {
        bail!("fixed Deploy allowed profiles must be distinct from the primary profile");
    }
    let issued_allowed = matches!(mode, FixedDeployHostMode::Issue(_))
        .then(|| allowed_targets(&manifest, &allowed_names, os))
        .transpose()?;
    let resolution = resolve_profile(
        manifest.deploy.as_ref(),
        Some(profile),
        ProfileMode::Forward(os),
    );
    let target = match (mode, resolution) {
        (FixedDeployHostMode::Issue(_), Ok(Some(resolution))) => {
            let [target_id] = resolution.selection().targets.as_slice() else {
                bail!("fixed Deploy profile must select exactly one active authored target");
            };
            manifest
                .deploy
                .as_ref()
                .and_then(|deploy| deploy.targets.iter().find(|target| &target.id == target_id))
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("fixed Deploy selected target is absent"))?
        }
        (FixedDeployHostMode::Issue(_), Ok(None)) => {
            bail!("fixed Deploy requires an authored profile")
        }
        (FixedDeployHostMode::Issue(_), Err(error)) => return Err(error.into()),
        (FixedDeployHostMode::Reattach, result) => {
            let target_id = result
                .ok()
                .flatten()
                .and_then(|selection| selection.selection().targets.first().cloned());
            manifest
                .deploy
                .as_ref()
                .and_then(|deploy| {
                    target_id
                        .as_ref()
                        .and_then(|id| deploy.targets.iter().find(|target| &target.id == id))
                })
                .or_else(|| {
                    manifest
                        .deploy
                        .as_ref()
                        .and_then(|deploy| deploy.targets.first())
                })
                .cloned()
                .unwrap_or(DeployTarget {
                    id: "recovery-only-unbound-target".into(),
                    artifact: "recovery-only-unbound-artifact".into(),
                    mechanism: "deploy:vibe-bin".parse()?,
                    provider: None,
                    when: None,
                    depends_on: None,
                    config: None,
                })
        }
    };
    let project = Arc::new(Project::open(root)?);
    let token = project.root_correlation_token()?;
    let roots = deploy::state_roots()?;
    if matches!(mode, FixedDeployHostMode::Issue(_)) {
        std::fs::create_dir_all(&roots.settings).with_context(|| {
            format!("creating Deploy settings root {}", roots.settings.display())
        })?;
    }
    let settings = Arc::new(Project::open(&roots.settings)?);
    let (home_path, clients) = deploy::client_authority()?;
    let home = Arc::new(Project::open(&home_path)?);
    let loaded = vibe_orchestrator::inspect(root)?;
    let profile_set = SelectedFileWitness::from_bytes(
        format!(
            "vibevm/fixed-deploy-allowed-profiles/e1\0{profile}\0{}",
            allowed_names.join("\0")
        )
        .into_bytes(),
    )
    .sha256;
    let scope_target = if allowed_names.is_empty() {
        format!("physical-root:{token}:deploy-profile:{profile}")
    } else {
        format!("physical-root:{token}:deploy-profile-set:{profile_set}")
    };
    let scope = SubmissionScope {
        context: "vibe-cli-project-local/e1".into(),
        target: scope_target,
        principal: "cooperating-project-cli".into(),
    };
    let binding_nonce = SelectedFileWitness::from_bytes(if allowed_names.is_empty() {
        format!("vibevm/project-local-fixed-deploy/e1\0{token}\0{profile}").into_bytes()
    } else {
        format!("vibevm/project-local-fixed-deploy/profiles/e1\0{token}\0{profile_set}")
            .into_bytes()
    })
    .sha256;
    let service = DeployGrantService::from_trusted_host(
        project,
        settings,
        home,
        scope,
        profile.to_owned(),
        target,
        Arc::new(loaded.mechanisms),
        Arc::new(loaded.mechanism_routes),
        clients,
        deploy::identity(&manifest),
        None,
        "project-local-no-generation/e1".into(),
        binding_nonce,
        1,
        Arc::new(|| Ok(None)),
        TrustedPolicySource::from_host_identifier("vibe-cli/fixed-deploy/e1".into())?,
    )?;
    let attachment = match mode {
        FixedDeployHostMode::Issue(ttl) => DeployGrantAttachment::Active(service.issue(ttl)?),
        FixedDeployHostMode::Reattach => service.reattach()?,
    };
    let route = match attachment {
        DeployGrantAttachment::Active(host) => {
            let additional = match issued_allowed {
                Some(rows) => rows,
                None => allowed_targets(&manifest, &allowed_names, os)?,
            };
            FixedDeployRoute::new(Arc::new(host.with_allowed_profiles(additional)?))
        }
        DeployGrantAttachment::RecoveryOnly(host) => {
            FixedDeployRoute::recovery_only(Arc::new(host))
        }
    };
    Ok(context.with_fixed_deploy_route(Arc::new(route))?)
}

fn allowed_targets(
    manifest: &Manifest,
    names: &[String],
    os: TargetOs,
) -> Result<Vec<(String, DeployTarget)>> {
    if !names.is_empty()
        && manifest
            .deploy
            .as_ref()
            .is_none_or(|deploy| deploy.targets.len() != 1)
    {
        bail!("profiled fixed Deploy requires one authored target shared by allowed profiles");
    }
    names
        .iter()
        .map(|name| {
            let resolution = resolve_profile(
                manifest.deploy.as_ref(),
                Some(name),
                ProfileMode::Forward(os),
            )?
            .ok_or_else(|| anyhow::anyhow!("allowed Deploy profile is not authored"))?;
            let [target_id] = resolution.selection().targets.as_slice() else {
                bail!("allowed Deploy profile must select exactly one active authored target");
            };
            let target = manifest
                .deploy
                .as_ref()
                .and_then(|deploy| deploy.targets.iter().find(|target| &target.id == target_id))
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("allowed Deploy target is absent"))?;
            Ok((name.clone(), target))
        })
        .collect()
}
