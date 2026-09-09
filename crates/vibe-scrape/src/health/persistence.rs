//! Generated epoch-2 persistence for the restart-executable health plan.

use vibe_wire::generated::scrape::e2::prepared_health_snapshot as w;

use super::model as d;

pub(crate) fn snapshot_bytes(value: &d::PreparedHealth) -> Result<Vec<u8>, d::HealthError> {
    serde_json::to_vec(&to_wire(value))
        .map_err(|error| d::HealthError::Preparation(format!("encoding health snapshot: {error}")))
}

pub(crate) fn snapshot_from_bytes(bytes: &[u8]) -> Result<d::PreparedHealth, d::HealthError> {
    let wire = serde_json::from_slice::<w::PreparedHealthSnapshot>(bytes).map_err(|error| {
        let is_prepublic_shape = serde_json::from_slice::<serde_json::Value>(bytes)
            .ok()
            .is_some_and(|value| {
                value
                    .as_object()
                    .is_some_and(|object| !object.contains_key("schema"))
            });
        if is_prepublic_shape {
            d::HealthError::Preparation(
                "prepared health snapshot is not schema 2; pre-public schema-1 snapshots cannot be recovered—restart the scrape transaction"
                    .to_owned(),
            )
        } else {
            d::HealthError::Preparation(format!("decoding schema-2 health snapshot: {error}"))
        }
    })?;
    if wire.schema != 2 {
        return Err(d::HealthError::Preparation(format!(
            "unsupported prepared health snapshot schema {}; restart the scrape transaction",
            wire.schema
        )));
    }
    Ok(from_wire(wire))
}

fn to_wire(value: &d::PreparedHealth) -> w::PreparedHealthSnapshot {
    w::PreparedHealthSnapshot {
        schema: 2,
        plan_id: value.plan_id.clone(),
        baseline: baseline_to_wire(value.baseline),
        max_stdout_bytes: value.max_stdout_bytes,
        max_stderr_bytes: value.max_stderr_bytes,
        max_result_bytes: value.max_result_bytes,
        termination_grace_seconds: value.termination_grace_seconds,
        checks: value.checks.iter().map(check_to_wire).collect(),
        blockers: value
            .blockers
            .iter()
            .map(|blocker| w::HealthBlocker {
                code: blocker.code.clone(),
                check_id: blocker.check_id.clone(),
                message: blocker.message.clone(),
            })
            .collect(),
    }
}

fn from_wire(value: w::PreparedHealthSnapshot) -> d::PreparedHealth {
    d::PreparedHealth {
        plan_id: value.plan_id,
        baseline: baseline_from_wire(value.baseline),
        max_stdout_bytes: value.max_stdout_bytes,
        max_stderr_bytes: value.max_stderr_bytes,
        max_result_bytes: value.max_result_bytes,
        termination_grace_seconds: value.termination_grace_seconds,
        checks: value.checks.into_iter().map(check_from_wire).collect(),
        blockers: value
            .blockers
            .into_iter()
            .map(|blocker| d::HealthBlocker {
                code: blocker.code,
                check_id: blocker.check_id,
                message: blocker.message,
            })
            .collect(),
    }
}

fn check_to_wire(value: &d::PreparedHealthcheck) -> w::PreparedHealthcheck {
    w::PreparedHealthcheck {
        id: value.id.clone(),
        kind: match value.kind {
            d::HealthcheckKind::Cargo => w::HealthcheckKind::Cargo,
            d::HealthcheckKind::Npm => w::HealthcheckKind::Npm,
            d::HealthcheckKind::Maven => w::HealthcheckKind::Maven,
            d::HealthcheckKind::PythonPip => w::HealthcheckKind::PythonPip,
            d::HealthcheckKind::Custom => w::HealthcheckKind::Custom,
        },
        root: value.root.clone(),
        applicability: applicability_to_wire(&value.applicability),
        tests: value.tests.map(test_to_wire),
        network: network_to_wire(value.network),
        assets: value.assets.iter().map(asset_to_wire).collect(),
        commands: value.commands.iter().map(command_to_wire).collect(),
        effects: w::EffectPlan {
            reads: value.effects.reads.clone(),
            writes: value.effects.writes.clone(),
            spawn: value.effects.spawn,
        },
        sandbox: sandbox_to_wire(value.sandbox),
        protocol: protocol_to_wire(&value.protocol),
        custom_bundle: value.custom_bundle.as_ref().map(bundle_to_wire),
        assurance_reductions: value.assurance_reductions.clone(),
        timeout_seconds: value.timeout_seconds,
    }
}

