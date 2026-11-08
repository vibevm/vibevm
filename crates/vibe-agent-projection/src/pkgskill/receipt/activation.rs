//! Project activation reuses the existing lock, stage, intent and CAS receipt transaction.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION");
use super::{
    nofollow::Project,
    stage::Stage,
    state::*,
    transaction::{Plan, finalize_receipt, publish_intent_and_execute},
};
use crate::{
    activation::{ActivationIntent, desired_enabled},
    agents::{Agent, Scope},
    pkgskill::{PackageSkillReport, ProjectSkillBinding, SkillActivationStatus},
};
use anyhow::{Result, bail};
use std::path::Path;

pub(crate) fn effective_binding(
    root: &Path,
    mut binding: ProjectSkillBinding,
) -> Result<ProjectSkillBinding> {
    let project = Project::open(root)?;
    if let Some(receipt) = read_receipt(&project)?
        && let Some(owned) = receipt
            .applying
            .as_ref()
            .map_or(&receipt.binding, |a| &a.binding)
            .iter()
            .find(|b| b.key == binding.identity())
    {
        binding.activation = owned.activation.clone();
    }
    binding
        .targets
        .retain(|target| desired_enabled(binding.activation.get(target.agent.as_str())));
    Ok(binding)
}

pub(crate) fn project_status(
    root: &Path,
    binding: &ProjectSkillBinding,
    agent: Agent,
) -> Result<SkillActivationStatus> {
    let project = Project::open(root)?;
    let receipt = read_receipt(&project)?.unwrap_or_else(empty_receipt);
    let row = receipt
        .binding
        .iter()
        .find(|row| row.key == binding.identity());
    let desired_row = receipt
        .applying
        .as_ref()
        .map_or(&receipt.binding, |a| &a.binding)
        .iter()
        .find(|row| row.key == binding.identity());
    let intent = desired_row
        .and_then(|row| row.activation.get(agent.as_str()))
        .cloned();
    let enabled = desired_enabled(intent.as_ref());
    let owned = row.and_then(|row| row.target.iter().find(|t| t.agent == agent.as_str()));
    let target = agent
        .skills_root(Scope::Project, Some(root))?
        .map(|p| p.join(&binding.skill.decl.name));
    let exists = target.as_ref().is_some_and(|p| p.join("SKILL.md").exists());
    let observed = if receipt.applying.is_some() {
        "pending"
    } else if let Some(target) = owned {
        if owned_target_matches(&project, root, target)? {
            if enabled { "enabled" } else { "pending" }
        } else {
            "changed"
        }
    } else if exists {
        "foreign"
    } else if !enabled {
        "disabled"
    } else {
        "absent"
    };
    Ok(SkillActivationStatus {
        provider: binding.skill.provider.identity(),
        entry: binding.skill.decl.name.clone(),
        agent: agent.as_str().into(),
        scope: "project",
        desired_enabled: enabled,
        activation: intent,
        observed,
    })
}

#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]

pub(crate) fn project_activation(
    root: &Path,
    binding: &ProjectSkillBinding,
    agent: Agent,
    intent: &ActivationIntent,
    dry_run: bool,
) -> Result<SkillActivationStatus> {
    crate::activation::validate(intent)?;
    project_intent(
        root,
        &binding.skill.provider.identity(),
        &binding.skill.decl.name,
        agent,
        intent,
        !dry_run,
    )?;
    mutate_subset(root, binding, &[agent], Change::Set(intent), dry_run).map_err(|e| {
        if dry_run {
            e
        } else {
            e.context("activation intent retained; projection reconciliation failed")
        }
    })?;
    project_status(root, binding, agent)
}

/// Selected agents change; unselected agents retain exact receipt-owned bytes.
pub(crate) fn project_subset(
    root: &Path,
    binding: &ProjectSkillBinding,
    agents: &[Agent],
    dry_run: bool,
) -> Result<Vec<PackageSkillReport>> {
    mutate_subset(root, binding, agents, Change::Refresh, dry_run)
}

