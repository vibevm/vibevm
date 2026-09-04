//! Phase execution over an already prepared plan.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-C");

use std::collections::BTreeMap;
use std::path::Path;

use super::backend::HealthBackend;
use super::model::*;
use super::protocol::parse_health_result;

pub fn run_phase<B: HealthBackend>(
    backend: &mut B,
    prepared: &PreparedHealth,
    context: &PhaseContext,
) -> Result<PhaseHealthResult, HealthError> {
    if let Some(blocker) = prepared.blockers.first() {
        return Err(HealthError::Preparation(format!(
            "prepared health is blocked by `{}`: {}",
            blocker.code, blocker.message
        )));
    }
    let capabilities = backend.capabilities();
    for check in prepared
        .checks
        .iter()
        .filter(|check| check.applicability == Applicability::Applicable)
    {
        if !capabilities.satisfies(check.sandbox, context.same_display_path_required) {
            return Err(HealthError::Unsupported(format!(
                "backend capabilities do not satisfy healthcheck `{}` requirements",
                check.id
            )));
        }
    }
    let mut results = Vec::with_capacity(prepared.checks.len());
    // Final-path execution guarded by complete tree reproof is honest and
    // transaction-safe, but it is a narrower assurance than an enforced COW
    // view and must never be reported as full isolation.
    let mut reduced = context.transactional_tree_reproof;
    for check in &prepared.checks {
        if let Applicability::SkippedWhenMissing { path } = &check.applicability {
            reduced = true;
            results.push(CheckResult {
                id: check.id.clone(),
                state: CheckState::Skipped {
                    reason: format!("when path `{path}` is absent"),
                },
                commands: Vec::new(),
            });
            continue;
        }
        if !check.assurance_reductions.is_empty() {
            reduced = true;
        }
        let assets = check
            .assets
            .iter()
            .map(|asset| (asset.id.as_str(), asset))
            .collect::<BTreeMap<_, _>>();
        let root = check_root(&context.root, &check.root);
        let scratch = check_root(&context.scratch, &check.id);
        let result_path = check_root(&context.result, &format!("{}.json", check.id));
        let mut executions = Vec::with_capacity(check.commands.len());
        for command in &check.commands {
            let executable = assets
                .get(command.executable_asset_id.as_str())
                .ok_or_else(|| {
                    HealthError::Preparation(format!(
                        "healthcheck `{}` command names absent asset `{}`",
                        check.id, command.executable_asset_id
                    ))
                })?;
            let expanded = expand_command(
                command,
                executable,
                &assets,
                context.phase,
                &root,
                &scratch,
                &result_path,
            )?;
            let execution = backend
                .execute(BackendCommandRequest {
                    check_id: check.id.clone(),
                    phase: context.phase,
                    phase_root: context.root.clone(),
                    root: root.clone(),
                    protected_root: context.protected_root.clone(),
                    scratch: scratch.clone(),
                    result: result_path.clone(),
                    command: expanded,
                    assets: &check.assets,
                    effects: check.effects.clone(),
                    network: check.network,
                    custom_bundle: check.custom_bundle.as_ref(),
                    expected_tree: &context.expected_tree,
                    transactional_tree_reproof: context.transactional_tree_reproof,
                    cancellation: context.cancellation.clone(),
                    timeout_seconds: check.timeout_seconds,
                    termination_grace_seconds: prepared.termination_grace_seconds,
                    max_result_bytes: prepared.max_result_bytes,
                    max_stdout_bytes: prepared.max_stdout_bytes,
                    max_stderr_bytes: prepared.max_stderr_bytes,
                })
                .map_err(|error| attach_prior_evidence(error, &results, &executions))?;
            validate_stream_bound(&execution.stdout, prepared.max_stdout_bytes, "stdout")?;
            validate_stream_bound(&execution.stderr, prepared.max_stderr_bytes, "stderr")?;
            if !command.accepted_exit_codes.contains(&execution.exit_code) {
                return Err(HealthError::CommandFailed {
                    check_id: check.id.clone(),
                    exit_code: execution.exit_code,
                    prior_checks: results,
                    prior_executions: executions,
                    execution: Box::new(execution),
                });
            }
            executions.push(execution);
        }
        let verdict = match check.protocol {
            ResultProtocol::BuiltIn | ResultProtocol::ExitCode => HealthVerdict::Pass,
            ResultProtocol::VibeHealthJsonV1 => {
                let result = executions
                    .last()
                    .and_then(|execution| execution.result.as_deref())
                    .ok_or_else(|| "command produced no atomic JSON result".to_owned());
                let parsed = result.and_then(|result| {
                    let cap = usize::try_from(prepared.max_result_bytes)
                        .map_err(|_| "prepared result cap exceeds platform usize".to_owned())?;
                    parse_health_result(result, cap).map_err(|error| error.to_string())
                });
                match parsed {
                    Ok(verdict) => HealthVerdict::Structured(verdict),
                    Err(detail) => {
                        return Err(HealthError::CheckProtocolFailed {
                            check_id: check.id.clone(),
                            detail,
                            prior_checks: results,
                            executions,
                        });
                    }
                }
            }
        };
        results.push(CheckResult {
            id: check.id.clone(),
            state: CheckState::Completed(verdict),
            commands: executions,
        });
    }
    let observed = backend.reprove_tree(context)?;
    let differences = context.expected_tree.compare(&observed);
    if !differences.is_empty() {
        return Err(HealthError::Tree(format!(
            "source/delivered tree changed during {:?} health: {differences:?}",
            context.phase
        )));
    }
    Ok(PhaseHealthResult {
        phase: context.phase,
        plan_id: prepared.plan_id.clone(),
        checks: results,
        assurance_reduced: reduced,
    })
}