fn check_from_wire(value: w::PreparedHealthcheck) -> d::PreparedHealthcheck {
    d::PreparedHealthcheck {
        id: value.id,
        kind: match value.kind {
            w::HealthcheckKind::Cargo => d::HealthcheckKind::Cargo,
            w::HealthcheckKind::Npm => d::HealthcheckKind::Npm,
            w::HealthcheckKind::Maven => d::HealthcheckKind::Maven,
            w::HealthcheckKind::PythonPip => d::HealthcheckKind::PythonPip,
            w::HealthcheckKind::Custom => d::HealthcheckKind::Custom,
        },
        root: value.root,
        applicability: applicability_from_wire(value.applicability),
        tests: value.tests.map(test_from_wire),
        network: network_from_wire(value.network),
        assets: value.assets.into_iter().map(asset_from_wire).collect(),
        commands: value.commands.into_iter().map(command_from_wire).collect(),
        effects: d::EffectPlan {
            reads: value.effects.reads,
            writes: value.effects.writes,
            spawn: value.effects.spawn,
        },
        sandbox: sandbox_from_wire(value.sandbox),
        protocol: protocol_from_wire(value.protocol),
        custom_bundle: value.custom_bundle.map(bundle_from_wire),
        assurance_reductions: value.assurance_reductions,
        timeout_seconds: value.timeout_seconds,
    }
}

fn asset_to_wire(value: &d::AssetIdentity) -> w::AssetIdentity {
    w::AssetIdentity {
        id: value.id.clone(),
        role: match value.role {
            d::AssetRole::Cargo => w::AssetRole::Cargo,
            d::AssetRole::Rustc => w::AssetRole::Rustc,
            d::AssetRole::Rustdoc => w::AssetRole::Rustdoc,
            d::AssetRole::Node => w::AssetRole::Node,
            d::AssetRole::NpmCli => w::AssetRole::NpmCli,
            d::AssetRole::MavenLauncher => w::AssetRole::MavenLauncher,
            d::AssetRole::Python => w::AssetRole::Python,
            d::AssetRole::CustomInterpreter => w::AssetRole::CustomInterpreter,
            d::AssetRole::CustomNative => w::AssetRole::CustomNative,
        },
        display_path: value.display_path.clone(),
        sha256: value.sha256.clone(),
        bytes: value.bytes,
        mode: value.mode,
        platform_identity: value.platform_identity.clone(),
        version: value.version.clone(),
        version_kind: match value.version_kind {
            d::VersionKind::Content => w::VersionKind::Content,
            d::VersionKind::Probe => w::VersionKind::Probe,
        },
        source: match &value.source {
            d::AssetSource::Resolved => {
                w::AssetSource::Resolved(Box::new(w::AssetSourceResolved {}))
            }
            d::AssetSource::Bundle { path } => {
                w::AssetSource::Bundle(Box::new(w::AssetSourceBundle { path: path.clone() }))
            }
        },
    }
}

