//! Desired-vs-owned reconciliation for bindings and vanished rows.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, Result, bail};

use super::containment::paths_overlap;
use super::nofollow::Project;
use super::stage::Stage;
use super::state::{
    canonicalize_receipt, empty_receipt, read_receipt, receipt_binding, write_receipt,
};
use super::transaction::{Plan, finalize_receipt, publish_intent_and_execute};
use crate::pkgskill::{PROJECT_SKILL_RECONCILE_KEY, PackageSkillReport, ProjectSkillBinding};

/// Reconcile one binding's desired projection against committed ownership.
/// A pending applying transaction refuses here — the recovery preset owns
/// it and runs first.
pub(crate) fn reconcile_binding(
    project_root: &Path,
    binding: &ProjectSkillBinding,
) -> Result<Vec<PackageSkillReport>> {
    let project = Project::open(project_root)?;
    let _guard = project.lock()?;
    let Some(mut receipt) = read_receipt(&project)? else {
        return reconcile_with_receipt(&project, project_root, binding, empty_receipt());
    };
    if receipt.applying.is_some() {
        bail!(
            "a pending package-skill transaction must be recovered by the \
             recovery preset before any ordinary binding runs"
        );
    }
    canonicalize_receipt(&mut receipt);
    reconcile_with_receipt(&project, project_root, binding, receipt)
}

