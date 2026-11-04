//! `update -g` (`PROP-059#commands`, `PROP-065#update-and-rollback`): the
//! newest resolved version as a new installation that takes the slot
//! default and the public commands, the replaced one kept hidden until the
//! next successful update of the slot, or refreshed in place.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#update-and-rollback");

use vibe_core::manifest::result_of;
pub(super) mod retirement;

use crate::error::AppError;
use crate::index::{Index, Lifecycle, Location};
use crate::resolve::ResolvedPackage;
use crate::select::InstallChoice;
use crate::steps::Previous;
use crate::storage::Storage;

use super::apply::{self, Mode, Selected};
use super::{
    Command, InstallRequest, OperationContext, Outcome, admit, choose, commit, deps,
    ensure_available, plan_option, previous_installation, remove,
};

/// Update one installed app package to the newest resolved version. An
/// owned update keeps the replaced installation hidden until the next
/// successful update of the slot (`PROP-065#update-and-rollback`).
pub fn update(
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    request: &InstallRequest,
) -> Result<Outcome, AppError> {
    let held = ctx.settings.lock()?;
    let lock = &held;
    if let Some(outcome) = super::pending::manual(lock, ctx, resolved, request)? {
        return Ok(outcome);
    }
    let mut stack = Vec::new();
    deps::enter(&mut stack, &resolved.package.key())?;
    update_locked(lock, ctx, resolved, request, &mut stack, None)
}

