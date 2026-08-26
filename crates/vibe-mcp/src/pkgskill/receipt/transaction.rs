//! The durable roll-forward plan executor: compare-and-swap rules applied
//! through retained capabilities, never a destructive rollback.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use vibe_wire::generated::package_skill_receipt::{
    PackageSkillApplying, PackageSkillBinding as ReceiptBinding,
    PackageSkillTarget as ReceiptTarget,
};

use super::nofollow::{Pinned, Project};
use super::stage::Stage;
use super::state::{digest, read_receipt, relative_components, write_receipt};
use crate::pkgskill::PackageSkillReport;

/// One transaction's committed before-state and complete desired after-state.
#[derive(Debug)]
pub(super) struct Plan {
    pub key: String,
    pub nonce: String,
    pub before: Vec<ReceiptBinding>,
    pub after: Vec<ReceiptBinding>,
}

impl Plan {
    /// The exact applying record to publish for this plan.
    pub(super) fn applying(&self) -> PackageSkillApplying {
        PackageSkillApplying {
            binding: self.after.clone(),
            key: self.key.clone(),
            nonce: self.nonce.clone(),
        }
    }

    /// The keys this transaction owns: every row whose committed before and
    /// desired after differ (added, removed, or changed), plus the
    /// transaction's own key — an ordinary update stays executable even when
    /// its receipt row is byte-identical, so its own binding can still heal a
    /// tampered target. Unchanged rows are never executed, verified,
    /// reported, or blocked by an unrelated transaction.
    pub(super) fn changed_keys(&self) -> BTreeSet<String> {
        let before = index(&self.before);
        let after = index(&self.after);
        let mut keys = BTreeSet::from([self.key.clone()]);
        keys.extend(
            before
                .keys()
                .filter(|key| after.get(*key) != Some(&before[*key]))
                .cloned(),
        );
        keys.extend(
            after
                .keys()
                .filter(|key| !before.contains_key(*key))
                .cloned(),
        );
        keys
    }

    /// The after-file digests a durable stage must hold for this plan: the
    /// desired digests of the changed rows only. A removal-only plan that
    /// retains other bindings requires no stage; an ordinary update or new
    /// binding still requires all of its desired digests.
    pub(super) fn required_stage_digests(&self) -> BTreeSet<String> {
        let after = index(&self.after);
        self.changed_keys()
            .into_iter()
            .filter_map(|key| after.get(&key))
            .flat_map(|row| {
                row.target
                    .iter()
                    .flat_map(|target| target.file.iter().map(|file| file.sha256.clone()))
            })
            .collect()
    }
}

/// Publish the intent receipt, then execute the plan's CAS rules. On any
/// error the durable intent and stage stay for retry — already-published
/// files are never rolled back or removed.
pub(super) fn publish_intent_and_execute(
    project: &Project,
    project_root: &Path,
    stage: Option<&Stage>,
    plan: &Plan,
) -> Result<Vec<PackageSkillReport>> {
    let mut intent = empty_with(&plan.before);
    intent.applying = Some(plan.applying());
    write_receipt(project, &intent)?;
    execute_plan(project, project_root, stage, plan)
}

/// Execute one plan's desired-vs-owned rules without publishing a new intent
/// (recovery of an already-durable one). Only the plan's changed keys are
/// executed; unchanged rows never heal, report, or block.
pub(super) fn execute_plan(
    project: &Project,
    project_root: &Path,
    stage: Option<&Stage>,
    plan: &Plan,
) -> Result<Vec<PackageSkillReport>> {
    let before = index(&plan.before);
    let after = index(&plan.after);
    let mut reports = Vec::new();
    for key in plan.changed_keys() {
        let owned = before.get(&key);
        let desired = after.get(&key);
        reports.extend(execute_binding(
            project,
            project_root,
            stage,
            &key,
            owned,
            desired,
        )?);
    }
    Ok(reports)
}

