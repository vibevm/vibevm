//! Pure step disposition: guards, host needs and dependency selection.
use super::*;
use crate::error_lifecycle::LifecycleError;
use crate::steps::params::{param_equals, param_holds};
use crate::steps::sizes::{input_local, source_cached};
use vibe_core::judgment;
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
    // `file_present` paths are expanded first, so this plane's typed
    // template refusal still reaches the caller unchanged and the shared
    // evaluator stays pure.
    let files = expanded_files(vars, step)?;
    let facts = judgment::ConditionFacts {
        feature_selected: &|id| vars.features.iter().any(|chosen| chosen == id),
        param_truthy: &|id| Ok(param_holds(input.option, vars.params, id)),
        param_equals: &|id, value| Ok(param_equals(input.option, vars.params, id, value)),
        command_present: input.facts.command_present,
        file_present: &|path| {
            Ok(files
                .iter()
                .find(|(declared, _)| *declared == path)
                .is_some_and(|(_, expanded)| input.resolved.root.join(expanded).exists()))
        },
        env_present: input.facts.env_present,
        agent_configured: input.agent.available(),
        offline: input.offline,
    };
    if let judgment::Verdict::Skip { reason } = judgment::verdict(&step.when, &step.unless, &facts)
        .unwrap_or_else(|reason| judgment::Verdict::Skip { reason })
    {
        return skip(reason);
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

/// Every `file_present` path this step declares, expanded through the
/// template layer once, so the shared evaluator asks only "does it exist".
fn expanded_files<'s>(
    vars: &Vars<'_>,
    step: &'s StepDecl,
) -> Result<Vec<(&'s str, std::path::PathBuf)>, AppError> {
    let mut files = Vec::new();
    for condition in step.when.iter().chain(step.unless.iter()) {
        if let Condition::FilePresent(path) = condition {
            let expanded = template::expand(path, vars, &step.id, "file_present")?;
            files.push((path.as_str(), std::path::PathBuf::from(expanded)));
        }
    }
    Ok(files)
}

/// Whether a need of a running step is met on this host or by the public
/// commands of its requirements (`PROP-065#app-dependencies`).
///
/// The vocabulary's meaning is `vibe_core::judgment::need_met`; what this
/// plane adds is the one answer that plane cannot give — whether an offline
/// run can still serve a step's own inputs from the cache, which depends on
/// the step's kind and on an installation work directory the ordinary plane
/// has none of.
fn need_met(
    input: &PlanInput<'_>,
    need: &Need,
    step: &StepDecl,
    work: &Path,
    provided: &[&str],
) -> bool {
    let network = !input.offline
        || match &step.kind {
            StepKind::Sources { sources } => sources
                .iter()
                .all(|source| source.url.is_none() || source_cached(input.settings, source)),
            StepKind::Archive { source, sha256, .. } | StepKind::Native { source, sha256, .. } => {
                input_local(input.settings, work, step, source, sha256)
            }
            _ => false,
        };
    judgment::need_met(
        need,
        &judgment::NeedFacts {
            command_present: &|name| {
                provided.contains(&name) || (input.facts.command_present)(name)
            },
            network,
            agent: input.agent.available(),
            terminal: input.attended,
        },
    )
}