fn asset_from_wire(value: w::AssetIdentity) -> d::AssetIdentity {
    d::AssetIdentity {
        id: value.id,
        role: match value.role {
            w::AssetRole::Cargo => d::AssetRole::Cargo,
            w::AssetRole::Rustc => d::AssetRole::Rustc,
            w::AssetRole::Rustdoc => d::AssetRole::Rustdoc,
            w::AssetRole::Node => d::AssetRole::Node,
            w::AssetRole::NpmCli => d::AssetRole::NpmCli,
            w::AssetRole::MavenLauncher => d::AssetRole::MavenLauncher,
            w::AssetRole::Python => d::AssetRole::Python,
            w::AssetRole::CustomInterpreter => d::AssetRole::CustomInterpreter,
            w::AssetRole::CustomNative => d::AssetRole::CustomNative,
        },
        display_path: value.display_path,
        sha256: value.sha256,
        bytes: value.bytes,
        mode: value.mode,
        platform_identity: value.platform_identity,
        version: value.version,
        version_kind: match value.version_kind {
            w::VersionKind::Content => d::VersionKind::Content,
            w::VersionKind::Probe => d::VersionKind::Probe,
        },
        source: match value.source {
            w::AssetSource::Resolved(_) => d::AssetSource::Resolved,
            w::AssetSource::Bundle(bundle) => d::AssetSource::Bundle { path: bundle.path },
        },
        live_identity: None,
    }
}

fn command_to_wire(value: &d::PreparedCommand) -> w::PreparedCommand {
    w::PreparedCommand {
        step: step_to_wire(value.step),
        executable_asset_id: value.executable_asset_id.clone(),
        argv: value.argv.iter().map(argument_to_wire).collect(),
        environment: value
            .environment
            .iter()
            .map(|(name, value)| (name.clone(), environment_to_wire(value)))
            .collect(),
        accepted_exit_codes: value.accepted_exit_codes.clone(),
    }
}

fn command_from_wire(value: w::PreparedCommand) -> d::PreparedCommand {
    d::PreparedCommand {
        step: step_from_wire(value.step),
        executable_asset_id: value.executable_asset_id,
        argv: value.argv.into_iter().map(argument_from_wire).collect(),
        environment: value
            .environment
            .into_iter()
            .map(|(name, value)| (name, environment_from_wire(value)))
            .collect(),
        accepted_exit_codes: value.accepted_exit_codes,
    }
}

fn argument_to_wire(value: &d::PreparedArg) -> w::PreparedArg {
    match value {
        d::PreparedArg::Literal(value) => {
            w::PreparedArg::Literal(Box::new(w::PreparedArgLiteral {
                value: value.clone(),
            }))
        }
        d::PreparedArg::Root => w::PreparedArg::Root(Box::new(w::PreparedArgRoot {})),
        d::PreparedArg::Scratch => w::PreparedArg::Scratch(Box::new(w::PreparedArgScratch {})),
        d::PreparedArg::Result => w::PreparedArg::Result(Box::new(w::PreparedArgResult {})),
        d::PreparedArg::Phase => w::PreparedArg::Phase(Box::new(w::PreparedArgPhase {})),
        d::PreparedArg::AssetPath(value) => {
            w::PreparedArg::AssetPath(Box::new(w::PreparedArgAssetPath {
                value: value.clone(),
            }))
        }
        d::PreparedArg::BundlePath(value) => {
            w::PreparedArg::BundlePath(Box::new(w::PreparedArgBundlePath {
                value: value.clone(),
            }))
        }
    }
}

fn argument_from_wire(value: w::PreparedArg) -> d::PreparedArg {
    match value {
        w::PreparedArg::Literal(value) => d::PreparedArg::Literal(value.value),
        w::PreparedArg::Root(_) => d::PreparedArg::Root,
        w::PreparedArg::Scratch(_) => d::PreparedArg::Scratch,
        w::PreparedArg::Result(_) => d::PreparedArg::Result,
        w::PreparedArg::Phase(_) => d::PreparedArg::Phase,
        w::PreparedArg::AssetPath(value) => d::PreparedArg::AssetPath(value.value),
        w::PreparedArg::BundlePath(value) => d::PreparedArg::BundlePath(value.value),
    }
}

fn environment_to_wire(value: &d::EnvironmentValue) -> w::EnvironmentValue {
    match value {
        d::EnvironmentValue::Literal(value) => {
            w::EnvironmentValue::Literal(Box::new(w::EnvironmentValueLiteral {
                value: value.clone(),
            }))
        }
        d::EnvironmentValue::ScratchPath(value) => {
            w::EnvironmentValue::ScratchPath(Box::new(w::EnvironmentValueScratchPath {
                value: value.clone(),
            }))
        }
        d::EnvironmentValue::AssetPath(value) => {
            w::EnvironmentValue::AssetPath(Box::new(w::EnvironmentValueAssetPath {
                value: value.clone(),
            }))
        }
    }
}

