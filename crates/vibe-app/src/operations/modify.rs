//! `vibe app modify` (`PROP-065#modify`): another choice for an installed
//! package of the same version — installer option, features, profile,
//! parameter values — planned against the record, the old choice's
//! recorded removal run, the new choice installed through the ordinary
//! engine, and the record rewritten in its slot with its default and its
//! public commands.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#modify");

use std::collections::BTreeSet;

use specmark::spec;
use vibe_core::manifest::InstallerOptionDecl;

use crate::error::{AppError, ResolveError};
use crate::index::{Index, Lifecycle, Location};
use crate::launch;
use crate::resolve::ResolvedPackage;
use crate::select::InstallChoice;
use crate::steps::{FeatureRequest, Previous, RunOutcome, RunRecord};
use crate::storage::Storage;

use super::apply::{self, Mode, Selected};
use super::tree::{self, Undo, script_of};
use super::{
    Command, InstallRequest, OperationContext, Outcome, admit, choose, commit, deps,
    ensure_available, plan_option, remove,
};

/// The choice flags of `vibe app modify`; anything not named keeps its
/// recorded value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#modify")]
pub struct ModifyRequest {
    /// `--installer ID`: another installer option of the same version.
    pub installer: Option<String>,
    /// `--profile`, `--feature`, `--no-feature`.
    pub features: FeatureRequest,
    /// `--set ID=VALUE`.
    pub parameters: Vec<(String, String)>,
    pub accept_license: bool,
}

impl ModifyRequest {
    fn is_empty(&self) -> bool {
        self.installer.is_none() && self.features.is_empty() && self.parameters.is_empty()
    }
}

