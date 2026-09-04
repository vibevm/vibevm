//! Generated epoch-1 wire projection for a fully prepared health plan.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-C");

use sha2::{Digest, Sha256};
use vibe_wire::generated::scrape::e1::plan as w;

use super::model as d;

pub fn to_wire(value: &d::PreparedHealth) -> Result<Vec<w::Healthcheck>, d::HealthError> {
    value.checks.iter().map(check).collect()
}

pub fn baseline(value: d::BaselinePolicy) -> w::HealthBaseline {
    match value {
        d::BaselinePolicy::Strict => w::HealthBaseline::Strict,
        d::BaselinePolicy::NoRegression => w::HealthBaseline::NoRegression,
    }
}

pub fn limits(value: &d::PreparedHealth) -> Result<w::HealthLimits, d::HealthError> {
    Ok(w::HealthLimits {
        max_stdout_bytes: value.max_stdout_bytes.to_string(),
        max_stderr_bytes: value.max_stderr_bytes.to_string(),
        max_result_bytes: value.max_result_bytes.to_string(),
        termination_grace_seconds: u32_value(value.termination_grace_seconds, "termination grace")?,
    })
}

fn check(value: &d::PreparedHealthcheck) -> Result<w::Healthcheck, d::HealthError> {
    let id = value.id.clone();
    let root = value.root.clone();
    let applicability = applicability(&value.applicability);
    let tests = value.tests.map(test_disposition);
    let assets = value.assets.iter().map(asset).collect();
    let commands = value.commands.iter().map(command).collect();
    let effects = effects(value);
    let sandbox = sandbox(value.sandbox);
    let timeout_seconds = u32_value(value.timeout_seconds, "health timeout")?;
    let assurance_reductions = value.assurance_reductions.clone();
    Ok(match value.kind {
        d::HealthcheckKind::Cargo => w::Healthcheck::Cargo(Box::new(w::HealthcheckCargo {
            id,
            root,
            applicability,
            tests,
            assets,
            commands,
            effects,
            sandbox,
            timeout_seconds,
            assurance_reductions,
        })),
        d::HealthcheckKind::Npm => w::Healthcheck::Npm(Box::new(w::HealthcheckNpm {
            id,
            root,
            applicability,
            tests,
            assets,
            commands,
            effects,
            sandbox,
            timeout_seconds,
            assurance_reductions,
        })),
        d::HealthcheckKind::Maven => w::Healthcheck::Maven(Box::new(w::HealthcheckMaven {
            id,
            root,
            applicability,
            tests,
            assets,
            commands,
            effects,
            sandbox,
            timeout_seconds,
            assurance_reductions,
        })),
        d::HealthcheckKind::PythonPip => {
            w::Healthcheck::PythonPip(Box::new(w::HealthcheckPythonPip {
                id,
                root,
                applicability,
                tests,
                assets,
                commands,
                effects,
                sandbox,
                timeout_seconds,
                assurance_reductions,
            }))
        }
        d::HealthcheckKind::Custom => {
            let bundle = value.custom_bundle.as_ref().ok_or_else(|| {
                d::HealthError::Preparation(format!(
                    "prepared custom healthcheck `{}` has no snapshot bundle",
                    value.id
                ))
            })?;
            let snapshot = bundle
                .entries
                .iter()
                .filter(|entry| entry.kind == d::BundleEntryKind::File)
                .map(|entry| {
                    Ok(w::SnapshotFileIdentity {
                        path: entry.path.clone(),
                        sha256: entry.sha256.clone().ok_or_else(|| {
                            d::HealthError::Preparation("snapshot file has no digest".to_owned())
                        })?,
                        bytes: entry
                            .bytes
                            .ok_or_else(|| {
                                d::HealthError::Preparation(
                                    "snapshot file has no byte count".to_owned(),
                                )
                            })?
                            .to_string(),
                        mode: entry.mode.unwrap_or(0),
                    })
                })
                .collect::<Result<Vec<_>, d::HealthError>>()?;
            let protocol = match value.protocol {
                d::ResultProtocol::ExitCode => w::HealthcheckCustomProtocol::ExitCode,
                d::ResultProtocol::VibeHealthJsonV1 => {
                    w::HealthcheckCustomProtocol::VibeHealthJsonV1
                }
                d::ResultProtocol::BuiltIn => {
                    return Err(d::HealthError::Preparation(format!(
                        "custom healthcheck `{}` has built-in protocol",
                        value.id
                    )));
                }
            };
            w::Healthcheck::Custom(Box::new(w::HealthcheckCustom {
                id,
                root,
                applicability,
                tests,
                assets,
                commands,
                effects,
                sandbox,
                timeout_seconds,
                assurance_reductions,
                source: bundle.source.clone(),
                snapshot,
                protocol,
            }))
        }
    })
}