pub(super) fn update_locked(
    lock: &crate::MutationLock,
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    request: &InstallRequest,
    stack: &mut Vec<String>,
    saved: Option<&crate::steps::StepsFile>,
) -> Result<Outcome, AppError> {
    let index = Index::load(ctx.settings)?;
    let storage = Storage::load(ctx.settings)?;
    let operation = saved.and_then(|s| s.operation.as_ref());
    let published = operation
        .map(|op| super::pending::was_published(ctx.settings, &index, op))
        .transpose()?
        .unwrap_or(false);
    let prior_key = match operation {
        Some(op) => op.predecessor.clone().ok_or_else(|| {
            super::pending::failure(
                ctx.settings,
                &resolved.package,
                "update has no retained predecessor".into(),
            )
        })?,
        None => previous_installation(ctx.settings, &index, resolved)?,
    };
    let original_prior = index.installations.get(&prior_key).cloned();
    let prior = if published {
        index.installations.get(&resolved.package.key()).cloned()
    } else {
        original_prior.clone()
    }
    .ok_or_else(|| AppError::not_installed(ctx.settings, &prior_key))?;
    if prior.slot != resolved.application.id {
        return Err(AppError::SlotChanged {
            key: prior_key,
            from: prior.slot,
            to: resolved.application.id.clone(),
        });
    }
    ensure_available(&prior_key, &prior, &storage)?;
    let explicit = request.choice != InstallChoice::default();
    let choice = if explicit {
        request.choice.clone()
    } else {
        InstallChoice {
            platform: Some(prior.platform.clone()),
            arch: Some(prior.arch.clone()),
            installer: Some(prior.option.clone()),
        }
    };
    let option = choose(ctx, resolved, &choice)?;
    let key = resolved.package.key();
    admit(
        ctx,
        &index,
        &[key.as_str(), prior_key.as_str()],
        resolved,
        &option,
    )?;
    let owned = result_of(&option).is_owned();
    let same_key = index.installations.get(&key).cloned();
    if let Some(existing) = &same_key {
        ensure_available(&key, existing, &storage)?;
    }
    let (target_prior, library, replaces) = if owned {
        let library = request.library.clone().or(match &prior.location {
            Location::Owned { library, .. } => Some(library.clone()),
            Location::External { .. } => None,
        });
        (same_key, library, None)
    } else if matches!(prior.location, Location::Owned { .. }) {
        (same_key, request.library.clone(), None)
    } else {
        (
            Some(prior.clone()),
            request.library.clone(),
            Some(prior_key.clone()),
        )
    };
    if operation.is_some_and(|op| {
        owned
            && prior_key != key
            && replaces.is_none()
            && request.purge_previous
            && !op.retirements.iter().any(|r| r.key == prior_key)
    }) {
        return Err(super::pending::failure(
            ctx.settings,
            &resolved.package,
            "requested previous-version retirement has no bound obligation".into(),
        ));
    }
    let previous = if let Some(op) = operation {
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
            secret_origins: &prior.secret_origins,
            configuration: prior.configuration.as_ref(),
            features: &prior.features,
            parameters: &prior.parameters,
            license: prior.license.as_ref().map(|l| l.name.as_str()),
            ask: false,
        }
    };
    let previous = if saved.is_none() && option.configuration.is_some() && option.id != prior.option
    {
        None
    } else {
        Some(previous)
    };
    let plan = plan_option(ctx, resolved, &option, request, previous, &index, operation)?;
    let retirements = if saved.is_none() && owned && prior_key != key && replaces.is_none() {
        retirement::bind(ctx, &index, &prior, &key, request.purge_previous)?
    } else {
        Vec::new()
    };
    apply::admit_agents(ctx, resolved, &plan, request, saved)?;
    let target = if let Some(state) = saved {
        super::pending::validate_plan(ctx, resolved, &option, request, &plan, state)?;
        operation
            .ok_or_else(|| {
                super::pending::failure(
                    ctx.settings,
                    &resolved.package,
                    "missing retained update".into(),
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
            request,
            &plan,
            super::pending::Checkpoint {
                kind: vibe_wire::generated::shared::ApplicationPendingOperationKind::Update,
                retirements,
                predecessor: Some(&prior_key),
                parent: None,
            },
        )?
    };
    super::pending::bind_resupply(ctx.settings, &target, &plan.secret_provided)?;
    super::pending::keep_work(ctx.settings, &target, request.keep_work)?;
    super::pending::before_dependencies(ctx.settings, &target)?;
    let paths = deps::install_required(
        lock,
        ctx,
        &plan,
        request.accept_license,
        stack,
        Some(&target),
    )?;
    let moved = prior_key != key;
    let mut applied = apply::apply(
        ctx,
        &storage,
        &Selected {
            operation: Some(&target),
            resolved,
            option: &option,
            plan: &plan,
            prior: target_prior.as_ref(),
            library: library.as_deref(),
            force: request.force,
            mode: Mode::Update,
            start: &request.start,
            command: Command::Update,
            replaces: moved.then(|| prior_key.clone()),
            keep_work: request.keep_work,
            paths,
        },
    )?;
    applied.installation.lifecycle = Lifecycle {
        hidden: prior.lifecycle.hidden,
        mark: prior.lifecycle.mark,
        requires: deps::recorded(&plan),
        env: std::mem::take(&mut applied.installation.lifecycle.env),
        held: prior.lifecycle.held,
        replaced: moved.then(|| prior_key.clone()),
        superseded_by: None,
    };
    let finish_failure = applied.finish_failure.take();
    let resumable_failure = request.keep_work
        || matches!(
            &finish_failure,
            Some(AppError::Agent(crate::AgentError::Parked(_)))
        );
    let has_retirements =
        super::pending::load(ctx.settings, &resolved.package, Some(&target.operation))?
            .operation
            .as_ref()
            .is_some_and(|o| !o.retirements.is_empty());
    applied.installation.pending = ((finish_failure.is_some() && resumable_failure)
        || (finish_failure.is_none() && has_retirements))
        .then(|| target.clone());
    super::pending::publishing(ctx.settings, &target, &applied.installation)?;
    // An owned update of another key keeps the previous installation,
    // hidden; earlier kept versions of the slot go now.
    let supersede = (owned && moved && replaces.is_none()).then(|| prior_key.clone());
    let mut index = Index::load(ctx.settings)?;
    let outcome = commit(
        ctx,
        &storage,
        &mut index,
        applied,
        if published { None } else { replaces },
        if published { None } else { supersede.clone() },
        super::Publication {
            lock,
            command: Command::Update,
            preserve_default: published,
        },
    )?;
    super::pending::phase(
        ctx.settings,
        &target,
        vibe_wire::generated::shared::ApplicationPendingOperationPhase::Published,
    )?;
    // The user's collections, pins, overrides and counts follow the
    // launchers to the new key (`PROP-059#launcher-runtime`).
    if moved && let Some(record) = index.installations.get(&key) {
        if let Some(op) = operation {
            crate::catalog::carry_over_ids(
                ctx.settings,
                &prior_key,
                &op.predecessor_launchers,
                record,
            )?;
        } else {
            crate::catalog::carry_over(ctx.settings, &prior, record)?;
        }
    }
    let mut message = if moved {
        format!("updated {prior_key} to {key}")
    } else {
        format!("refreshed {key} in place")
    };
    if let Some(failure) = finish_failure {
        // Setup did not succeed. Keep previous versions; queued retirement never
        // starts merely because a replacement record was published.
        if !resumable_failure
            && let Err(cleanup) = super::pending::complete_failed(lock, ctx.settings, &target)
            && let Some(notice) = ctx.on_notice
        {
            notice(&format!("failed update work remains: {cleanup}"));
        }
        return Err(failure);
    }
    if has_retirements {
        retirement::run(lock, ctx, &target)?;
        retirement::clear_pending(lock, ctx, &target)?;
    }
    if let Some(kept) = supersede.as_ref() {
        if request.purge_previous {
            message.push_str(&format!("; removed the previous {kept}"));
        } else {
            message.push_str(&format!("; the previous {kept} is kept hidden until the next successful update (`vibe app rollback` restores it)"));
        }
    }
    super::pending::complete(lock, ctx.settings, &target)?;
    Ok(Outcome { message, ..outcome })
}