pub(crate) fn project_remove(
    root: &Path,
    binding: &ProjectSkillBinding,
    agents: &[Agent],
    dry_run: bool,
) -> Result<Vec<PackageSkillReport>> {
    mutate_subset(root, binding, agents, Change::Remove, dry_run)
}

#[derive(Clone, Copy)]
enum Change<'a> {
    Refresh,
    Set(&'a ActivationIntent),
    Remove,
}

fn mutate_subset(
    root: &Path,
    binding: &ProjectSkillBinding,
    agents: &[Agent],
    change: Change<'_>,
    dry_run: bool,
) -> Result<Vec<PackageSkillReport>> {
    let request = match change {
        Change::Set(intent) => Some(intent),
        _ => None,
    };
    let project = Project::open(root)?;
    let _lock = if dry_run {
        None
    } else {
        Some(project.lock(super::nofollow::LOCK_FILE)?)
    };
    let mut before = read_receipt(&project)?.unwrap_or_else(empty_receipt);
    if before.applying.is_some() {
        bail!("recover pending package-skill transaction before activation");
    }
    let key = binding.identity();
    let selected = |agent: &str| agents.iter().any(|a| a.as_str() == agent);
    let prior = before.binding.iter().find(|r| r.key == key).cloned();
    let files = binding
        .selected_files
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("skill source is missing"))?;
    let mut raw = binding.clone();
    // Reconstitute only the explicitly selected target, including previously off targets.
    for &agent in agents {
        if !raw.targets.iter().any(|t| t.agent == agent) {
            let path = agent
                .skills_root(Scope::Project, Some(root))?
                .ok_or_else(|| anyhow::anyhow!("agent has no project skill loader"))?
                .join(&binding.skill.decl.name);
            raw.targets
                .push(crate::pkgskill::ProjectSkillTarget { agent, path });
        }
    }
    let mut after_row = prior
        .clone()
        .unwrap_or_else(|| receipt_binding(&raw, files));
    // Migrate standalone ownership only after exact file verification; unowned targets refuse.
    if prior.is_none() {
        after_row.target.clear();
        for target in &raw.targets {
            if !selected(target.agent.as_str()) {
                continue;
            }
            let parent = target
                .path
                .parent()
                .ok_or_else(|| anyhow::anyhow!("skill target lacks parent"))?;
            let path = crate::pkgskill::standalone_receipt_path(parent, &binding.skill.decl.name);
            if let Some(old) =
                crate::pkgskill::read_standalone_receipt(&path, &binding.skill.decl.name)?
            {
                if old
                    .provider
                    .as_deref()
                    .is_some_and(|p| p != binding.skill.provider.identity())
                {
                    bail!("skill provider ownership conflicts");
                }
                let current = crate::pkgskill::snapshot_dir(&target.path)?;
                crate::pkgskill::validate_standalone_uninstall(
                    &target.path,
                    current.as_ref(),
                    Some(&old),
                )?;
                if old.provider.is_none() && current.as_ref() != Some(files) {
                    bail!("legacy skill provider unknown; exact-source adoption required");
                }
                if let Some(intent) = old.activation {
                    after_row
                        .activation
                        .insert(target.agent.as_str().into(), intent);
                }
                let expected = receipt_binding(&raw, files);
                let mut owned = expected
                    .target
                    .into_iter()
                    .find(|t| t.agent == target.agent.as_str())
                    .ok_or_else(|| anyhow::anyhow!("missing selected target"))?;
                owned.file = old
                    .file
                    .into_iter()
                    .map(
                        |f| vibe_wire::generated::package_skill::e2::receipt::PackageSkillFile {
                            path: f.path,
                            sha256: format!("sha256:{}", f.sha256),
                        },
                    )
                    .collect();
                after_row.target.push(owned);
            }
        }
        if request.is_some() && after_row.target.is_empty() {
            bail!("skill is not managed; install it before activation");
        }
        if !after_row.target.is_empty() {
            before.binding.push(after_row.clone());
        }
    }
    if request.is_some()
        && !agents.iter().all(|a| {
            after_row.activation.contains_key(a.as_str())
                || after_row.target.iter().any(|t| t.agent == a.as_str())
        })
    {
        bail!("selected skill agent is not managed");
    }
    if let Some(intent) = request {
        for agent in agents {
            after_row
                .activation
                .insert(agent.as_str().into(), intent.clone());
        }
    }
    let wanted = receipt_binding(&raw, files);
    after_row.target.retain(|t| !selected(&t.agent));
    if matches!(change, Change::Remove) {
        after_row.activation.retain(|agent, _| !selected(agent));
    } else {
        after_row.target.extend(
            wanted.target.into_iter().filter(|t| {
                selected(&t.agent) && desired_enabled(after_row.activation.get(&t.agent))
            }),
        );
    }
    after_row.source_snapshot = binding.source_snapshot.clone();
    let mut after = before.binding.clone();
    after.retain(|r| r.key != key);
    after.push(after_row);
    // Preflight all owned removals/kept bytes and stage every after digest before intent.
    let mut staged = files.clone();
    for row in before.binding.iter().filter(|r| r.key == key) {
        for target in &row.target {
            if !owned_target_matches(&project, root, target)? {
                bail!("owned skill drift; refusing activation");
            }
            let chain = relative_components(root, Path::new(&target.path))
                .ok_or_else(|| anyhow::anyhow!("uncontained target"))?;
            let directory = project.dir(&chain, false)?;
            for file in &target.file {
                if let Some(bytes) = project.read_file_in(&directory, &file.path)? {
                    staged.insert(format!("{}/{}", target.agent, file.path), bytes);
                }
            }
        }
    }
    for &agent in agents {
        let target = raw
            .targets
            .iter()
            .find(|t| t.agent == agent)
            .ok_or_else(|| anyhow::anyhow!("missing selected target"))?;
        let current = crate::pkgskill::snapshot_dir(&target.path)?;
        let owned = before
            .binding
            .iter()
            .find(|r| r.key == key)
            .and_then(|r| r.target.iter().find(|t| t.agent == agent.as_str()));
        if current.as_ref().is_some_and(|files| !files.is_empty()) && owned.is_none() {
            bail!("refusing unowned project skill target");
        }
        if let (Some(current), Some(owned)) = (&current, owned) {
            for name in files.keys() {
                if current.contains_key(name) && !owned.file.iter().any(|f| &f.path == name) {
                    bail!("refusing foreign file in project skill target");
                }
            }
        }
    }
    if dry_run {
        let old = before.binding.iter().find(|r| r.key == key);
        let new = after.iter().find(|r| r.key == key);
        return Ok(agents
            .iter()
            .map(|agent| {
                let prior = old.and_then(|r| r.target.iter().find(|t| t.agent == agent.as_str()));
                let wanted = new.and_then(|r| r.target.iter().find(|t| t.agent == agent.as_str()));
                let disabled = new
                    .and_then(|r| r.activation.get(agent.as_str()))
                    .is_some_and(|i| !i.enabled);
                let status = if prior == wanted {
                    if disabled { "disabled" } else { "unchanged" }
                } else if wanted.is_none() {
                    if disabled {
                        "would-disable"
                    } else {
                        "would-remove"
                    }
                } else if prior.is_none() {
                    "would-create"
                } else {
                    "would-update"
                };
                PackageSkillReport {
                    skill: binding.skill.decl.name.clone(),
                    agent: agent.as_str().into(),
                    scope: "project",
                    path: raw
                        .targets
                        .iter()
                        .find(|t| t.agent == *agent)
                        .map(|t| vibe_core::machine_json_path(&t.path)),
                    status,
                    note: None,
                }
            })
            .collect());
    }
    // Persist migrated ownership before using it; native targets have not changed.
    if prior.is_none() && !before.binding.is_empty() {
        write_receipt(&project, &before)?;
    }
    let stage = Stage::create(&project, &staged)?;
    let plan = Plan {
        key,
        nonce: stage.nonce.clone(),
        before: before.binding,
        after,
    };
    let reports = publish_intent_and_execute(&project, root, Some(&stage), &plan)?;
    finalize_receipt(&project, root, &plan)?;
    stage.cleanup(&project)?;
    for target in &raw.targets {
        if selected(target.agent.as_str())
            && let Some(parent) = target.path.parent()
        {
            let path = crate::pkgskill::standalone_receipt_path(parent, &binding.skill.decl.name);
            crate::pkgskill::remove_file_if_present(&path)?;
        }
    }
    Ok(reports)
}

