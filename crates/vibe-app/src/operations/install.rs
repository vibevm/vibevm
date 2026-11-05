//! Selected installation, retained preflight, dependency descent and final publication.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-CONTINUATION");
use super::apply::{self, Mode, Selected};
use super::{
    Command, OperationContext, admit, choose, commit, deps, ensure_available, pending, retire_env,
};
use crate::steps::{Previous, StepsFile};
use crate::storage::Storage;
use crate::{AppError, Index, InstallRequest, Lifecycle, Mark, Outcome, ResolvedPackage};
use vibe_wire::generated::shared as w;

/// Install one app package into the current user's settings root: its
/// requirements first (`PROP-065#app-dependencies`), then the package.
pub fn install(
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    request: &InstallRequest,
) -> Result<Outcome, AppError> {
    let held = ctx.settings.lock()?;
    let lock = &held;
    if let Some(outcome) = pending::manual(lock, ctx, resolved, request)? {
        return Ok(outcome);
    }
    install_locked(lock, ctx, resolved, request, &mut Vec::new(), None, None)
}

/// [`install`] under the held lock; `stack` names the installations in
/// progress further up, so a requirement cycle is refused.
pub(super) fn install_locked(
    lock: &crate::MutationLock,
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    request: &InstallRequest,
    stack: &mut Vec<String>,
    saved: Option<&StepsFile>,
    parent: Option<pending::Parent<'_>>,
) -> Result<Outcome, AppError> {
    let key = resolved.package.key();
    deps::enter(stack, &key)?;
    let result = install_entered(lock, ctx, resolved, request, stack, saved, parent);
    stack.pop();
    result
}