fn environment_from_wire(value: w::EnvironmentValue) -> d::EnvironmentValue {
    match value {
        w::EnvironmentValue::Literal(value) => d::EnvironmentValue::Literal(value.value),
        w::EnvironmentValue::ScratchPath(value) => d::EnvironmentValue::ScratchPath(value.value),
        w::EnvironmentValue::AssetPath(value) => d::EnvironmentValue::AssetPath(value.value),
    }
}

fn bundle_to_wire(value: &d::CustomBundle) -> w::CustomBundle {
    w::CustomBundle {
        sha256: value.sha256.clone(),
        source: value.source.clone(),
        entries: value
            .entries
            .iter()
            .map(|entry| w::BundleEntry {
                path: entry.path.clone(),
                kind: match entry.kind {
                    d::BundleEntryKind::File => w::BundleEntryKind::File,
                    d::BundleEntryKind::Directory => w::BundleEntryKind::Directory,
                },
                sha256: entry.sha256.clone(),
                bytes: entry.bytes,
                mode: entry.mode,
            })
            .collect(),
    }
}

fn bundle_from_wire(value: w::CustomBundle) -> d::CustomBundle {
    d::CustomBundle {
        sha256: value.sha256,
        source: value.source,
        entries: value
            .entries
            .into_iter()
            .map(|entry| d::BundleEntry {
                path: entry.path,
                kind: match entry.kind {
                    w::BundleEntryKind::File => d::BundleEntryKind::File,
                    w::BundleEntryKind::Directory => d::BundleEntryKind::Directory,
                },
                sha256: entry.sha256,
                bytes: entry.bytes,
                mode: entry.mode,
                content: None,
            })
            .collect(),
    }
}

fn applicability_to_wire(value: &d::Applicability) -> w::Applicability {
    match value {
        d::Applicability::Applicable => {
            w::Applicability::Applicable(Box::new(w::ApplicabilityApplicable {}))
        }
        d::Applicability::SkippedWhenMissing { path } => {
            w::Applicability::SkippedWhenMissing(Box::new(w::ApplicabilitySkippedWhenMissing {
                path: path.clone(),
            }))
        }
    }
}

fn applicability_from_wire(value: w::Applicability) -> d::Applicability {
    match value {
        w::Applicability::Applicable(_) => d::Applicability::Applicable,
        w::Applicability::SkippedWhenMissing(value) => {
            d::Applicability::SkippedWhenMissing { path: value.path }
        }
    }
}

fn baseline_to_wire(value: d::BaselinePolicy) -> w::BaselinePolicy {
    match value {
        d::BaselinePolicy::Strict => w::BaselinePolicy::Strict,
        d::BaselinePolicy::NoRegression => w::BaselinePolicy::NoRegression,
    }
}

fn baseline_from_wire(value: w::BaselinePolicy) -> d::BaselinePolicy {
    match value {
        w::BaselinePolicy::Strict => d::BaselinePolicy::Strict,
        w::BaselinePolicy::NoRegression => d::BaselinePolicy::NoRegression,
    }
}

fn test_to_wire(value: d::TestDisposition) -> w::TestDisposition {
    match value {
        d::TestDisposition::SkippedByContract => w::TestDisposition::SkippedByContract,
        d::TestDisposition::SkippedNotPresent => w::TestDisposition::SkippedNotPresent,
        d::TestDisposition::RunIfPresent => w::TestDisposition::RunIfPresent,
        d::TestDisposition::RunRequired => w::TestDisposition::RunRequired,
    }
}

fn test_from_wire(value: w::TestDisposition) -> d::TestDisposition {
    match value {
        w::TestDisposition::SkippedByContract => d::TestDisposition::SkippedByContract,
        w::TestDisposition::SkippedNotPresent => d::TestDisposition::SkippedNotPresent,
        w::TestDisposition::RunIfPresent => d::TestDisposition::RunIfPresent,
        w::TestDisposition::RunRequired => d::TestDisposition::RunRequired,
    }
}

