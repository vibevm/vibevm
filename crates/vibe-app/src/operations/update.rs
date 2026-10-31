//! `update -g` (`PROP-059#commands`, `PROP-065#update-and-rollback`): the
//! newest resolved version as a new installation that takes the slot
//! default and the public commands, the replaced one kept hidden until the
//! next successful update of the slot, or refreshed in place.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#update-and-rollback");

use vibe_core::manifest::result_of;

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
    let _lock = ctx.settings.lock()?;
    let mut stack = Vec::new();
    deps::enter(&mut stack, &resolved.package.key())?;
    update_locked(ctx, resolved, request, &mut stack)
}

fn update_locked(
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    request: &InstallRequest,
    stack: &mut Vec<String>,
) -> Result<Outcome, AppError> {
    let index = Index::load(ctx.settings)?;
    let storage = Storage::load(ctx.settings)?;
    let prior_key = previous_installation(&index, resolved)?;
    let prior = index
        .installations
        .get(&prior_key)
        .cloned()
        .ok_or_else(|| AppError::NotInstalled {
            package: prior_key.clone(),
        })?;
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
    let previous = Previous {
        features: &prior.features,
        parameters: &prior.parameters,
        license: prior.license.as_ref().map(|license| license.name.as_str()),
        ask: false,
    };
    let plan = plan_option(ctx, resolved, &option, request, Some(previous), &index)?;
    let paths = deps::install_required(ctx, &plan, stack)?;
    let moved = prior_key != key;
    let mut applied = apply::apply(
        ctx,
        &storage,
        &Selected {
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
    // An owned update of another key keeps the previous installation,
    // hidden; earlier kept versions of the slot go now.
    let supersede = (owned && moved && replaces.is_none()).then(|| prior_key.clone());
    let mut index = Index::load(ctx.settings)?;
    let outcome = commit(
        ctx,
        &storage,
        &mut index,
        applied,
        replaces,
        supersede.clone(),
        Command::Update,
    )?;
    let mut message = if moved {
        format!("updated {prior_key} to {key}")
    } else {
        format!("refreshed {key} in place")
    };
    if let Some(kept) = &supersede {
        let earlier: Vec<String> = index
            .installations
            .iter()
            .filter(|(other, record)| {
                record.slot == prior.slot
                    && record.lifecycle.superseded_by.is_some()
                    && (*other != kept || request.purge_previous)
            })
            .map(|(other, _)| other.clone())
            .collect();
        for other in &earlier {
            remove::remove_locked(ctx, other, false)?;
        }
        if request.purge_previous {
            message.push_str(&format!("; removed the previous {kept}"));
        } else {
            message.push_str(&format!(
                "; the previous {kept} is kept hidden until the next update (`vibe app rollback` restores it)"
            ));
        }
    }
    if let Some(failure) = finish_failure {
        return Err(failure);
    }
    Ok(Outcome { message, ..outcome })
}
