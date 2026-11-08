//! Standalone activation in the existing adjacent receipt, never a second inventory.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION");
use super::*;
use crate::activation::{ActivationIntent, desired_enabled};

/// Desired intent and disk observation; absence does not assert client runtime state.
#[derive(Debug, Clone, Serialize)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
pub struct SkillActivationStatus {
    pub provider: String,
    pub entry: String,
    pub agent: String,
    pub scope: &'static str,
    pub desired_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activation: Option<ActivationIntent>,
    pub observed: &'static str,
}

fn paths(
    agent: Agent,
    scope: Scope,
    project: Option<&Path>,
    name: &str,
) -> anyhow::Result<(PathBuf, PathBuf)> {
    anyhow::ensure!(
        vibe_core::manifest::SkillDecl::valid_name(name),
        "invalid skill name"
    );
    anyhow::ensure!(scope != Scope::Both, "expand activation scopes first");
    let root = agent
        .skills_root(scope, project)?
        .ok_or_else(|| anyhow::anyhow!("agent has no skill loader for this scope"))?;
    receipt::ensure_no_follow_walk(
        project.filter(|_| scope == Scope::Project).unwrap_or(&root),
        &root,
        true,
    )?;
    Ok((root.join(name), standalone_receipt_path(&root, name)))
}

fn owns_provider(prior: &StandaloneSkillReceipt, provider: &str) -> anyhow::Result<()> {
    if prior.provider.as_deref().is_some_and(|p| p != provider) {
        anyhow::bail!("skill name is owned by a different package");
    }
    Ok(())
}

/// Inventory observation without writing or resolving the package again.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
pub fn managed_skill_status(
    agent: Agent,
    scope: Scope,
    project: Option<&Path>,
    provider: &str,
    name: &str,
) -> anyhow::Result<SkillActivationStatus> {
    let (target, path) = paths(agent, scope, project, name)?;
    let prior = read_standalone_receipt(&path, name)?;
    if let Some(prior) = &prior {
        owns_provider(prior, provider)?;
    }
    let current = snapshot_dir(&target)?;
    let intent = prior.as_ref().and_then(|r| r.activation.clone());
    let enabled = desired_enabled(intent.as_ref());
    let observed = match (&prior, &current) {
        (None, None) => "absent",
        (None, Some(_)) => "foreign",
        (Some(prior), Some(current)) => {
            let owned_present = prior.file.iter().any(|f| current.contains_key(&f.path));
            let drift = validate_standalone_uninstall(&target, Some(current), Some(prior)).is_err();
            if drift {
                "changed"
            } else if !enabled && !owned_present {
                "disabled"
            } else if !enabled {
                "pending"
            } else if prior.file.iter().all(|f| current.contains_key(&f.path)) {
                "enabled"
            } else {
                "missing"
            }
        }
        (Some(_), None) => {
            if enabled {
                "missing"
            } else {
                "disabled"
            }
        }
    };
    Ok(SkillActivationStatus {
        provider: provider.into(),
        entry: name.into(),
        agent: agent.as_str().into(),
        scope: scope.as_str(),
        desired_enabled: enabled,
        activation: intent,
        observed,
    })
}

/// Exact selection only; ownership is still admitted from its existing receipt.
#[derive(Debug, Clone, Copy)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
pub struct SkillActivationTarget<'a> {
    pub agent: Agent,
    pub scope: Scope,
    pub project: Option<&'a Path>,
    pub provider: &'a str,
    pub name: &'a str,
}

/// Retain durable intent before retracting owned bytes. Enable uses current owned package bytes.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
pub fn change_managed_skill_activation(
    selection: SkillActivationTarget<'_>,
    desired: &BTreeMap<String, Vec<u8>>,
    intent: &ActivationIntent,
    dry_run: bool,
) -> anyhow::Result<SkillActivationStatus> {
    let SkillActivationTarget {
        agent,
        scope,
        project,
        provider,
        name,
    } = selection;
    crate::activation::validate(intent)?;
    let (target, _) = paths(agent, scope, project, name)?;
    let (_, prior) = known_intent_receipt(agent, scope, project, provider, name, intent)?;
    if dry_run {
        let current = snapshot_dir(&target)?;
        if intent.enabled {
            validate_standalone_ownership(&target, current.as_ref(), Some(&prior), desired)?;
        } else {
            validate_standalone_uninstall(&target, current.as_ref(), Some(&prior))?;
        }
    } else {
        retain_managed_skill_activation(agent, scope, project, provider, name, intent)?;
        install_managed_skill_snapshot(agent, scope, project, Some(provider), name, desired, false)
            .map_err(|e| {
                anyhow::anyhow!("activation intent retained; projection reconciliation failed: {e}")
            })?;
    }
    managed_skill_status(agent, scope, project, provider, name)
}