fn attach_prior_evidence(
    error: HealthError,
    completed_checks: &[CheckResult],
    completed: &[CommandExecution],
) -> HealthError {
    match error {
        HealthError::CommandChangedTree {
            check_id,
            detail,
            mut prior_checks,
            mut prior_executions,
            execution,
        } => {
            let mut prior = completed.to_vec();
            prior.append(&mut prior_executions);
            let mut checks = completed_checks.to_vec();
            checks.append(&mut prior_checks);
            HealthError::CommandChangedTree {
                check_id,
                detail,
                prior_checks: checks,
                prior_executions: prior,
                execution,
            }
        }
        HealthError::Cancelled {
            phase,
            check_id,
            disposition,
            mut prior_checks,
            mut prior_executions,
            execution,
        } => {
            let mut prior = completed.to_vec();
            prior.append(&mut prior_executions);
            let mut checks = completed_checks.to_vec();
            checks.append(&mut prior_checks);
            HealthError::Cancelled {
                phase,
                check_id,
                disposition,
                prior_checks: checks,
                prior_executions: prior,
                execution,
            }
        }
        HealthError::TimedOut {
            phase,
            check_id,
            timeout_seconds,
            mut prior_checks,
            mut prior_executions,
            execution,
        } => {
            let mut prior = completed.to_vec();
            prior.append(&mut prior_executions);
            let mut checks = completed_checks.to_vec();
            checks.append(&mut prior_checks);
            HealthError::TimedOut {
                phase,
                check_id,
                timeout_seconds,
                prior_checks: checks,
                prior_executions: prior,
                execution,
            }
        }
        other => other,
    }
}

fn expand_command(
    command: &PreparedCommand,
    executable: &AssetIdentity,
    assets: &BTreeMap<&str, &AssetIdentity>,
    phase: HealthPhase,
    root: &str,
    scratch: &str,
    result: &str,
) -> Result<ExpandedCommand, HealthError> {
    let argv = command
        .argv
        .iter()
        .map(|argument| match argument {
            PreparedArg::Literal(value) => ExpandedArg::Value(value.clone()),
            PreparedArg::Root => ExpandedArg::Value(root.to_owned()),
            PreparedArg::Scratch => ExpandedArg::Value(scratch.to_owned()),
            PreparedArg::Result => ExpandedArg::Value(result.to_owned()),
            PreparedArg::Phase => ExpandedArg::Value(phase.as_str().to_owned()),
            PreparedArg::AssetPath(id) => ExpandedArg::AssetPath(id.clone()),
            PreparedArg::BundlePath(path) => ExpandedArg::BundlePath(path.clone()),
        })
        .collect();
    let environment = command
        .environment
        .iter()
        .map(|(name, value)| {
            let value = match value {
                EnvironmentValue::Literal(value) => value.clone(),
                EnvironmentValue::ScratchPath(suffix) => check_root(scratch, suffix),
                EnvironmentValue::AssetPath(id) => assets
                    .get(id.as_str())
                    .map(|asset| asset.display_path.clone())
                    .ok_or_else(|| {
                        HealthError::Preparation(format!(
                            "command environment names absent asset `{id}`"
                        ))
                    })?,
            };
            Ok((name.clone(), value))
        })
        .collect::<Result<_, HealthError>>()?;
    Ok(ExpandedCommand {
        step: command.step,
        executable: executable.clone(),
        argv,
        environment,
    })
}

fn validate_stream_bound(
    evidence: &StreamEvidence,
    cap: u64,
    name: &str,
) -> Result<(), HealthError> {
    let retained = evidence.head.len() as u64 + evidence.tail.len() as u64;
    if retained > cap || evidence.truncated != (evidence.total_bytes > cap) {
        return Err(HealthError::Execution(format!(
            "backend returned inconsistent bounded {name} evidence"
        )));
    }
    Ok(())
}

fn check_root(base: &str, relative: &str) -> String {
    if relative == "." || relative.is_empty() {
        base.to_owned()
    } else {
        Path::new(base).join(relative).display().to_string()
    }
}
