//! MCP `lifecycle_run` — the strict hosted execution of one default
//! lifecycle phase (R7.4 A15c3).
//!
//! The grammar is one closed member: `phase`. Everything else about the
//! run — the chain, the identity, the lease, the ports, the policy — is
//! decided by the shared A15b command cell and this crate's context, so
//! no caller can forge a chain, reread a manifest, move configuration
//! ahead of the lease, or smuggle an offline/resume/provider spelling.
//!
//! The outcome funnel mirrors the CLI's byte-for-byte REPORT semantics
//! (the generated `cli-lifecycle-report` document is surface-identical)
//! while differing deliberately in the two places a surface owns: MCP
//! ignores `emit_report` (CLI historical silence does not apply — every
//! executed failure returns its generated `ok:false` root), and the text
//! channel carries MCP-native guidance (`lifecycle_tasks`, then this
//! tool again) instead of terminal rendering.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#lifecycle");

mod ports;

use serde::Deserialize;
use serde_json::{Value, json};
use specmark::{cell, spec};
use vibe_lifecycle::{DEFAULT_PHASES, Phase};
use vibe_orchestrator::failure::Measurement;
use vibe_orchestrator::trace::CommandExit;
use vibe_orchestrator::trace::finalize;
use vibe_orchestrator::values::{LifecycleValues, contribution_report};
use vibe_orchestrator::{
    DefaultLifecyclePorts, DefaultLifecycleRequest, InstallInputs, PhaseOutcome,
    lease_default_lifecycle, prepare_default_lifecycle,
};
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;

use super::{McpTool, ToolOutput};
use crate::{ServerContext, ToolDescriptor, ToolError};

use ports::{
    HostedConfirmGate, HostedInstallObserver, HostedPackageSourceFactory,
    HostedRegistryEnvironment, HostedRunObserver,
};

/// MCP `lifecycle_run`: execute one default-lifecycle phase for this
/// project as the hosted agent — validate → … → `phase`, real install
/// barrier included — and return the SAME generated lifecycle report the
/// CLI's `--json` mode produces.
///
/// ```
/// use vibe_mcp::tools::{LifecycleRunMcpTool, McpTool};
///
/// let descriptor = LifecycleRunMcpTool.descriptor();
/// assert_eq!(descriptor.name, "lifecycle_run");
/// // The enum is the canonical phase table, not a second handwritten one.
/// let phases = descriptor.input_schema["properties"]["phase"]["enum"]
///     .as_array().unwrap();
/// assert_eq!(phases.len(), 9);
/// assert_eq!(phases[0], "validate");
/// assert_eq!(phases[8], "deploy");
/// ```
#[cell(seam = "McpTool", variant = "lifecycle_run")]
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#AGENT-HANDSHAKE")]
pub struct LifecycleRunMcpTool;

/// Runtime argument authority: exactly `phase`, nothing else, decoded
/// BEFORE anything executes.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LifecycleRunArgs {
    phase: String,
}

impl McpTool for LifecycleRunMcpTool {
    fn descriptor(&self) -> ToolDescriptor {
        // The enum is derived from DEFAULT_PHASES — the same table the
        // chain derivation walks — so a future phase lands in both or
        // neither, never as a drift between grammar and execution.
        let phases: Vec<&str> = DEFAULT_PHASES.iter().map(|phase| phase.as_str()).collect();
        ToolDescriptor {
            name: "lifecycle_run".into(),
            description:
                "Execute one default-lifecycle phase for this project as the hosted agent: the exact same algorithmic run (validate up to the named phase, prerequisite install included) the CLI performs, returning the same generated lifecycle report as the CLI's --json mode. When an agent contribution parks, the report carries a delegation with the exact task document; read it with lifecycle_tasks, do the work, then call lifecycle_run with the SAME phase to resume — a park is not an error. Takes exactly one argument, `phase`; there is deliberately no path, force, offline, resume, clean or provider option. Offline and install policy are fixed when this MCP server starts, with no per-call override.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "phase": {
                        "type": "string",
                        "enum": phases,
                        "description": "The last default-lifecycle phase to execute (validate, install, generate, build, test, create, verify, package or deploy)."
                    }
                },
                "required": ["phase"],
                "additionalProperties": false
            }),
        }
    }

    fn run(&self, args: &Value, ctx: &ServerContext) -> Result<ToolOutput, ToolError> {
        let phase = parse_phase(args)?;
        // Stage one: the lease. Any refusal here — a busy workspace, no
        // discoverable project — is PRE-EXECUTION: nothing ran, no
        // structured root exists, and the full typed chain travels in the
        // text channel verbatim.
        let leased = lease_default_lifecycle(&ctx.project_root)
            .map_err(|error| ToolError::PreExecution(format!("{error:#}")))?;
        // Stage two: snapshot, identity, DERIVED chain. The tool decides
        // exactly the posture facts the grammar owns: agent mode, assume
        // yes, no force, no trace flag — and nothing else.
        let prepared = prepare_default_lifecycle(
            leased,
            DefaultLifecycleRequest {
                requested: phase,
                force: false,
                agent_mode: RunAgentMode::Agent,
                assume_yes: true,
                trace_flag: false,
                install_inputs: InstallInputs::default(),
                policy: ctx.lifecycle_policy,
            },
        )
        .map_err(|error| ToolError::PreExecution(format!("{error:#}")))?;
        // The prepared metadata is the projection a Slot/InstallBarrier
        // failure borrows for its report; the hosted backend is built from
        // the leased workspace root ONLY — the paid manifest accessor is
        // never read on this surface (no [llm] crosses).
        let metadata = prepared.metadata().clone();
        let agent = std::sync::Arc::new(vibe_lifecycle::agent::HostedAgentBackend::new(
            prepared.workspace_root().to_path_buf(),
        ));
        // The lease owner is held until the ToolOutput exists, so the
        // workspace stays owned through trace finalisation and the report
        // construction that follow the executed region.
        let lease_owner = prepared.retain_lease();
        let now = trace_clock;
        let preparation = prepared.prepare_trace(&now);
        let environment = HostedRegistryEnvironment::new(ctx);
        let sources = HostedPackageSourceFactory;
        let confirm_gate = HostedConfirmGate;
        let install_observer = HostedInstallObserver;
        let observer = HostedRunObserver;
        let outcome = prepared.run(
            DefaultLifecyclePorts {
                observer: &observer,
                install_observer: &install_observer,
                confirm_gate: &confirm_gate,
                sources: &sources,
                environment: &environment,
                manifest_mutation: &vibe_orchestrator::ports::NoManifestMutation,
                agent,
            },
            preparation.recorder(),
        );
        // The funnel: one exit, one finalize, one report. MCP IGNORES
        // `emit_report` — CLI historical silence does not travel — and
        // folds the owner notices exactly when the CLI's JSON arm would.
        let exit = match outcome {
            PhaseOutcome::Completed(values) => CommandExit::Success(values),
            PhaseOutcome::Parked(values) => CommandExit::Parked(values),
            PhaseOutcome::Failed {
                measurement,
                original,
                emit_machine_failure,
            } => CommandExit::Failed {
                report: failure_values(measurement, &metadata),
                original_error: original,
                emit_when_trace_disabled: emit_machine_failure,
            },
        };
        let finalized = finalize(preparation, exit, &now);
        let mut values = finalized.report;
        absorb_owner_notices(
            &mut values,
            finalized.trace.is_some(),
            finalized.notices.iter().cloned(),
        );
        let report = values.into_report(finalized.trace.clone());
        let structured = serde_json::to_value(&report).map_err(|error| {
            ToolError::Internal(format!("serialising the lifecycle report: {error}"))
        })?;
        let output = match finalized.original_error {
            None => {
                let parked = report.delegation.is_some();
                let text = if parked {
                    format!(
                        "parked for you — read the task with `lifecycle_tasks`, do the work it \
                         describes, then call `lifecycle_run` with phase `{}` again",
                        report.requested
                    )
                } else {
                    format!(
                        "phase `{}` completed ({} step(s), {} contribution(s))",
                        report.requested,
                        report.steps.len(),
                        report.contributions.len()
                    )
                };
                ToolOutput::executed(structured, text)
            }
            Some(original) => ToolOutput::executed_failure(structured, format!("{original:#}")),
        };
        drop(lease_owner);
        Ok(output)
    }
}

