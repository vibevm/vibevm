//! Pure step disposition: guards, host needs and dependency selection.
use super::*;
use crate::error_lifecycle::LifecycleError;
use crate::steps::params::{param_equals, param_holds};
use crate::steps::sizes::{input_local, source_cached};
use vibe_core::manifest::{AgentMode, Condition, Need, RequirementDecl};
pub(super) const NO_AGENT: &str = "no agent configured";
pub(super) fn effective_agent_mode(
    input: &PlanInput<'_>,
    step: &str,
    mode: Option<AgentMode>,
) -> crate::RunMode {
    input
        .agent_modes
        .and_then(|m| m.get(step))
        .copied()
        .unwrap_or_else(|| crate::steps::agent::mode_of(mode, input.attended))
}

/// Resolve one requirement through the caller's resolver.
pub(super) fn require(
    input: &PlanInput<'_>,
    key: &str,
    requirement: &RequirementDecl,
    default: RequirementScope,
    step: Option<&str>,
) -> Result<PlannedDependency, AppError> {
    let resolver = input
        .requirements
        .ok_or_else(|| LifecycleError::RequirementUnresolved {
            key: key.into(),
            requirement: requirement.package().into(),
            reason: "no package resolver is available in this context".into(),
        })?;
    resolver(requirement.package(), requirement.scope_or(default), step)
}

/// One step's decision: whether it runs, why not, and the requirements
/// it brings when it runs.
type Decision = (bool, Option<String>, Vec<PlannedDependency>);

fn skip(reason: String) -> Result<Decision, AppError> {
    Ok((false, Some(reason), Vec::new()))
}

/// Run or skip one step, or refuse the whole plan. `provided` names the
/// public commands of the option-level requirements.
pub(super) fn decide(
    input: &PlanInput<'_>,
    step: &StepDecl,
    vars: &Vars<'_>,
    work: &Path,
    key: &str,
    provided: &[String],
) -> Result<Decision, AppError> {
    for condition in &step.when {
        if !holds(input, condition, vars, step)? {
            return skip(format!("when `{condition}` does not hold"));
        }
    }
    for condition in &step.unless {
        if holds(input, condition, vars, step)? {
            return skip(format!("unless `{condition}` holds"));
        }
    }
    if let StepKind::Agent { required, mode, .. } = &step.kind {
        // `PROP-064#selection`: no agent means skip, unless the step is
        // required; `#modes`: interactive needs a terminal unless parked.
        if !input.agent.available() {
            if *required {
                return Err(StepError::AgentRequired {
                    key: key.into(),
                    step: step.id.clone(),
                    reason: step
                        .description
                        .clone()
                        .unwrap_or_else(|| step.title.clone()),
                }
                .into());
            }
            return skip(NO_AGENT.into());
        }
        if effective_agent_mode(input, &step.id, *mode) == crate::RunMode::Interactive
            && !input.attended
            && !input.agent.hosted
        {
            if step.optional {
                return skip("unmet need `terminal`".into());
            }
            return Err(StepError::UnmetNeed {
                key: key.into(),
                step: step.id.clone(),
                need: Need::Terminal.to_string(),
            }
            .into());
        }
    }
    let mut needed = Vec::new();
    for requirement in &step.requires {
        match require(
            input,
            key,
            requirement,
            RequirementScope::Build,
            Some(&step.id),
        ) {
            Ok(dependency) => needed.push(dependency),
            Err(error) if step.optional => {
                return skip(format!("unresolved requirement: {error}"));
            }
            Err(error) => return Err(error),
        }
    }
    let commands: Vec<&str> = provided
        .iter()
        .map(String::as_str)
        .chain(
            needed
                .iter()
                .flat_map(|dependency| dependency.commands.iter().map(String::as_str)),
        )
        .collect();
    for need in &step.needs {
        if need_met(input, need, step, work, &commands) {
            continue;
        }
        if step.optional {
            return skip(format!("unmet need `{need}`"));
        }
        return Err(StepError::UnmetNeed {
            key: key.into(),
            step: step.id.clone(),
            need: need.to_string(),
        }
        .into());
    }
    Ok((true, None, needed))
}

fn holds(
    input: &PlanInput<'_>,
    condition: &Condition,
    vars: &Vars<'_>,
    step: &StepDecl,
) -> Result<bool, AppError> {
    Ok(match condition {
        Condition::Feature(id) => vars.features.iter().any(|chosen| chosen == id),
        Condition::Param(id) => param_holds(input.option, vars.params, id),
        Condition::ParamEquals(test) => {
            param_equals(input.option, vars.params, &test.id, &test.value)
        }
        Condition::CommandPresent(name) => (input.facts.command_present)(name),
        Condition::EnvPresent(name) => (input.facts.env_present)(name),
        Condition::AgentConfigured(value) => input.agent.available() == *value,
        Condition::Offline(value) => input.offline == *value,
        Condition::FilePresent(path) => {
            let expanded = template::expand(path, vars, &step.id, "file_present")?;
            input.resolved.root.join(expanded).exists()
        }
    })
}

/// Whether a need of a running step is met on this host or by the public
/// commands of its requirements (`PROP-065#app-dependencies`).
fn need_met(
    input: &PlanInput<'_>,
    need: &Need,
    step: &StepDecl,
    work: &Path,
    provided: &[&str],
) -> bool {
    match need {
        Need::Command(name) => {
            provided.contains(&name.as_str()) || (input.facts.command_present)(name)
        }
        Need::Agent => input.agent.available(),
        Need::Terminal => input.attended,
        Need::Network if !input.offline => true,
        Need::Network => match &step.kind {
            StepKind::Sources { sources } => sources
                .iter()
                .all(|source| source.url.is_none() || source_cached(input.settings, source)),
            StepKind::Archive { source, sha256, .. } | StepKind::Native { source, sha256, .. } => {
                input_local(work, step, source, sha256)
            }
            _ => false,
        },
    }
}
