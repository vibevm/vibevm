//! Operation composition around the existing configuration/step planner.
use super::*;
use crate::steps::{self, PlanInput, Previous, Prompts, RequirementResolver};
/// Plan one installation without downloading or running anything
/// (`vibe app plan`, `install --plan-only`): the choice, features,
/// parameters and license acceptance of `request`.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-JOURNEY"
)]
pub fn plan(
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    request: &InstallRequest,
) -> Result<Plan, AppError> {
    let option = choose(ctx, resolved, &request.choice)?;
    let index = Index::load(ctx.settings)?;
    let mut plan = plan_option(ctx, resolved, &option, request, None, &index, None)?;
    plan.suggested = resolved
        .suggests
        .packages
        .iter()
        .map(ToString::to_string)
        .collect();
    if request.recommends {
        plan.recommended = super::recommends::plan(ctx, &index, resolved);
    }
    Ok(plan)
}

/// Compute the plan of the selected option and show it.
pub(super) fn plan_option(
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    option: &InstallerOptionDecl,
    request: &InstallRequest,
    previous: Option<Previous<'_>>,
    index: &Index,
    pending: Option<&vibe_wire::generated::shared::ApplicationPendingOperation>,
) -> Result<Plan, AppError> {
    plan_option_with_modes(
        ctx,
        resolved,
        option,
        request,
        previous,
        index,
        (pending, None),
    )
}

pub(super) fn plan_option_with_modes(
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    option: &InstallerOptionDecl,
    request: &InstallRequest,
    previous: Option<Previous<'_>>,
    index: &Index,
    continuation: (
        Option<&vibe_wire::generated::shared::ApplicationPendingOperation>,
        Option<
            &std::collections::BTreeMap<String, vibe_wire::generated::shared::ApplicationAgentMode>,
        >,
    ),
) -> Result<Plan, AppError> {
    match prepare_option(
        ctx,
        resolved,
        option,
        request,
        previous,
        index,
        PreparedContinuation {
            pending: continuation.0,
            modes: continuation.1,
            request_node: false,
            prepared: None,
        },
    )? {
        steps::PreparedInstall::Ready(plan) => Ok(*plan),
        steps::PreparedInstall::NeedsNode(_) => {
            unreachable!("ordinary planning cannot request Node")
        }
    }
}

pub(super) struct PreparedContinuation<'a> {
    pub pending: Option<&'a vibe_wire::generated::shared::ApplicationPendingOperation>,
    pub modes: Option<
        &'a std::collections::BTreeMap<String, vibe_wire::generated::shared::ApplicationAgentMode>,
    >,
    pub request_node: bool,
    pub prepared: Option<steps::PreparedSelection>,
}