fn install_entered(
    lock: &crate::MutationLock,
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    request: &InstallRequest,
    stack: &mut Vec<String>,
    saved: Option<&StepsFile>,
    parent: Option<pending::Parent<'_>>,
) -> Result<Outcome, AppError> {
    let index = Index::load(ctx.settings)?;
    let storage = Storage::load(ctx.settings)?;
    let option = choose(ctx, resolved, &request.choice)?;
    let key = resolved.package.key();
    admit(ctx, &index, &[key.as_str()], resolved, &option)?;
    let prior = index.installations.get(&key).cloned();
    if let Some(prior) = &prior {
        ensure_available(&key, prior, &storage)?;
    }
    let previous = if let Some(op) = saved.and_then(|s| s.operation.as_ref()) {
        Some(Previous {
            secret_origins: &op.request.secret_origins,
            configuration: op.request.configuration.as_ref(),
            features: &op.request.features,
            parameters: &op.request.parameters,
            license: None,
            ask: false,
        })
    } else {
        prior
            .as_ref()
            .filter(|p| p.option == option.id)
            .map(|p| Previous {
                secret_origins: &p.secret_origins,
                configuration: p.configuration.as_ref(),
                features: &p.features,
                parameters: &p.parameters,
                license: p.license.as_ref().map(|l| l.name.as_str()),
                ask: false,
            })
    };
    let import_parent = match parent {
        Some(pending::Parent::ImportRow { target, row }) => Some((target.clone(), row)),
        _ => saved
            .and_then(|s| s.operation.as_ref())
            .and_then(|op| op.parent.as_ref())
            .and_then(|owner| match owner {
                w::ApplicationContinuationTarget::Import(owner) => {
                    Some((owner.target.clone(), owner.row))
                }
                _ => None,
            }),
    };
    let owner_state = import_parent
        .as_ref()
        .map(|(target, _)| super::import::load(ctx.settings, &target.operation))
        .transpose()?;
    let import_row = owner_state
        .as_ref()
        .zip(import_parent.as_ref())
        .map(|(owner, (_, row))| {
            if owner.cursor() != *row {
                return Err(pending::failure(
                    ctx.settings,
                    &resolved.package,
                    "import parent cursor no longer names this exact row".into(),
                ));
            }
            owner
                .agent_modes(*row as usize)
                .filter(|_| owner.root_key(*row as usize) == Some(key.as_str()))
                .ok_or_else(|| {
                    pending::failure(
                        ctx.settings,
                        &resolved.package,
                        "import parent row no longer names this package".into(),
                    )
                })
        })
        .transpose()?;
    let plan = super::plan_option_with_modes(
        ctx,
        resolved,
        &option,
        request,
        previous,
        &index,
        (saved.and_then(|s| s.operation.as_ref()), import_row),
    )?;
    apply::admit_agents(ctx, resolved, &plan, request, saved)?;
    let target = if let Some(state) = saved {
        pending::validate_plan(ctx, resolved, &option, request, &plan, state)?;
        state
            .operation
            .as_ref()
            .ok_or_else(|| {
                pending::failure(
                    ctx.settings,
                    &plan.package,
                    "missing retained operation".into(),
                )
            })?
            .target
            .clone()
    } else {
        pending::begin(
            lock,
            ctx,
            resolved,
            &option,
            request,
            &plan,
            super::pending::Checkpoint {
                kind: w::ApplicationPendingOperationKind::Install,
                retirements: Vec::new(),
                predecessor: prior.as_ref().map(|p| p.package.key()).as_deref(),
                parent,
            },
        )?
    };
    if saved.is_none()
        && let Some(parent) = parent
    {
        match parent {
            pending::Parent::Dependency {
                target: owner,
                requirement,
            } => pending::link_child(ctx.settings, owner, &target, requirement)?,
            pending::Parent::ImportRow { target: owner, row } => {
                super::import::link_child(ctx.settings, owner, row, &target)?
            }
        }
    }
    pending::bind_resupply(ctx.settings, &target, &plan.secret_provided)?;
    pending::keep_work(ctx.settings, &target, request.keep_work)?;
    pending::before_dependencies(ctx.settings, &target)?;
    let paths = deps::install_required(
        lock,
        ctx,
        &plan,
        request.accept_license,
        stack,
        Some(&target),
    )?;
    let mut applied = apply::apply(
        ctx,
        &storage,
        &Selected {
            operation: Some(&target),
            resolved,
            option: &option,
            plan: &plan,
            prior: prior.as_ref(),
            library: request.library.as_deref(),
            force: request.force,
            mode: Mode::Install,
            start: &request.start,
            command: Command::Install,
            replaces: None,
            keep_work: request.keep_work,
            paths,
        },
    )?;
    let kept = prior.as_ref().map(|prior| &prior.lifecycle);
    applied.installation.lifecycle = Lifecycle {
        hidden: request.hidden,
        mark: match (request.mark, kept.map(|kept| kept.mark)) {
            (Mark::Auto, Some(Mark::Manual)) => Mark::Manual,
            (mark, _) => mark,
        },
        requires: deps::recorded(&plan),
        env: std::mem::take(&mut applied.installation.lifecycle.env),
        held: kept.is_some_and(|kept| kept.held),
        replaced: None,
        superseded_by: None,
    };
    if let Some(prior) = &prior {
        retire_env(ctx, &key, prior, &applied.installation)?;
    }
    let replaced = prior.is_some();
    if let Some(held) = owner_state
        .as_ref()
        .zip(import_parent.as_ref())
        .and_then(|(owner, (_, row))| owner.held(*row as usize))
    {
        applied.installation.lifecycle.held = held;
    }
    if let Some(binding) = ctx.import_scope.and_then(|scope| scope.binding(&key)) {
        applied.installation.lifecycle.held = binding.application.held;
    }
    let finish_failure = applied.finish_failure.take();
    let resumable_failure = request.keep_work
        || parent.is_some()
        || saved
            .and_then(|s| s.operation.as_ref())
            .is_some_and(|o| o.parent.is_some())
        || matches!(
            &finish_failure,
            Some(AppError::Agent(crate::AgentError::Parked(_)))
        );
    applied.installation.pending =
        (finish_failure.is_some() && resumable_failure).then(|| target.clone());
    pending::publishing(ctx.settings, &target, &applied.installation)?;
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
            command: Command::Install,
            preserve_default: saved
                .and_then(|s| s.operation.as_ref())
                .map(|op| pending::was_published(ctx.settings, &Index::load(ctx.settings)?, op))
                .transpose()?
                .unwrap_or(false),
        },
    )?;
    pending::phase(
        ctx.settings,
        &target,
        w::ApplicationPendingOperationPhase::Published,
    )?;
    if let Some(failure) = finish_failure {
        if !resumable_failure {
            pending::complete(lock, ctx.settings, &target)?;
        }
        return Err(failure);
    }
    pending::complete(lock, ctx.settings, &target)?;
    Ok(Outcome {
        message: format!(
            "{} {}{} via installer `{}`",
            if replaced { "replaced" } else { "installed" },
            outcome.key,
            if request.hidden { " hidden" } else { "" },
            option.id
        ),
        ..outcome
    })
}