fn asset(value: &d::AssetIdentity) -> w::AssetIdentity {
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
        bytes: value.bytes.to_string(),
        mode: value.mode,
        platform_identity: value.platform_identity.clone(),
        version: value.version.clone(),
        version_kind: match value.version_kind {
            d::VersionKind::Content => w::AssetIdentityVersionKind::Content,
            d::VersionKind::Probe => w::AssetIdentityVersionKind::Probe,
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

fn command(value: &d::PreparedCommand) -> w::PreparedCommand {
    w::PreparedCommand {
        step: match value.step {
            d::CommandStep::Install => w::CommandStep::Install,
            d::CommandStep::Build => w::CommandStep::Build,
            d::CommandStep::Test => w::CommandStep::Test,
            d::CommandStep::Verify => w::CommandStep::Verify,
        },
        executable_asset_id: value.executable_asset_id.clone(),
        argv: value.argv.iter().map(argument).collect(),
        environment: value
            .environment
            .iter()
            .map(|(name, value)| {
                let template = environment_template(value);
                w::EnvironmentIdentity {
                    name: name.clone(),
                    value_sha256: digest("vibe-scrape-env-e1", template.as_bytes()),
                    value_template: template,
                }
            })
            .collect(),
        accepted_exit_codes: value.accepted_exit_codes.clone(),
    }
}

fn argument(value: &d::PreparedArg) -> w::PreparedArg {
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

fn applicability(value: &d::Applicability) -> w::Applicability {
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

fn test_disposition(value: d::TestDisposition) -> w::TestDisposition {
    match value {
        d::TestDisposition::SkippedByContract => w::TestDisposition::SkippedByContract,
        d::TestDisposition::SkippedNotPresent => w::TestDisposition::SkippedNotPresent,
        d::TestDisposition::RunIfPresent => w::TestDisposition::RunIfPresent,
        d::TestDisposition::RunRequired => w::TestDisposition::RunRequired,
    }
}

fn effects(value: &d::PreparedHealthcheck) -> w::EffectPlan {
    w::EffectPlan {
        reads: value.effects.reads.clone(),
        writes: value.effects.writes.clone(),
        spawn: value.effects.spawn,
        network: network(value.network),
    }
}

fn sandbox(value: d::SandboxRequirement) -> w::SandboxRequirement {
    w::SandboxRequirement {
        exact_executable_identity: value.exact_executable_identity,
        filesystem_isolation: value.filesystem_isolation,
        read_policy_enforcement: value.read_policy_enforcement,
        process_tree_containment: value.process_tree_containment,
        graceful_termination: value.graceful_termination,
        termination_mode: match value.termination_mode {
            d::TerminationMode::ForcedTree => w::SandboxRequirementTerminationMode::ForcedTree,
            d::TerminationMode::GracefulThenForced => {
                w::SandboxRequirementTerminationMode::GracefulThenForced
            }
        },
        spawn_prevention: value.spawn_prevention,
        network_deny: value.network_deny,
        bounded_output: value.bounded_output,
        atomic_result: value.atomic_result,
        bundle_materialization: value.bundle_materialization,
    }
}

fn network(value: d::NetworkMode) -> w::Network {
    match value {
        d::NetworkMode::Deny => w::Network::Deny,
        d::NetworkMode::ToolOffline => w::Network::ToolOffline,
        d::NetworkMode::Inherit => w::Network::Inherit,
    }
}

fn environment_template(value: &d::EnvironmentValue) -> String {
    match value {
        d::EnvironmentValue::Literal(value) => value.clone(),
        d::EnvironmentValue::ScratchPath(value) => format!("{{scratch}}/{value}"),
        d::EnvironmentValue::AssetPath(value) => format!("{{asset:{value}}}"),
    }
}

fn digest(domain: &str, bytes: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(domain.as_bytes());
    hash.update(b"\0");
    hash.update(bytes);
    format!("sha256:{:x}", hash.finalize())
}

fn u32_value(value: u64, field: &str) -> Result<u32, d::HealthError> {
    u32::try_from(value).map_err(|_| {
        d::HealthError::Preparation(format!("{field} exceeds the epoch-1 wire uint32"))
    })
}