/// Modify the installation of `resolved` (the same package version as
/// the record): plan the new choice, show what the old choice recorded as
/// removal and what the new plan runs, run the old removal (last step
/// first), install the new choice and rewrite the record in place.
pub fn modify(
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    request: &ModifyRequest,
) -> Result<Outcome, AppError> {
    let _lock = ctx.settings.lock()?;
    let index = Index::load(ctx.settings)?;
    let storage = Storage::load(ctx.settings)?;
    let key = resolved.package.key();
    let record = index
        .installations
        .get(&key)
        .cloned()
        .ok_or_else(|| AppError::not_installed(ctx.settings, &key))?;
    ensure_available(&key, &record, &storage)?;
    let choice = match &request.installer {
        Some(id) => {
            let named = resolved
                .options
                .iter()
                .find(|option| &option.id == id)
                .ok_or_else(|| ResolveError::NoOption {
                    detail: format!("installer `{id}`"),
                    choices: resolved
                        .options
                        .iter()
                        .map(|option| option.id.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                })?;
            InstallChoice {
                platform: Some(named.platform.clone()),
                arch: Some(named.arch.clone()),
                installer: Some(id.clone()),
            }
        }
        None => InstallChoice {
            platform: Some(record.platform.clone()),
            arch: Some(record.arch.clone()),
            installer: Some(record.option.clone()),
        },
    };
    let option = choose(ctx, resolved, &choice)?;
    admit(ctx, &index, &[key.as_str()], resolved, &option)?;
    let recorded = declared_only(&option, &record.features);
    let install_request = InstallRequest {
        choice,
        features: keep_recorded(&option, &recorded, &request.features),
        parameters: request.parameters.clone(),
        accept_license: request.accept_license,
        ..InstallRequest::default()
    };
    let previous = Previous {
        features: &recorded,
        parameters: &record.parameters,
        license: record.license.as_ref().map(|license| license.name.as_str()),
        ask: request.is_empty(),
    };
    let quiet = OperationContext {
        on_plan: None,
        ..*ctx
    };
    let mut plan = plan_option(
        &quiet,
        resolved,
        &option,
        &install_request,
        Some(previous),
        &index,
    )?;
    for item in tree::authored_removal(&record) {
        plan.warnings.push(match item {
            Undo::Script { step, commands } => format!(
                "modify first undoes `{step}` of the recorded choice: {}",
                script_of(commands, false).unwrap_or_default()
            ),
            Undo::Env { step, entries } => format!(
                "modify first rolls back {} environment entr{} of `{step}`",
                entries.len(),
                if entries.len() == 1 { "y" } else { "ies" }
            ),
        });
    }
    if let Some(show) = ctx.on_plan {
        show(&plan);
    }
    let mut stack = Vec::new();
    deps::enter(&mut stack, &key)?;
    let paths = deps::install_required(ctx, &plan, request.accept_license, &mut stack)?;
    let root = match &record.location {
        Location::Owned { .. } => launch::app_root(&record, &storage).ok(),
        Location::External { app_root } => Some(app_root.clone()),
    };
    if let Some(root) = root.as_deref().filter(|root| root.exists()) {
        let mut run = RunRecord::open(
            ctx.settings,
            &record.package,
            Command::Modify,
            &record.option,
            None,
        )?;
        let undo = tree::authored_removal(&record);
        let result = remove::undo_authored(ctx, &record, &undo, false, root, &mut run);
        run.close(if result.is_ok() {
            RunOutcome::Done
        } else {
            RunOutcome::Failed
        });
        result?;
    }
    let library = match &record.location {
        Location::Owned { library, .. } if option_owned(&plan.result) => Some(library.clone()),
        _ => None,
    };
    let mut applied = apply::apply(
        ctx,
        &storage,
        &Selected {
            resolved,
            option: &option,
            plan: &plan,
            prior: Some(&record),
            library: library.as_deref(),
            force: false,
            mode: Mode::Install,
            start: &install_request.start,
            command: Command::Modify,
            replaces: None,
            keep_work: install_request.keep_work,
            paths,
        },
    )?;
    applied.installation.lifecycle = Lifecycle {
        requires: deps::recorded(&plan),
        env: std::mem::take(&mut applied.installation.lifecycle.env),
        ..record.lifecycle.clone()
    };
    let finish_failure = applied.finish_failure.take();
    let mut index = Index::load(ctx.settings)?;
    let outcome = commit(
        ctx,
        &storage,
        &mut index,
        applied,
        None,
        None,
        Command::Modify,
    )?;
    if let Some(failure) = finish_failure {
        return Err(failure);
    }
    Ok(Outcome {
        message: format!(
            "modified {key}: installer `{}` → `{}`, features [{}] → [{}]",
            record.option,
            option.id,
            record.features.join(", "),
            plan.features.join(", ")
        ),
        ..outcome
    })
}

fn option_owned(result: &vibe_core::manifest::ResultDecl) -> bool {
    result.is_owned()
}

/// The recorded features the new option still declares.
fn declared_only(option: &InstallerOptionDecl, recorded: &[String]) -> Vec<String> {
    recorded
        .iter()
        .filter(|id| option.features.iter().any(|feature| &feature.id == *id))
        .cloned()
        .collect()
}

/// Feature flags on top of the recorded set: a named `--feature` /
/// `--no-feature` changes only that feature (and its exclusive group);
/// a profile replaces the set as on install.
fn keep_recorded(
    option: &InstallerOptionDecl,
    recorded: &[String],
    request: &FeatureRequest,
) -> FeatureRequest {
    if request.is_empty() || request.profile.is_some() {
        return request.clone();
    }
    let group_of = |id: &str| {
        option
            .features
            .iter()
            .find(|feature| feature.id == id)
            .and_then(|feature| feature.exclusive_group.clone())
    };
    let requested_groups: BTreeSet<String> = request
        .enable
        .iter()
        .filter_map(|id| group_of(id))
        .collect();
    let mut enable: Vec<String> = recorded
        .iter()
        .filter(|id| !request.disable.contains(id))
        .filter(|id| group_of(id).is_none_or(|group| !requested_groups.contains(&group)))
        .cloned()
        .collect();
    for id in &request.enable {
        if !enable.contains(id) {
            enable.push(id.clone());
        }
    }
    FeatureRequest {
        enable,
        disable: request.disable.clone(),
        profile: None,
    }
}
