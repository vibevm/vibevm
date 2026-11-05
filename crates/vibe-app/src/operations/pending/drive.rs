//! Exact public continuation and the established explicit manual current-work selector.
use super::super::{OperationContext, Outcome};
use super::*;

/// Exact retained operation to continue; parameter values are ephemeral secret resupply.
#[derive(Clone, Default, PartialEq, Eq)]
#[specmark::spec(
    documents = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-CONTINUATION"
)]
pub struct ResumeRequest {
    pub key: String,
    pub operation: String,
    pub parameters: Vec<(String, String)>,
}
impl std::fmt::Debug for ResumeRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResumeRequest")
            .field("key", &self.key)
            .field("operation", &self.operation)
            .field(
                "parameter_ids",
                &self.parameters.iter().map(|(id, _)| id).collect::<Vec<_>>(),
            )
            .finish()
    }
}
/// Value-free context needed to select the currently available executor at the CLI boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
#[specmark::spec(
    documents = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-CONTINUATION"
)]
pub struct ResumeInfo {
    /// No installer or agent runs; only exact recorded cleanup is continued.
    pub cleanup_only: bool,
    pub kind: w::ApplicationPendingOperationKind,
    pub target: w::ApplicationPendingTarget,
    pub lookup: w::ApplicationPendingLookup,
    pub phase: w::ApplicationPendingOperationPhase,
    pub agent: Option<String>,
    pub agent_external: bool,
    pub hosted: bool,
    pub offline: bool,
}
/// Inspect an exact generation without creating a home or migrating any state.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-CONTINUATION"
)]
pub fn resume_info(
    settings: &SettingsRoot,
    key: &str,
    operation: &str,
) -> Result<ResumeInfo, AppError> {
    let package = coordinate(key)?;
    let state = load(settings, &package, Some(operation))?;
    let op = state
        .operation
        .ok_or_else(|| failure(settings, &package, "no retained operation".into()))?;
    Ok(ResumeInfo {
        cleanup_only: super::super::update::retirement::cleanup_only(&op),
        kind: op.kind,
        lookup: op.lookup,
        target: op.target,
        phase: op.phase,
        agent: op.request.agent,
        agent_external: op.request.agent_external,
        hosted: op.request.hosted,
        offline: op.request.offline,
    })
}
/// Continue precisely the requested saved operation; public reconfiguration is never inferred.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-CONTINUATION"
)]
pub fn resume(ctx: &OperationContext<'_>, request: &ResumeRequest) -> Result<Outcome, AppError> {
    let held = ctx.settings.lock()?;
    let lock = &held;
    let package = coordinate(&request.key)?;
    let state = load(ctx.settings, &package, Some(&request.operation))?;
    let incoming = InstallRequest {
        parameters: request.parameters.clone(),
        start: Start::Resume,
        ..Default::default()
    };
    continue_saved(lock, ctx, &state, &incoming, true, &mut Vec::new())
}

pub(crate) fn manual(
    lock: &crate::MutationLock,
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    request: &InstallRequest,
) -> Result<Option<Outcome>, AppError> {
    if request.start != Start::Resume {
        return Ok(None);
    }
    let Some(state) = StepsFile::load(&work(ctx.settings, &resolved.package))? else {
        return Ok(None);
    };
    let Some(op) = &state.operation else {
        return Ok(None);
    };
    if op.phase == w::ApplicationPendingOperationPhase::Complete {
        return Ok(None);
    }
    let state = load(ctx.settings, &resolved.package, Some(&op.target.operation))?;
    continue_saved(lock, ctx, &state, request, false, &mut Vec::new()).map(Some)
}

pub(crate) fn continue_target(
    lock: &crate::MutationLock,
    ctx: &OperationContext<'_>,
    target: &w::ApplicationPendingTarget,
    stack: &mut Vec<String>,
) -> Result<Outcome, AppError> {
    continue_target_with_parameters(lock, ctx, target, &[], stack)
}
pub(crate) fn continue_target_with_parameters(
    lock: &crate::MutationLock,
    ctx: &OperationContext<'_>,
    target: &w::ApplicationPendingTarget,
    parameters: &[(String, String)],
    stack: &mut Vec<String>,
) -> Result<Outcome, AppError> {
    let package = coordinate(&target.key)?;
    let state = load(ctx.settings, &package, Some(&target.operation))?;
    let incoming = InstallRequest {
        parameters: parameters.to_vec(),
        start: Start::Resume,
        ..Default::default()
    };
    continue_saved(lock, ctx, &state, &incoming, true, stack)
}