fn reconcile_with_receipt(
    project: &Project,
    project_root: &Path,
    binding: &ProjectSkillBinding,
    before: vibe_wire::generated::package_skill_receipt::PackageSkillReceipt,
) -> Result<Vec<PackageSkillReport>> {
    // Plan-time safety re-check: the authored source stays inside its
    // declaring provider's root, and every target stays inside the selected
    // project without overlapping the source.
    super::containment::ensure_no_follow_walk(
        binding.skill.provider.root(),
        &binding.skill.source,
        true,
    )
    .with_context(|| format!("unsafe source for `{}`", binding.skill.decl.name))?;

    let key = binding.identity();
    let expected = binding
        .selected_files
        .as_ref()
        .map(|files| receipt_binding(binding, files));
    let mut after = before.clone();
    after.binding.retain(|row| row.key != key);
    if let Some(expected) = &expected {
        after.binding.push(expected.clone());
    }
    canonicalize_receipt(&mut after);

    // An unsupported portable rename refuses here — before the stage, the
    // durable intent, and every write — so all visible bytes and the
    // previous receipt are preserved exactly as they were. Deliberately
    // unwrapped: surfaces render only an error's top-level message, and the
    // guard's own message is the actionable one.
    super::rename::ensure_no_portable_rename(&before.binding, &after.binding)?;

    if expected.is_none() {
        // Missing source with no prior ownership is a non-mutating no-op
        // **before** any adoption guard: a foreign `HUMAN.md` in the would-be
        // target is preserved untouched and no `SKILL.md` is created. Only
        // prior ownership turns a missing source into a removal plan.
        if !before.binding.iter().any(|row| row.key == key) {
            write_receipt(project, &after)?;
            return Ok(Vec::new());
        }
        let plan = Plan {
            key,
            nonce: super::state::fresh_nonce(),
            before: before.binding.clone(),
            after: after.binding.clone(),
        };
        let reports = publish_intent_and_execute(project, project_root, None, &plan)
            .with_context(|| format!("projecting package skill `{}`", binding.skill.decl.name))?;
        finalize_receipt(project, project_root, &plan)?;
        return Ok(reports);
    }

    let previously_owned = before
        .binding
        .iter()
        .find(|row| row.key == binding.identity())
        .map(|row| {
            row.target
                .iter()
                .map(|target| target.path.clone())
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    for target in &binding.targets {
        super::containment::ensure_lexically_contained(project_root, &target.path)
            .with_context(|| format!("unsafe target for `{}`", binding.skill.decl.name))?;
        if paths_overlap(&binding.skill.source, &target.path) {
            bail!(
                "package skill `{}` source `{}` overlaps target `{}`",
                binding.skill.decl.name,
                binding.skill.source.display(),
                target.path.display()
            );
        }
        // Adoption guard: a pre-existing target directory this receipt never
        // owned refuses wholesale — unowned bytes are never adopted. The
        // receipt writes the canonical spelling itself, so ownership compares
        // exact canonical target strings, not case-folded equivalents.
        if !previously_owned.contains(&vibe_core::machine_json_path(&target.path)) {
            refuse_unowned_target(project, project_root, &target.path.to_string_lossy())?;
        }
    }

    let stage = match binding.selected_files.as_ref() {
        Some(files) => Some(Stage::create(project, files)?),
        None => None,
    };
    let nonce = stage
        .as_ref()
        .map_or_else(super::state::fresh_nonce, |stage| stage.nonce.clone());
    let plan = Plan {
        key,
        nonce: nonce.clone(),
        before: before.binding.clone(),
        after: after.binding.clone(),
    };
    let reports = publish_intent_and_execute(project, project_root, stage.as_ref(), &plan)
        .with_context(|| format!("projecting package skill `{}`", binding.skill.decl.name));
    let reports = match reports {
        Ok(reports) => reports,
        Err(error) => {
            // If the intent never became durable, this validated stage is
            // garbage: clean it. Once the intent exists, both stay for the
            // recovery preset.
            let intent_live = match read_receipt(project) {
                Ok(Some(current)) => current
                    .applying
                    .as_ref()
                    .is_some_and(|applying| applying.nonce == nonce),
                _ => true,
            };
            if !intent_live && let Some(stage) = &stage {
                stage.cleanup(project)?;
            }
            return Err(error);
        }
    };
    finalize_receipt(project, project_root, &plan)?;
    if let Some(stage) = stage {
        stage.cleanup(project)?;
    }
    Ok(reports)
}

/// Refuse a pre-existing, non-empty target directory the receipt never
/// owned; an absent or empty one may be created into.
fn refuse_unowned_target(project: &Project, project_root: &Path, target_path: &str) -> Result<()> {
    let Some(components) = super::state::relative_components(project_root, Path::new(target_path))
    else {
        return Ok(());
    };
    let Some(directory) = project.dir_if_present(&components)? else {
        return Ok(());
    };
    let names = project.child_names(&directory)?;
    if names.is_empty() {
        return Ok(());
    }
    bail!(
        "refusing unowned pre-existing package skill target `{target_path}`; \
         remove or reconcile it manually before rerunning"
    );
}

/// Remove receipt-owned rows whose bindings vanished from the authenticated
/// desired set. Never deletes without receipt ownership; preserves
/// neighbours; refuses tampered owned files.
pub(crate) fn reconcile_vanished(
    project_root: &Path,
    desired: &BTreeSet<String>,
) -> Result<Vec<PackageSkillReport>> {
    let project = Project::open(project_root)?;
    let _guard = project.lock()?;
    let Some(mut receipt) = read_receipt(&project)? else {
        // A missing receipt proves no ownership and therefore authorises no
        // deletion. This is deliberately a successful, write-free no-op.
        return Ok(Vec::new());
    };
    if receipt.applying.is_some() {
        bail!(
            "a pending package-skill transaction must be recovered by the \
             recovery preset before reconciliation runs"
        );
    }
    canonicalize_receipt(&mut receipt);
    let stale = receipt
        .binding
        .iter()
        .filter(|row| !desired.contains(&row.key))
        .count();
    if stale == 0 {
        return Ok(Vec::new());
    }
    let mut after = receipt.clone();
    after.binding.retain(|row| desired.contains(&row.key));
    canonicalize_receipt(&mut after);
    let plan = Plan {
        key: PROJECT_SKILL_RECONCILE_KEY.to_string(),
        nonce: super::state::fresh_nonce(),
        before: receipt.binding.clone(),
        after: after.binding.clone(),
    };
    let reports = publish_intent_and_execute(&project, project_root, None, &plan)
        .context("reconciling vanished package-skill bindings")?;
    finalize_receipt(&project, project_root, &plan)?;
    Ok(reports
        .into_iter()
        .map(|mut report| {
            report.note = Some("binding vanished from the authenticated desired set".into());
            report
        })
        .collect())
}