/// Canonical owned snapshot reconciliation used by declared and builtin skill writers.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
pub fn install_managed_skill_snapshot(
    agent: Agent,
    scope: Scope,
    project: Option<&Path>,
    provider: Option<&str>,
    name: &str,
    desired: &BTreeMap<String, Vec<u8>>,
    dry_run: bool,
) -> Result<PackageSkillReport, PackageSkillError> {
    if let Some(project_root) = project.filter(|_| scope == Scope::Project) {
        let intent = receipt::project_intent_for_target(project_root, name, agent, provider)
            .map_err(|e| PackageSkillError::UnsafePath {
                path: project_root.into(),
                reason: e.to_string(),
            })?;
        if !desired_enabled(intent.as_ref()) {
            return Ok(PackageSkillReport {
                skill: name.into(),
                agent: agent.as_str().into(),
                scope: scope.as_str(),
                path: None,
                status: "activation-managed",
                note: Some(
                    "project receipt owns disabled intent; use its reconciliation owner".into(),
                ),
            });
        }
    }
    let root = agent
        .skills_root(scope, project)
        .map_err(|e| PackageSkillError::SkillsRoot {
            detail: e.to_string(),
        })?;
    let Some(root) = root else {
        return Ok(skipped(name, agent, scope.as_str()));
    };
    let target = root.join(name);
    receipt::ensure_no_follow_walk(
        project.filter(|_| scope == Scope::Project).unwrap_or(&root),
        &target,
        true,
    )
    .map_err(|e| PackageSkillError::UnsafePath {
        path: target.clone(),
        reason: e.to_string(),
    })?;
    let path = standalone_receipt_path(&root, name);
    let prior = read_standalone_receipt(&path, name)?;
    if let (Some(prior), Some(provider)) = (&prior, provider) {
        owns_provider(prior, provider).map_err(|e| PackageSkillError::UnsafePath {
            path: path.clone(),
            reason: e.to_string(),
        })?;
    }
    let current = snapshot_dir(&target)?;
    let enabled = desired_enabled(prior.as_ref().and_then(|r| r.activation.as_ref()));
    if provider.is_some()
        && prior.as_ref().is_some_and(|r| r.provider.is_none())
        && current.as_ref() != Some(desired)
    {
        return Err(PackageSkillError::UnsafePath {
            path: target.clone(),
            reason: "legacy skill provider is unknown; exact-source adoption required".into(),
        });
    }
    let absent = BTreeMap::new();
    validate_standalone_ownership(
        &target,
        current.as_ref(),
        prior.as_ref(),
        if enabled { desired } else { &absent },
    )?;
    let mut next = standalone_receipt(name, desired);
    next.provider = provider
        .map(str::to_owned)
        .or_else(|| prior.as_ref().and_then(|r| r.provider.clone()));
    next.activation = prior.as_ref().and_then(|r| r.activation.clone());
    if next.provider.is_some() || next.activation.is_some() {
        next.schema = 2;
    }
    let matched = owned_projection_matches(current.as_ref(), prior.as_ref(), desired);
    let status = if !enabled {
        if dry_run { "would-disable" } else { "disabled" }
    } else if matched {
        "unchanged"
    } else if current.is_none() && prior.is_none() {
        "created"
    } else {
        "updated"
    };
    if !dry_run {
        if enabled {
            if !matched {
                reconcile_standalone_files(&target, prior.as_ref(), desired)?;
            }
            if prior.as_ref() != Some(&next) {
                write_standalone_receipt(&path, &next)?;
            }
        } else {
            // Keep the old owned hashes through removal; a crash retries the same guard.
            remove_owned_standalone_files(&target, prior.as_ref())?;
            // Do not turn foreign neighbours into owned content or erase retained intent.
        }
    }
    Ok(PackageSkillReport {
        skill: name.into(),
        agent: agent.as_str().into(),
        scope: scope.as_str(),
        path: Some(machine_json_path(&target)),
        status: crate::preview_status(status, dry_run),
        note: None,
    })
}

/// Explicit registration can adopt identical canonical builtin content; observation cannot.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
pub fn register_builtin_skill_snapshot(
    agent: Agent,
    scope: Scope,
    project: Option<&Path>,
    desired: &BTreeMap<String, Vec<u8>>,
    dry_run: bool,
) -> anyhow::Result<PackageSkillReport> {
    adopt_builtin_skill_snapshot(
        agent,
        scope,
        project,
        desired,
        crate::activation::BuiltinAdoption::explicit("vibe mcp install"),
        dry_run,
    )
}