pub(super) fn prepare_option(
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    option: &InstallerOptionDecl,
    request: &InstallRequest,
    previous: Option<Previous<'_>>,
    index: &Index,
    continuation: PreparedContinuation<'_>,
) -> Result<steps::PreparedInstall, AppError> {
    let PreparedContinuation {
        pending,
        modes,
        request_node,
        prepared,
    } = continuation;
    let owner = resolved.package.key();
    if let Some(required) = &option.system_python {
        ctx.facts.system_python
            .ok_or_else(|| crate::StepError::RuntimeRemediationRefused { key: owner.clone(), reason: "system Python observation unavailable; install a compatible system CPython 3.14 and retry".into() })?()
            .and_then(|observed| observed.admit(required))
            .map_err(|reason| crate::StepError::RuntimeRemediationRefused { key: owner.clone(), reason })?;
    }
    let requirements = |spelling: &str, scope, step: Option<&str>| {
        if let Some(operation) = pending {
            let binding = operation
                .children
                .iter()
                .find(|b| b.requirement == spelling && b.step.as_deref() == step)
                .ok_or_else(|| {
                    pending::failure(
                        ctx.settings,
                        &resolved.package,
                        "current requirements differ from the retained dependency plan".into(),
                    )
                })?;
            return pending::resolve_binding(ctx, index, operation, binding, scope, step);
        }
        deps::resolve_requirement(
            ctx,
            index,
            &owner,
            spelling,
            scope,
            step,
            request.accept_license,
        )
    };
    let agent_modes = pending
        .map(|op| &op.agent_modes)
        .or(modes)
        .or_else(|| {
            ctx.import_scope
                .and_then(|scope| scope.binding(&resolved.package.key()))
                .map(|binding| &binding.agent_modes)
        })
        .map(|modes| {
            modes
                .iter()
                .map(|(id, mode)| {
                    (
                        id.clone(),
                        match mode {
                            vibe_wire::generated::shared::ApplicationAgentMode::Interactive => {
                                crate::RunMode::Interactive
                            }
                            vibe_wire::generated::shared::ApplicationAgentMode::Unattended => {
                                crate::RunMode::Unattended
                            }
                        },
                    )
                })
                .collect::<std::collections::BTreeMap<_, _>>()
        });
    // Original/prefix provision metadata remains unchanged in steps2. The
    // monotonic current resupply list only strengthens the planner's requirement.
    let mut secret_origins = previous.map(|p| p.secret_origins.clone());
    let mut configuration = previous.and_then(|p| p.configuration.cloned());
    if let Some(op) = pending {
        for id in &op.secret_required {
            use vibe_wire::generated::shared as w;
            let explicit = || {
                w::ConfigurationParameterOrigin::Explicit(Box::new(
                    w::ConfigurationParameterOriginExplicit {},
                ))
            };
            if let Some(origins) = &mut secret_origins {
                origins.insert(id.clone(), explicit());
            }
            if let Some(record) = &mut configuration {
                for parameter in &mut record.evaluation.parameters {
                    if let w::ConfigurationParameterResult::Secret(secret) = parameter
                        && &secret.id == id
                    {
                        secret.provided = true;
                        secret.origin = explicit();
                    }
                }
            }
        }
    }
    let previous = previous.map(|p| Previous {
        secret_origins: secret_origins.as_ref().unwrap_or(p.secret_origins),
        configuration: configuration.as_ref(),
        ..p
    });
    let input = PlanInput {
        import_scope: ctx.import_scope.map(|s| s.fingerprint()),
        transfer: request.transfer.as_ref(),
        agent_modes: agent_modes.as_ref(),
        configuration: request.configuration.as_ref(),
        resolved,
        option,
        request: &request.features,
        parameters: &request.parameters,
        accept_license: request.accept_license,
        previous,
        settings: ctx.settings,
        facts: ctx.facts,
        offline: ctx.offline,
        attended: ctx.chooser.is_some(),
        prompts: Prompts {
            chooser: ctx.chooser,
            features: ctx.feature_chooser,
            parameter: ctx.parameter_prompt,
            license: ctx.license_prompt,
        },
        agent: ctx.agent,
        requirements: Some(&requirements as RequirementResolver<'_>),
        required_by: request.required_by.clone(),
    };
    let plan = match prepared {
        Some(prepared) => steps::PreparedInstall::Ready(Box::new(prepared.continue_plan(&input)?)),
        None => steps::prepare_install(&input, request_node)?,
    };
    if let steps::PreparedInstall::Ready(ready) = &plan
        && let Some(show) = ctx.on_plan
    {
        show(ready);
    }
    Ok(plan)
}

/// Common plan view: unresolved questions carry no executable plan.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-069#APP-CONFIGURATION-JOURNEY"
)]
pub fn configuration_plan(
    ctx: &OperationContext<'_>,
    resolved: &ResolvedPackage,
    request: &InstallRequest,
) -> Result<vibe_wire::generated::configuration::e1::app_plan::AppPlan, AppError> {
    match plan(ctx, resolved, request) {
        Ok(plan) => plan.to_configuration_wire(),
        Err(AppError::Configuration(crate::configuration::ConfigurationError::Unresolved {
            option,
            evaluation,
        })) => {
            let selected = resolved
                .options
                .iter()
                .find(|o| o.id == option)
                .ok_or_else(|| crate::ResolveError::NoOption {
                    detail: option.clone(),
                    choices: resolved
                        .options
                        .iter()
                        .map(|o| o.id.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                })?;
            Ok(vibe_wire::generated::configuration::e1::app_plan::AppPlan {
                source_hosting: vibe_registry::source_hosting(resolved.source_evidence.as_ref()),
                source_evidence: resolved.source_evidence.clone(),
                protocol: "vibe-app-configuration-plan/1".into(),
                package: vibe_wire::generated::configuration::e1::app_plan::Package {
                    group: resolved.package.group.clone(),
                    name: resolved.package.name.clone(),
                    version: resolved.package.version.clone(),
                },
                option,
                platform: selected.platform.clone(),
                arch: selected.arch.clone(),
                configuration: *evaluation,
                execution: None,
            })
        }
        Err(error) => Err(error),
    }
}