/// Compatibility writers may not bypass a project-owned disabled target.
pub(crate) fn project_intent_for_target(
    root: &Path,
    name: &str,
    agent: Agent,
    provider: Option<&str>,
) -> Result<Option<ActivationIntent>> {
    let project = Project::open(root)?;
    let Some(receipt) = read_receipt(&project)? else {
        return Ok(None);
    };
    let rows = receipt
        .applying
        .as_ref()
        .map_or(&receipt.binding, |a| &a.binding);
    let row = rows.iter().find(|r| {
        r.skill == name
            && (r.activation.contains_key(agent.as_str())
                || r.target.iter().any(|t| t.agent == agent.as_str()))
    });
    if let Some(row) = row {
        if provider.is_some_and(|p| p != row.provider) {
            bail!("skill target belongs to another project package");
        }
        Ok(row.activation.get(agent.as_str()).cloned())
    } else {
        Ok(None)
    }
}

#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-073#MANAGED-ACTIVATION")]
pub(crate) fn project_intent(
    root: &Path,
    provider: &str,
    skill: &str,
    agent: Agent,
    intent: &ActivationIntent,
    retain: bool,
) -> Result<()> {
    crate::activation::validate(intent)?;
    let project = Project::open(root)?;
    // Admission is read-only. A real write rechecks the same owner under its lock.
    let initial = read_receipt(&project)?.ok_or_else(|| {
        anyhow::anyhow!("project skill is not managed; explicitly install it first")
    })?;
    let key = format!(
        "{}{provider}/{skill}",
        crate::pkgskill::PROJECT_SKILL_PREFIX
    );
    validate_intent_owner(&initial, &key, agent)?;
    if !retain {
        return Ok(());
    }
    let _lock = project.lock(super::nofollow::LOCK_FILE)?;
    let mut receipt =
        read_receipt(&project)?.ok_or_else(|| anyhow::anyhow!("project skill receipt vanished"))?;
    let index = validate_intent_owner(&receipt, &key, agent)?;
    receipt.binding[index]
        .activation
        .insert(agent.as_str().into(), intent.clone());
    write_receipt(&project, &receipt).map_err(|e| {
        anyhow::anyhow!("activation intent not confirmed retained; projection untouched: {e}")
    })
}
fn validate_intent_owner(
    receipt: &vibe_wire::generated::package_skill::e2::receipt::PackageSkillReceipt,
    key: &str,
    agent: Agent,
) -> Result<usize> {
    if receipt.applying.is_some() {
        bail!("recover active project skill transaction before activation");
    }
    let (index, row) = receipt
        .binding
        .iter()
        .enumerate()
        .find(|(_, r)| r.key == key)
        .ok_or_else(|| {
            anyhow::anyhow!("project skill owner is unknown; explicitly install it first")
        })?;
    if !row.activation.contains_key(agent.as_str())
        && !row.target.iter().any(|t| t.agent == agent.as_str())
    {
        bail!("selected project skill agent is not managed");
    }
    Ok(index)
}