/// The trace clock. The generated shared timestamp is a chrono UTC instant;
/// use the same real clock as the CLI rather than parsing the identity's
/// display string or inventing a fixed epoch.
fn trace_clock() -> vibe_wire::generated::shared::Timestamp {
    chrono::Utc::now()
}

/// Match the CLI JSON notice law: without a trace member, the lifecycle root
/// is the only structured carrier; with one, its warnings already contain the
/// owner notices and adding them to the root would duplicate them.
fn absorb_owner_notices(
    values: &mut LifecycleValues,
    trace_present: bool,
    notices: impl IntoIterator<Item = String>,
) {
    if !trace_present {
        values.notices.extend(notices);
    }
}

/// The strict argument parse: only `{ "phase": "<canonical>" }` passes.
/// Omitted/null `arguments` normalise to `{}` and then fail
/// missing-required; scalars, arrays, wrong types, unknown members
/// (including `path` beside a valid phase), uppercase spellings and
/// `clean` all refuse BEFORE the lease, the lock, the `.vibe` tree, any
/// state or the outbox is touched.
fn parse_phase(args: &Value) -> Result<Phase, ToolError> {
    let normalized = match args {
        Value::Null => Value::Object(serde_json::Map::new()),
        Value::Object(_) => args.clone(),
        other => {
            return Err(ToolError::InvalidArguments(format!(
                "`lifecycle_run` takes exactly an object with one `phase` member — got {other}"
            )));
        }
    };
    let decoded: LifecycleRunArgs = serde_json::from_value(normalized).map_err(|error| {
        ToolError::InvalidArguments(format!(
            "`lifecycle_run` takes exactly one `phase` member: {error}"
        ))
    })?;
    decoded.phase.parse().map_err(|error| {
        ToolError::InvalidArguments(format!(
            "`lifecycle_run` phase `{}` is not a default-lifecycle phase — {error}. `clean` is \
             deliberately not exposed over MCP; call the CLI for a clean chain",
            decoded.phase
        ))
    })
}

/// The failure-arm projection: a Lifecycle measurement reports its own
/// requested/chain/stopped phase; a Slot or InstallBarrier measurement
/// reports the PREPARED run's real requested/chain, stopped at `install`,
/// with its slot rows converted to contributions — the same family law
/// the CLI's phase-verb projection applies.
fn failure_values(
    measurement: Measurement,
    metadata: &vibe_lifecycle::RunMetadata,
) -> LifecycleValues {
    match measurement {
        Measurement::Lifecycle {
            rows,
            stopped_phase,
            requested,
            chain,
        } => LifecycleValues::failed(&requested, chain, &stopped_phase, rows),
        Measurement::Slot { reports, .. } | Measurement::InstallBarrier { reports, .. } => {
            LifecycleValues::failed(
                &metadata.requested,
                metadata.chain.clone(),
                Phase::Install.as_str(),
                reports.into_iter().map(contribution_report).collect(),
            )
        }
    }
}

#[cfg(test)]
#[path = "lifecycle_run/tests.rs"]
mod tests;
