//! Full portable roots use the ordinary installer and the existing cohort slot.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTINUATION");
use super::{full_store, pending, rows, store};
use crate::portable::full_prepare::PreparedFullImport;
use crate::{AppError, ImportSecrets, MutationLock, OperationContext};
use vibe_wire::generated::{application::e3::import_work as wire, shared as w};
#[path = "full/settlement.rs"]
mod settlement;
use settlement::{admit_prefix, advance, finish, release_refused, settle_error, skip, wait_error};

#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTINUATION"
)]
pub fn import_full(
    ctx: &OperationContext<'_>,
    prepared: &mut PreparedFullImport,
    secrets: &ImportSecrets,
) -> Result<(), AppError> {
    let lock = ctx.settings.lock()?;
    let target = w::ApplicationImportTarget {
        home: pending::actual_home(ctx.settings)?,
        operation: pending::generation(),
    };
    let mut document = wire::ImportWork {
        kind: "full".into(),
        source_evidence: prepared.source_evidence.clone(),
        protocol: full_store::PROTOCOL.into(),
        target,
        phase: w::ApplicationImportPhase::Running,
        context: w::ApplicationImportContext {
            lookup: pending::lookup(ctx)?,
            offline: ctx.offline,
            hosted: ctx.agent.hosted,
            assume_yes: ctx.agent.assume_yes,
            agent: ctx.agent.selected.map(|a| a.id.clone()),
            agent_external: ctx
                .agent
                .selected
                .is_some_and(|a| a.source == crate::AgentSource::External),
        },
        cohort: prepared.state.clone(),
    };
    let mut bytes = full_store::encode(&document).map_err(|_| {
        store::failure(
            ctx.settings,
            &document.target.operation,
            "cannot encode prepared full import",
        )
    })?;
    bytes.push(b'\n');
    store::reserve(
        ctx.settings,
        &document.target.operation,
        &bytes,
        ctx.on_notice,
    )?;
    prepared.target = Some(document.target.clone());
    let result = drive(ctx, &lock, &mut document, secrets);
    prepared.state = document.cohort;
    result
}
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-CONTINUATION"
)]
pub fn resume_full_import(
    ctx: &OperationContext<'_>,
    id: &str,
    secrets: &ImportSecrets,
) -> Result<PreparedFullImport, AppError> {
    let lock = ctx.settings.lock()?;
    let super::document::ImportWork::Full(mut document) = full_store::load(ctx.settings, id)?
    else {
        return Err(store::failure(
            ctx.settings,
            id,
            "this is not a full import cohort",
        ));
    };
    let cleanup_only = super::inspect_import_resume(ctx.settings, id)?.cleanup_only;
    if cleanup_only {
        // Completed execution consumes neither newly supplied answers nor source/executor data.
        if let Some(target) = document.cohort.current.clone() {
            let state = pending::load(
                ctx.settings,
                &pending::coordinate(&target.key)?,
                Some(&target.operation),
            )?;
            if state.pending(ctx.settings)?.phase != w::ApplicationPendingOperationPhase::Complete {
                pending::continue_target_with_parameters(&lock, ctx, &target, &[], &mut vec![])?;
            }
            pending::ack_import(ctx.settings, &target)?;
            advance(
                ctx,
                &mut document,
                &target.key,
                state.pending(ctx.settings)?.request.installer.clone(),
                target.clone(),
            )?;
        }
        finish(ctx, &lock, &mut document)?;
        return Ok(PreparedFullImport {
            source_evidence: document.source_evidence.clone(),
            state: document.cohort,
            target: Some(document.target),
        });
    }
    let saved = &document.context;
    let selected = if cleanup_only || saved.hosted || saved.agent.is_none() {
        None
    } else {
        Some(
            ctx.agent
                .selected
                .filter(|agent| {
                    Some(&agent.id) == saved.agent.as_ref()
                        && (agent.source == crate::AgentSource::External) == saved.agent_external
                })
                .ok_or_else(|| {
                    store::failure(
                        ctx.settings,
                        id,
                        "retained executor is unavailable in this invocation",
                    )
                })?,
        )
    };
    let continued = OperationContext {
        offline: ctx.offline || saved.offline,
        agent: crate::AgentHooks {
            selected,
            hosted: !cleanup_only && saved.hosted,
            assume_yes: saved.assume_yes || ctx.agent.assume_yes,
            ..ctx.agent
        },
        ..*ctx
    };
    let ctx = &continued;
    let scope = crate::ImportScope::new(
        document.target.clone(),
        document.cohort.roots.clone(),
        document.cohort.bindings.clone(),
        document.cohort.secret_requirements.clone(),
        secrets,
    )?;
    settlement::verify_scope(ctx.settings, &document, scope.fingerprint())?;
    admit_prefix(ctx, &document)?;
    super::receipts::recover(&lock, ctx.settings, &mut document, scope.fingerprint())?;
    let index = crate::Index::load(ctx.settings)?;
    let mut admitted = Vec::new();
    for key in secrets.keys() {
        if scope.completed_binding(ctx.settings, key, &index)? {
            continue;
        }
        let binding = scope.binding(key).ok_or_else(|| store::failure(ctx.settings, id, "secret input is unused: recipient is outside the selected roots and required closure"))?;
        let (_, option) = scope.admit(ctx, binding)?;
        let declarations = option
            .configuration
            .as_ref()
            .map(|d| d.parameters.as_slice())
            .unwrap_or(&option.parameters);
        let ids = secrets
            .for_key(key)
            .iter()
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        if ids
            .iter()
            .any(|id| !declarations.iter().any(|p| &p.id == id && p.secret))
        {
            return Err(store::failure(
                ctx.settings,
                id,
                "secret input names an unknown or public parameter",
            ));
        }
        admitted.push((key.to_owned(), ids));
    }
    for (key, ids) in admitted {
        let required = document.cohort.secret_requirements.entry(key).or_default();
        required.extend(ids);
        required.sort();
        required.dedup();
    }
    full_store::save_full(ctx.settings, &document)?;
    drive(ctx, &lock, &mut document, secrets)?;
    Ok(PreparedFullImport {
        source_evidence: document.source_evidence.clone(),
        state: document.cohort,
        target: Some(document.target),
    })
}