fn continue_saved(
    lock: &crate::MutationLock,
    ctx: &OperationContext<'_>,
    state: &StepsFile,
    incoming: &InstallRequest,
    canonical: bool,
    stack: &mut Vec<String>,
) -> Result<Outcome, AppError> {
    let op = state.operation.as_ref().ok_or_else(|| {
        failure(
            ctx.settings,
            &state.package,
            "legacy work has no exact operation target".into(),
        )
    })?;
    if op.phase == w::ApplicationPendingOperationPhase::Complete {
        return Err(crate::StepError::NothingToResume{key:op.target.key.clone(),work:work(ctx.settings,&state.package).display().to_string(),reason:"this exact execution generation is complete; use a deliberate manual retry or --from-step to reopen kept work".into()}.into());
    }
    if matches!(
        op.phase,
        w::ApplicationPendingOperationPhase::Removing
            | w::ApplicationPendingOperationPhase::Placing
    ) {
        return Err(failure(ctx.settings,&state.package,"removal or placement has an uncertain interrupted outcome; inspect and recover explicitly before fresh reconfiguration".into()));
    }
    if super::super::update::retirement::cleanup_only(op) {
        if !incoming.parameters.is_empty()
            || incoming.configuration.is_some()
            || !incoming.features.is_empty()
            || incoming.choice != crate::InstallChoice::default()
            || incoming.force
            || incoming.library.is_some()
        {
            return Err(failure(ctx.settings,&state.package,"record-owned cleanup consumes no fresh setup arguments; use the exact app resume command without setters".into()));
        }
        return super::super::update::retirement::resume(lock, ctx, state);
    }
    let scope_answers =
        crate::ImportSecrets::single(op.target.key.clone(), incoming.parameters.clone());
    let restored = if ctx.import_scope.is_none() {
        super::super::import::scope_from_state(ctx.settings, state, &scope_answers)?
    } else {
        None
    };
    let scoped = OperationContext {
        import_scope: ctx.import_scope.or(restored.as_ref()),
        ..*ctx
    };
    let ctx = &scoped;
    let resolved = source(ctx, op)?;
    let option = resolved
        .options
        .iter()
        .find(|o| {
            o.id == op.request.installer
                && o.platform == op.request.platform
                && o.arch == op.request.arch
        })
        .ok_or_else(|| {
            failure(
                ctx.settings,
                &state.package,
                "the retained installer selection is no longer available".into(),
            )
        })?;
    let request = input::restore(ctx.settings, op, option, incoming, canonical)?;
    let selected = if op.request.hosted || op.request.agent.is_none() {
        None
    } else {
        let selected=ctx.agent.selected.filter(|a|Some(&a.id)==op.request.agent.as_ref()&&(a.source==crate::AgentSource::External)==op.request.agent_external).ok_or_else(||failure(ctx.settings,&state.package,format!("the retained agent `{}` is not available from the current executor configuration",op.request.agent.as_deref().unwrap_or("unknown"))))?;
        Some(selected)
    };
    let agent = crate::AgentHooks {
        selected,
        hosted: op.request.hosted,
        assume_yes: op.request.assume_yes || ctx.agent.assume_yes,
        ..ctx.agent
    };
    let continued = OperationContext {
        offline: ctx.offline || op.request.offline,
        agent,
        ..*ctx
    };
    let mut outcome = match op.kind {
        w::ApplicationPendingOperationKind::Install => super::super::install_locked(
            lock,
            &continued,
            &resolved,
            &request,
            stack,
            Some(state),
            None,
        )?,
        w::ApplicationPendingOperationKind::Modify => {
            let modification = crate::ModifyRequest {
                configuration: None,
                installer: Some(request.choice.installer.clone().unwrap_or_default()),
                features: request.features.clone(),
                parameters: request.parameters.clone(),
                accept_license: request.accept_license,
                keep_work: request.keep_work,
            };
            super::super::modify::modify_locked(
                lock,
                &continued,
                &resolved,
                &modification,
                Some((state, &request)),
            )?
        }
        w::ApplicationPendingOperationKind::Update => {
            super::super::deps::enter(stack, &resolved.package.key())?;
            let result = super::super::update::update_locked(
                lock,
                &continued,
                &resolved,
                &request,
                stack,
                Some(state),
            );
            stack.pop();
            result?
        }
    };
    if let Some(parent) = &op.parent {
        outcome.message.push_str(&format!(
            "; continue parent with `{}`",
            continuation_command(parent)
        ));
    }
    Ok(outcome)
}
