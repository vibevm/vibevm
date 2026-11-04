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
    pub configuration: Option<crate::configuration::ConfigurationSubmission>,
    /// `--installer ID`: another installer option of the same version.
    pub installer: Option<String>,
    /// `--profile`, `--feature`, `--no-feature`.
    pub features: FeatureRequest,
    /// `--set ID=VALUE`.
    pub parameters: Vec<(String, String)>,
    pub accept_license: bool,
    /// `--keep-work`: keep the work directory of a successful run, as
    /// `install` does (`PROP-065#work`).
    pub keep_work: bool,
}

impl ModifyRequest {
    fn is_empty(&self) -> bool {
        self.configuration.is_none()
            && self.installer.is_none()
            && self.features.is_empty()
            && self.parameters.is_empty()
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
    let held = ctx.settings.lock()?;
    let lock = &held;
    modify_locked(lock, ctx, resolved, request, None)
}

pub(super) fn modify_locked(
    lock: &crate::MutationLock,
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    request: &ModifyRequest,
    resumed: Option<(&crate::steps::StepsFile, &InstallRequest)>,
) -> Result<Outcome, AppError> {
    let saved = resumed.map(|(state, _)| state);
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
    let install_request = resumed
        .map(|(_, request)| request.clone())
        .unwrap_or_else(|| InstallRequest {
            configuration: request.configuration.clone(),
            choice,
            features: if option.configuration.is_some() {
                request.features.clone()
            } else {
                keep_recorded(&option, &recorded, &request.features)
            },
            parameters: request.parameters.clone(),
            accept_license: request.accept_license,
            keep_work: request.keep_work,
            ..InstallRequest::default()
        });
    let previous = if let Some(op) = saved.and_then(|s| s.operation.as_ref()) {
        Previous {
            secret_origins: &op.request.secret_origins,
            configuration: op.request.configuration.as_ref(),
            features: &op.request.features,
            parameters: &op.request.parameters,
            license: None,
            ask: false,
        }
    } else {
        Previous {
            secret_origins: &record.secret_origins,
            configuration: record.configuration.as_ref(),
            features: &recorded,
            parameters: &record.parameters,
            license: record.license.as_ref().map(|l| l.name.as_str()),
            ask: request.is_empty(),
        }
    };
    let published = saved
        .and_then(|s| s.operation.as_ref())
        .map(|op| super::pending::was_published(ctx.settings, &index, op))
        .transpose()?
        .unwrap_or(false);
    let needs_removal = saved.and_then(|s| s.operation.as_ref()).is_none_or(|op| {
        matches!(
            op.phase,
            vibe_wire::generated::shared::ApplicationPendingOperationPhase::Prepared
                | vibe_wire::generated::shared::ApplicationPendingOperationPhase::Dependencies
        )
    });
    let quiet = OperationContext {
        on_plan: None,
        ..*ctx
    };
    let mut plan = plan_option(
        &quiet,
        resolved,
        &option,
        &install_request,
        if saved.is_none() && option.configuration.is_some() && option.id != record.option {
            None
        } else {
            Some(previous)
        },
        &index,
        saved.and_then(|s| s.operation.as_ref()),
    )?;
    for item in tree::authored_removal(&record)
        .into_iter()
        .filter(|_| needs_removal)
    {
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
    apply::admit_agents(ctx, resolved, &plan, &install_request, saved)?;
    let target = if let Some(state) = saved {
        super::pending::validate_plan(ctx, resolved, &option, &install_request, &plan, state)?;
        state
            .operation
            .as_ref()
            .ok_or_else(|| {
                super::pending::failure(
                    ctx.settings,
                    &resolved.package,
                    "missing modification operation".into(),
                )
            })?
            .target
            .clone()
    } else {
        super::pending::begin(
            lock,
            ctx,
            resolved,
            &option,
            &install_request,
            &plan,
            super::pending::Checkpoint {
                kind: vibe_wire::generated::shared::ApplicationPendingOperationKind::Modify,
                retirements: Vec::new(),
                predecessor: Some(&key),
                parent: None,
            },
        )?
    };
    super::pending::bind_resupply(ctx.settings, &target, &plan.secret_provided)?;
    super::pending::keep_work(ctx.settings, &target, install_request.keep_work)?;
    if let Some(show) = ctx.on_plan {
        show(&plan);
    }
    let mut stack = Vec::new();
    deps::enter(&mut stack, &key)?;
    super::pending::before_dependencies(ctx.settings, &target)?;
    let paths = deps::install_required(
        lock,
        ctx,
        &plan,
        request.accept_license,
        &mut stack,
        Some(&target),
    )?;
    if needs_removal {
        super::pending::phase(
            ctx.settings,
            &target,
            vibe_wire::generated::shared::ApplicationPendingOperationPhase::Removing,
        )?;
    }
    let root = match &record.location {
        Location::Owned { .. } => launch::app_root(&record, &storage).ok(),
        Location::External { app_root } => Some(app_root.clone()),
    };
    if let Some(root) = root
        .as_deref()
        .filter(|root| needs_removal && root.exists())
    {
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
    if needs_removal {
        super::pending::phase(
            ctx.settings,
            &target,
            vibe_wire::generated::shared::ApplicationPendingOperationPhase::Removed,
        )?;
    }
    let library = match &record.location {
        Location::Owned { library, .. } if option_owned(&plan.result) => Some(library.clone()),
        _ => None,
    };
    let mut applied = apply::apply(
        ctx,
        &storage,
        &Selected {
            operation: Some(&target),
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
    let resumable_failure = install_request.keep_work
        || matches!(
            &finish_failure,
            Some(AppError::Agent(crate::AgentError::Parked(_)))
        );
    applied.installation.pending =
        (finish_failure.is_some() && resumable_failure).then(|| target.clone());
    super::pending::publishing(ctx.settings, &target, &applied.installation)?;
    let mut index = Index::load(ctx.settings)?;
    let outcome = commit(
        ctx,
        &storage,
        &mut index,
        applied,
        None,
        None,
        super::Publication {
            lock,
            command: Command::Modify,
            preserve_default: published,
        },
    )?;
    super::pending::phase(
        ctx.settings,
        &target,
        vibe_wire::generated::shared::ApplicationPendingOperationPhase::Published,
    )?;
    if let Some(failure) = finish_failure {
        if !resumable_failure {
            super::pending::complete(lock, ctx.settings, &target)?;
        }
        return Err(failure);
    }
    super::pending::complete(lock, ctx.settings, &target)?;
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