fn drive(
    ctx: &OperationContext<'_>,
    lock: &MutationLock,
    document: &mut wire::ImportWork,
    secrets: &ImportSecrets,
) -> Result<(), AppError> {
    admit_prefix(ctx, document)?;
    let scope = crate::ImportScope::new(
        document.target.clone(),
        document.cohort.roots.clone(),
        document.cohort.bindings.clone(),
        document.cohort.secret_requirements.clone(),
        secrets,
    )?;
    settlement::verify_scope(ctx.settings, document, scope.fingerprint())?;
    super::receipts::recover(lock, ctx.settings, document, scope.fingerprint())?;
    release_refused(ctx, lock, document, scope.fingerprint())?;
    let scoped = OperationContext {
        import_scope: Some(&scope),
        ..*ctx
    };
    let ctx = &scoped;
    while let Some(key) = document
        .cohort
        .roots
        .get(document.cohort.cursor as usize)
        .cloned()
    {
        if let Some(target) = document.cohort.current.clone() {
            let state = pending::load(
                ctx.settings,
                &pending::coordinate(&key)?,
                Some(&target.operation),
            )?;
            if state.pending(ctx.settings)?.phase == w::ApplicationPendingOperationPhase::Complete {
                if !pending::matches_import_scope(&state, scope.fingerprint()) {
                    return Err(store::failure(
                        ctx.settings,
                        &document.target.operation,
                        "completed root import scope changed",
                    ));
                }
                pending::ack_import(ctx.settings, &target)?;
                advance(
                    ctx,
                    document,
                    &key,
                    state.pending(ctx.settings)?.request.installer.clone(),
                    target,
                )?;
                continue;
            }
            let outcome = pending::continue_target_with_parameters(
                lock,
                ctx,
                &target,
                scope.answers(&key),
                &mut vec![],
            );
            if let super::document::ImportWork::Full(saved) =
                full_store::load(ctx.settings, &document.target.operation)?
            {
                *document = saved;
            }
            match outcome {
                Ok(_) => {
                    pending::ack_import(ctx.settings, &target)?;
                    advance(
                        ctx,
                        document,
                        &key,
                        state.pending(ctx.settings)?.request.installer.clone(),
                        target,
                    )?;
                    continue;
                }
                Err(error) => {
                    settle_error(
                        ctx,
                        lock,
                        document,
                        &key,
                        scope.fingerprint(),
                        scope.input_error(&key, error),
                    )?;
                    continue;
                }
            }
        }
        let binding = document.cohort.bindings.get(&key).cloned().ok_or_else(|| {
            store::failure(
                ctx.settings,
                &document.target.operation,
                "missing prepared root binding",
            )
        })?;
        let index = crate::Index::load(ctx.settings)?;
        if scope.completed_binding(ctx.settings, &key, &index)? {
            skip(
                ctx,
                document,
                &key,
                "already applied by this exact cohort as a dependency",
            )?;
            continue;
        }
        let (resolved, option) = scope.admit(ctx, &binding)?;
        if let Some(installed) = index.installations.get(&key) {
            match scope.compatible(ctx, &binding, &option, installed) {
                Ok(()) => {
                    skip(
                        ctx,
                        document,
                        &key,
                        "existing installation already matches full configuration",
                    )?;
                    continue;
                }
                Err(_) if document.cohort.skip_installed => {
                    skip(
                        ctx,
                        document,
                        &key,
                        "kept installed choice; imported configuration was not applied",
                    )?;
                    continue;
                }
                Err(error) => return Err(error),
            }
        }
        let request = match scope.request(ctx, &binding, &option, document.cohort.accept_license) {
            Ok(request) => request,
            Err(error) => return wait_error(ctx, document, scope.input_error(&key, error)),
        };
        let result = super::super::install_locked(
            lock,
            ctx,
            &resolved,
            &request,
            &mut vec![],
            None,
            Some(pending::Parent::ImportRow {
                target: &document.target,
                row: document.cohort.cursor,
            }),
        );
        let fresh = full_store::load(ctx.settings, &document.target.operation)?;
        if let super::document::ImportWork::Full(saved) = fresh {
            *document = saved;
        } else {
            return Err(store::failure(
                ctx.settings,
                &document.target.operation,
                "full cohort protocol changed",
            ));
        }
        match result {
            Ok(_) => {
                let target = document.cohort.current.clone().ok_or_else(|| {
                    store::failure(
                        ctx.settings,
                        &document.target.operation,
                        "completed root has no exact attempt",
                    )
                })?;
                pending::ack_import(ctx.settings, &target)?;
                advance(ctx, document, &key, binding.installer, target)?;
            }
            Err(error) => settle_error(
                ctx,
                lock,
                document,
                &key,
                scope.fingerprint(),
                scope.input_error(&key, error),
            )?,
        }
    }
    finish(ctx, lock, document)
}