fn network_to_wire(value: d::NetworkMode) -> w::NetworkMode {
    match value {
        d::NetworkMode::Deny => w::NetworkMode::Deny,
        d::NetworkMode::ToolOffline => w::NetworkMode::ToolOffline,
        d::NetworkMode::Inherit => w::NetworkMode::Inherit,
    }
}

fn network_from_wire(value: w::NetworkMode) -> d::NetworkMode {
    match value {
        w::NetworkMode::Deny => d::NetworkMode::Deny,
        w::NetworkMode::ToolOffline => d::NetworkMode::ToolOffline,
        w::NetworkMode::Inherit => d::NetworkMode::Inherit,
    }
}

fn protocol_to_wire(value: &d::ResultProtocol) -> w::ResultProtocol {
    match value {
        d::ResultProtocol::BuiltIn => w::ResultProtocol::BuiltIn,
        d::ResultProtocol::ExitCode => w::ResultProtocol::ExitCode,
        d::ResultProtocol::VibeHealthJsonV1 => w::ResultProtocol::VibeHealthJsonV1,
    }
}

fn protocol_from_wire(value: w::ResultProtocol) -> d::ResultProtocol {
    match value {
        w::ResultProtocol::BuiltIn => d::ResultProtocol::BuiltIn,
        w::ResultProtocol::ExitCode => d::ResultProtocol::ExitCode,
        w::ResultProtocol::VibeHealthJsonV1 => d::ResultProtocol::VibeHealthJsonV1,
    }
}

fn step_to_wire(value: d::CommandStep) -> w::CommandStep {
    match value {
        d::CommandStep::Install => w::CommandStep::Install,
        d::CommandStep::Build => w::CommandStep::Build,
        d::CommandStep::Test => w::CommandStep::Test,
        d::CommandStep::Verify => w::CommandStep::Verify,
    }
}

fn step_from_wire(value: w::CommandStep) -> d::CommandStep {
    match value {
        w::CommandStep::Install => d::CommandStep::Install,
        w::CommandStep::Build => d::CommandStep::Build,
        w::CommandStep::Test => d::CommandStep::Test,
        w::CommandStep::Verify => d::CommandStep::Verify,
    }
}

fn sandbox_to_wire(value: d::SandboxRequirement) -> w::SandboxRequirement {
    w::SandboxRequirement {
        exact_executable_identity: value.exact_executable_identity,
        filesystem_isolation: value.filesystem_isolation,
        read_policy_enforcement: value.read_policy_enforcement,
        process_tree_containment: value.process_tree_containment,
        graceful_termination: value.graceful_termination,
        termination_mode: match value.termination_mode {
            d::TerminationMode::ForcedTree => w::TerminationMode::ForcedTree,
            d::TerminationMode::GracefulThenForced => w::TerminationMode::GracefulThenForced,
        },
        spawn_prevention: value.spawn_prevention,
        network_deny: value.network_deny,
        bounded_output: value.bounded_output,
        atomic_result: value.atomic_result,
        bundle_materialization: value.bundle_materialization,
    }
}

fn sandbox_from_wire(value: w::SandboxRequirement) -> d::SandboxRequirement {
    d::SandboxRequirement {
        exact_executable_identity: value.exact_executable_identity,
        filesystem_isolation: value.filesystem_isolation,
        read_policy_enforcement: value.read_policy_enforcement,
        process_tree_containment: value.process_tree_containment,
        graceful_termination: value.graceful_termination,
        termination_mode: match value.termination_mode {
            w::TerminationMode::ForcedTree => d::TerminationMode::ForcedTree,
            w::TerminationMode::GracefulThenForced => d::TerminationMode::GracefulThenForced,
        },
        spawn_prevention: value.spawn_prevention,
        network_deny: value.network_deny,
        bounded_output: value.bounded_output,
        atomic_result: value.atomic_result,
        bundle_materialization: value.bundle_materialization,
    }
}

#[cfg(test)]
#[path = "persistence/tests.rs"]
mod tests;