/// Explicit builtin registration with a stated adoption policy.
///
/// An unreceipted `vibevm` skill target is claimed only by an explicit
/// command: byte-identical canonical content is adopted as-is; stale
/// content is replaced only under [`BuiltinAdoption::force`], and only when
/// every file in the target is one the builtin skill itself ships (a target
/// carrying any other file is never claimed). The replaced `SKILL.md` digest
/// is reported. Files are written before the receipt so an interrupted
/// forced adoption leaves canonical unreceipted bytes that a plain explicit
/// registration adopts, never a receipt that claims bytes not on disk.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
pub fn adopt_builtin_skill_snapshot(
    agent: Agent,
    scope: Scope,
    project: Option<&Path>,
    desired: &BTreeMap<String, Vec<u8>>,
    adoption: crate::activation::BuiltinAdoption,
    dry_run: bool,
) -> anyhow::Result<PackageSkillReport> {
    let (target, path) = paths(agent, scope, project, "vibevm")?;
    let prior = read_standalone_receipt(&path, "vibevm")?;
    let current = snapshot_dir(&target)?;
    if let (None, Some(current)) = (&prior, &current) {
        let identical = current == desired;
        if !identical && !adoption.force {
            return Err(PackageSkillError::Unowned {
                path: target.clone(),
                reason: "unreceipted builtin skill differs from the canonical template; refusing to overwrite".into(),
            }
            .into());
        }
        if let Some(extra) = current.keys().find(|rel| !desired.contains_key(*rel)) {
            return Err(PackageSkillError::Unowned {
                path: target.join(extra),
                reason: "unreceipted builtin skill target holds a file the builtin skill never ships; refusing to adopt it even under --force".into(),
            }
            .into());
        }
        let replaced: Vec<String> = current
            .iter()
            .filter(|(rel, bytes)| desired.get(*rel) != Some(*bytes))
            .map(|(rel, bytes)| format!("{rel} sha256 {}", digest_bytes(bytes)))
            .collect();
        if !dry_run {
            if !identical {
                reconcile_standalone_files(&target, None, desired)?;
            }
            let mut receipt = standalone_receipt("vibevm", desired);
            receipt.schema = 2;
            receipt.provider = Some("org.vibevm.core/vibevm".into());
            receipt.activation = Some(adoption.intent(!identical));
            write_standalone_receipt(&path, &receipt)?;
            validate_standalone_ownership(
                &target,
                snapshot_dir(&target)?.as_ref(),
                Some(&receipt),
                desired,
            )?;
        }
        let note = if replaced.is_empty() {
            "new ownership by explicit registration; prior author unknown".to_string()
        } else {
            format!(
                "new ownership by forced registration; replaced unreceipted content written by an older vibe ({})",
                replaced.join(", ")
            )
        };
        return Ok(PackageSkillReport {
            skill: "vibevm".into(),
            agent: agent.as_str().into(),
            scope: scope.as_str(),
            path: Some(machine_json_path(&target)),
            status: if dry_run { "would-adopt" } else { "adopted" },
            note: Some(note),
        });
    }
    Ok(install_managed_skill_snapshot(
        agent,
        scope,
        project,
        Some("org.vibevm.core/vibevm"),
        "vibevm",
        desired,
        dry_run,
    )?)
}

#[cfg(test)]
#[path = "activation_tests.rs"]
mod tests;

#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
pub fn admit_managed_skill_activation(
    agent: Agent,
    scope: Scope,
    project: Option<&Path>,
    provider: &str,
    name: &str,
    intent: &ActivationIntent,
) -> anyhow::Result<()> {
    known_intent_receipt(agent, scope, project, provider, name, intent).map(|_| ())
}
fn known_intent_receipt(
    agent: Agent,
    scope: Scope,
    project: Option<&Path>,
    provider: &str,
    name: &str,
    intent: &ActivationIntent,
) -> anyhow::Result<(PathBuf, StandaloneSkillReceipt)> {
    crate::activation::validate(intent)?;
    let (_, path) = paths(agent, scope, project, name)?;
    let prior = read_standalone_receipt(&path, name)?
        .ok_or_else(|| anyhow::anyhow!("skill is not managed; explicitly register it first"))?;
    if prior.provider.as_deref() != Some(provider) {
        anyhow::bail!("skill provider is unknown or differs; explicit registration required");
    }
    Ok((path, prior))
}
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
pub fn retain_managed_skill_activation(
    agent: Agent,
    scope: Scope,
    project: Option<&Path>,
    provider: &str,
    name: &str,
    intent: &ActivationIntent,
) -> anyhow::Result<()> {
    let (path, mut prior) = known_intent_receipt(agent, scope, project, provider, name, intent)?;
    prior.schema = 2;
    prior.activation = Some(intent.clone());
    write_standalone_receipt(&path, &prior).map_err(|e| {
        anyhow::anyhow!("activation intent not confirmed retained; projection untouched: {e}")
    })
}