fn execute_binding(
    project: &Project,
    project_root: &Path,
    stage: Option<&Stage>,
    key: &str,
    owned: Option<&ReceiptBinding>,
    desired: Option<&ReceiptBinding>,
) -> Result<Vec<PackageSkillReport>> {
    let mut reports = Vec::new();
    match (owned, desired) {
        (Some(before), Some(after)) => {
            for target in &after.target {
                let prior = before
                    .target
                    .iter()
                    .find(|candidate| candidate.agent == target.agent);
                let status = publish_target_cas(project, project_root, stage, target, prior)?;
                reports.push(report(key, &target.agent, &target.path, status));
            }
            for target in &before.target {
                if !after
                    .target
                    .iter()
                    .any(|candidate| candidate.agent == target.agent)
                {
                    remove_target_owned(project, project_root, key, target, &mut reports)?;
                }
            }
        }
        (None, Some(after)) => {
            for target in &after.target {
                let status = publish_target_cas(project, project_root, stage, target, None)?;
                reports.push(report(key, &target.agent, &target.path, status));
            }
        }
        (Some(before), None) => {
            for target in &before.target {
                remove_target_owned(project, project_root, key, target, &mut reports)?;
            }
        }
        (None, None) => {}
    }
    Ok(reports)
}

/// Apply the CAS rule to every desired file of one target:
///
/// - current digest == desired digest: already published, keep it;
/// - the path was previously owned by the receipt **under exactly this
///   spelling**: replace — its bytes are ours to heal whatever they drifted
///   to;
/// - absent: create;
/// - any other present state on a never-owned path: refuse and preserve
///   the bytes. Exact staged desired bytes are the only proof that an
///   interrupted *new* file is ours; intent alone is never ownership.
///
/// Ownership is the exact canonical relative spelling, never a fold key: a
/// prior `SKILL.md` must not authorize overwriting a distinct `skill.md` on
/// a case-sensitive host. Fold identity stays a portability *collision
/// detector* — the plan-level refusal in [`super::rename`].
fn publish_target_cas(
    project: &Project,
    project_root: &Path,
    stage: Option<&Stage>,
    desired: &ReceiptTarget,
    prior: Option<&ReceiptTarget>,
) -> Result<&'static str> {
    let directory = target_directory(project, project_root, &desired.path, true)?;
    let owned_paths = prior
        .map(|prior| {
            prior
                .file
                .iter()
                .map(|file| file.path.as_str())
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let mut changed = false;
    for file in &desired.file {
        let current = project.read_file(&directory, &file.path)?;
        let current_digest = current.as_deref().map(digest);
        match (&current_digest, owned_paths.contains(file.path.as_str())) {
            (Some(current), _) if current == &file.sha256 => continue,
            (_, true) | (None, false) => {
                publish(project, stage, &directory, file)?;
                changed = true;
            }
            (Some(_), false) => bail!(
                "refusing unowned pre-existing file `{}` inside target `{}`; \
                 remove or reconcile it manually before rerunning",
                file.path,
                desired.path
            ),
        }
    }
    remove_dropped_owned_files(project, &directory, desired, prior)?;
    prune_empty_directories(project, project_root, desired, prior)?;
    Ok(match (prior, changed) {
        (None, false) => "unchanged",
        (None, true) => "created",
        (Some(_), false) => "unchanged",
        (Some(_), true) => "updated",
    })
}

/// Remove files the prior target owned and the desired state dropped. A
/// tampered current digest refuses — foreign bytes are preserved, never
/// silently deleted. "Kept" is the exact spelling: an owned `SKILL.md` that
/// the desired state dropped must not be left silently unowned because some
/// `skill.md` folds onto it.
fn remove_dropped_owned_files(
    project: &Project,
    directory: &Pinned,
    desired: &ReceiptTarget,
    prior: Option<&ReceiptTarget>,
) -> Result<()> {
    let Some(prior) = prior else {
        return Ok(());
    };
    let kept = desired
        .file
        .iter()
        .map(|file| file.path.as_str())
        .collect::<BTreeSet<_>>();
    for file in &prior.file {
        if kept.contains(file.path.as_str()) {
            continue;
        }
        match project.read_file(directory, &file.path)? {
            Some(bytes) if digest(&bytes) == file.sha256 => {
                project.remove_file(directory, &file.path)?;
            }
            Some(_) => bail!(
                "refusing to remove tampered owned file `{}` in target `{}`; \
                 restore or delete it manually before rerunning",
                file.path,
                prior.path
            ),
            None => {}
        }
    }
    Ok(())
}

fn publish(
    project: &Project,
    stage: Option<&Stage>,
    directory: &Pinned,
    file: &vibe_wire::generated::package_skill_receipt::PackageSkillFile,
) -> Result<()> {
    let Some(stage) = stage else {
        bail!(
            "missing durable stage for `{}`; rerun after restoring `.vibe/package-skills/staged`",
            file.path
        );
    };
    let bytes = stage.require(&file.sha256)?;
    project.write_atomic(directory, &file.path, bytes)
}

/// Remove files a prior target owned and the desired state dropped. A file
/// whose current digest no longer matches the owned digest refuses —
/// tampered bytes are preserved, never silently deleted.
fn remove_target_owned(
    project: &Project,
    project_root: &Path,
    key: &str,
    prior: &ReceiptTarget,
    reports: &mut Vec<PackageSkillReport>,
) -> Result<()> {
    {
        let components = relative_components(project_root, Path::new(&prior.path))
            .with_context(|| format!("target `{}` is not project-relative", prior.path))?;
        // A target that is genuinely absent is a no-op; link/reparse and
        // unreadable failures propagate rather than collapsing into absence.
        let Some(directory) = project.dir_if_present(&components)? else {
            return Ok(());
        };
        for file in &prior.file {
            match project.read_file(&directory, &file.path)? {
                Some(bytes) if digest(&bytes) == file.sha256 => {
                    project.remove_file(&directory, &file.path)?;
                }
                Some(_) => bail!(
                    "refusing to remove tampered owned file `{}` in target `{}`; \
                     restore or delete it manually before rerunning",
                    file.path,
                    prior.path
                ),
                None => {}
            }
        }
    }
    prune_empty_directories(project, project_root, prior, None)?;
    let mut components = relative_components(project_root, Path::new(&prior.path))
        .with_context(|| format!("target `{}` is not project-relative", prior.path))?;
    components.pop();
    let target_dir = project.dir(&components, false)?;
    project.remove_dir_if_empty(&target_dir, last_component(&prior.path))?;
    reports.push(report(
        key,
        &prior.agent,
        &prior.path,
        if prior.file.is_empty() {
            "unchanged"
        } else {
            "removed"
        },
    ));
    Ok(())
}

/// Prune directories that held only owned files and are now empty, deepest
/// first, through capability-relative opens.
fn prune_empty_directories(
    project: &Project,
    project_root: &Path,
    desired: &ReceiptTarget,
    prior: Option<&ReceiptTarget>,
) -> Result<()> {
    let mut dirs = BTreeSet::new();
    for file in &desired.file {
        collect_parents(&file.path, &mut dirs);
    }
    if let Some(prior) = prior {
        for file in &prior.file {
            collect_parents(&file.path, &mut dirs);
        }
    }
    let target = Path::new(&desired.path);
    let Some(mut base) = relative_components(project_root, target)
        .map(|parts| parts.into_iter().map(String::from).collect::<Vec<_>>())
    else {
        return Ok(());
    };
    let mut ordered = dirs
        .into_iter()
        .map(|dir| dir.split('/').map(String::from).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    ordered.sort_by_key(|components| std::cmp::Reverse(components.len()));
    for components in ordered {
        let Some((name, parents)) = components.split_last() else {
            continue;
        };
        base.extend(parents.iter().cloned());
        let chain = base.iter().map(String::as_str).collect::<Vec<_>>();
        let parent_dir = project.dir(&chain, false);
        base.truncate(base.len() - parents.len());
        if let Ok(parent_dir) = parent_dir {
            project.remove_dir_if_empty(&parent_dir, name.as_str())?;
        }
    }
    Ok(())
}

fn collect_parents(relative: &str, dirs: &mut BTreeSet<String>) {
    let mut current = match relative.rsplit_once('/') {
        Some((parent, _)) => parent.to_string(),
        None => return,
    };
    loop {
        dirs.insert(current.clone());
        match current.rsplit_once('/') {
            Some((parent, _)) => current = parent.to_string(),
            None => break,
        }
    }
}

fn target_directory(
    project: &Project,
    project_root: &Path,
    target_path: &str,
    create: bool,
) -> Result<Pinned> {
    let absolute = Path::new(target_path);
    let components = relative_components(project_root, absolute)
        .with_context(|| format!("target `{target_path}` is not project-relative"))?;
    project.dir(&components, create)
}

fn last_component(path: &str) -> &str {
    path.rsplit(['/']).next().unwrap_or(path)
}

fn report(key: &str, agent: &str, path: &str, status: &'static str) -> PackageSkillReport {
    PackageSkillReport {
        skill: key.rsplit('/').next().unwrap_or(key).to_string(),
        agent: agent.to_string(),
        scope: "project",
        path: Some(path.to_string()),
        status,
        note: None,
    }
}

fn empty_with(
    bindings: &[ReceiptBinding],
) -> vibe_wire::generated::package_skill_receipt::PackageSkillReceipt {
    vibe_wire::generated::package_skill_receipt::PackageSkillReceipt {
        applying: None,
        binding: bindings.to_vec(),
        schema: 2,
    }
}

fn index(bindings: &[ReceiptBinding]) -> BTreeMap<String, ReceiptBinding> {
    bindings
        .iter()
        .map(|binding| (binding.key.clone(), binding.clone()))
        .collect()
}

/// Re-read the visible receipt under the lock and replace it with the final
/// no-applying state only when it still carries this **exact** transaction:
/// same schema, same complete committed-before bindings, and the same
/// complete applying object (key, nonce, and after-plan). Before replacing
/// it, the visible target namespace is purely verified — every desired file
/// digest present, every removed owned file absent — with no mutation.
pub(super) fn finalize_receipt(project: &Project, project_root: &Path, plan: &Plan) -> Result<()> {
    let Some(current) = read_receipt(project)? else {
        bail!(
            "package-skill receipt vanished under transaction `{}`; \
             rerun to recover from the durable intent",
            plan.key
        );
    };
    let mut expected = empty_with(&plan.before);
    expected.applying = Some(plan.applying());
    super::state::canonicalize_receipt(&mut expected);
    if current != expected {
        bail!(
            "package-skill receipt changed under transaction `{}`; \
             rerun to recover from the durable intent",
            plan.key
        );
    }
    verify_visible(project, project_root, plan)
        .context("verifying published package-skill targets before finalising")?;
    write_receipt(project, &empty_with(&plan.after))
}

/// Pure verification of one plan against the visible target namespace,
/// scoped to the plan's changed keys: every desired file of a changed row
/// must exist with exactly its digest, and every file the plan removed from
/// a previously owned target must be absent. Unchanged rows are never
/// verified or healed here; nothing is mutated — a failure leaves the
/// durable intent for retry.
pub(super) fn verify_visible(project: &Project, project_root: &Path, plan: &Plan) -> Result<()> {
    let before = index(&plan.before);
    let after = index(&plan.after);
    for key in plan.changed_keys() {
        if let Some(desired) = after.get(&key) {
            let prior_targets = before.get(&key);
            for target in &desired.target {
                let components = relative_components(project_root, Path::new(&target.path))
                    .with_context(|| format!("target `{}` is not project-relative", target.path))?;
                let Some(directory) = project.dir_if_present(&components)? else {
                    bail!("target `{}` is absent after publication", target.path);
                };
                for file in &target.file {
                    match project.read_file(&directory, &file.path)? {
                        Some(bytes) if digest(&bytes) == file.sha256 => {}
                        Some(bytes) => bail!(
                            "target file `{}/{}` shows `{}` instead of `{}`",
                            target.path,
                            file.path,
                            digest(&bytes),
                            file.sha256
                        ),
                        None => bail!(
                            "target file `{}{}` is absent after publication",
                            target.path,
                            file.path
                        ),
                    }
                }
                if let Some(prior_targets) = prior_targets {
                    let prior = prior_targets
                        .target
                        .iter()
                        .find(|candidate| candidate.agent == target.agent);
                    if let Some(prior) = prior {
                        let kept = target
                            .file
                            .iter()
                            .map(|file| file.path.as_str())
                            .collect::<BTreeSet<_>>();
                        for file in &prior.file {
                            if kept.contains(file.path.as_str()) {
                                continue;
                            }
                            if project.read_file(&directory, &file.path)?.is_some() {
                                bail!(
                                    "removed owned file `{}/{}` is still present",
                                    target.path,
                                    file.path
                                );
                            }
                        }
                    }
                }
            }
        }
        // Entire targets/bindings absent from the after-plan were removed by
        // this transaction; reopen the visible namespace and prove their
        // owned files are still absent before ownership is dropped.
        if let Some(prior_binding) = before.get(&key) {
            let desired_binding = after.get(&key);
            for prior_target in &prior_binding.target {
                let retained = desired_binding.is_some_and(|binding| {
                    binding
                        .target
                        .iter()
                        .any(|target| target.agent == prior_target.agent)
                });
                if retained {
                    continue;
                }
                let components = relative_components(project_root, Path::new(&prior_target.path))
                    .with_context(|| {
                    format!("target `{}` is not project-relative", prior_target.path)
                })?;
                let Some(directory) = project.dir_if_present(&components)? else {
                    continue;
                };
                for file in &prior_target.file {
                    if project.read_file(&directory, &file.path)?.is_some() {
                        bail!(
                            "removed owned file `{}/{}` is still present",
                            prior_target.path,
                            file.path
                        );
                    }
                }
            }
        }
    }
    Ok(())
}
