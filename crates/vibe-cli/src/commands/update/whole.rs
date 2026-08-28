//! The whole-graph `vibe update`: the `vibe install` from-manifest path,
//! executed under THIS command's identity, recorder and report root.
//!
//! ## Why the install substrate never opens its own recorder here
//!
//! `execute_prepared` BORROWS `Option<&TraceRun>`. The owner is the update
//! boundary, so one run id spans the delegate's empty-world regeneration, its
//! fresh fast path and its ready apply — the same run a park suspends and a
//! flagless resume reopens.
//!
//! ## Why a measured slot failure keeps the INSTALL root
//!
//! The install substrate reports a slot failure in a `cli-install-report`, and
//! a hosting agent parses that. The root family is decided where the failure is
//! MEASURED — inside the ready apply, with the rows and progress still in hand
//! — and travels outward as a typed carrier. This boundary calls
//! [`compile_trace::classify`], which takes that carrier apart into exactly
//! what its site froze: the Install draft, the original error object, and the
//! emission policy that site has always had. It does not re-derive any of the
//! three, because by the time an error reaches here all that is left to infer
//! from is its Display text — which is how these two families drifted apart
//! once already.
//!
//! A carried Install failure therefore emits ONE install root and ZERO update
//! roots, in both trace modes; the only difference tracing makes is the
//! optional member on that one root.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use anyhow::Result;
use vibe_install::InstallProgress;
use vibe_workspace::compile_trace::TraceRun;

use crate::commands::compile_trace::{self, CommandExit, RegisteredReportDraft};
use crate::commands::install;
use crate::output;

use super::Execution;
use super::draft::{UpdateDraft, UpdateIdentity};
use super::inputs::install_args_from;

/// The one boundary: everything after `prepare` and before `finalize`.
///
/// No `?` and no `return Err` leaves this function — the inner `Result` is
/// classified into the typed exit instead.
pub(super) fn execute_after_open(
    ctx: &output::Context,
    execution: Execution,
    trace: Option<&TraceRun>,
) -> CommandExit<RegisteredReportDraft> {
    // Built from the same values the execution uses, before they move: a
    // failure draft that re-derived the root would be a second answer to
    // "which node did this command act on".
    let identity = UpdateIdentity::from_args(execution.selection.root(), &execution.args);
    match run(ctx, execution, trace) {
        Ok(draft) => classify_success(draft),
        // A carried Install failure keeps its own root, error and policy; a
        // generic update-stage failure takes this command's own root and the
        // historical silence such stages have always had.
        Err(error) => compile_trace::classify(absorb_resume_failure(error, &identity), || {
            RegisteredReportDraft::Update(Box::new(UpdateDraft::failed(
                &identity,
                0,
                Vec::new(),
                InstallProgress::default(),
                Vec::new(),
            )))
        }),
    }
}

/// Give a NEUTRAL resume failure THIS command's registered family.
///
/// A continuation the delegate serviced belongs to `vibe update`: it is an
/// update-stage failure, and its root is the update one. That is a different
/// fact from the Ready apply's already-carried `SlotFailed`, which the
/// substrate measured as install-shaped and which stays install-shaped —
/// `take_resume_failure` is exact, so the two cannot be confused.
///
/// The resolved count, the durable progress and the ordered rows all come from
/// the transport rather than from a default, and the emission policy is the
/// historical silence of a generic update-stage failure.
fn absorb_resume_failure(error: anyhow::Error, identity: &UpdateIdentity) -> anyhow::Error {
    match install::take_measured_failure(error) {
        Ok(install::MeasuredFailure {
            original,
            measurement:
                install::Measurement::Slot {
                    progress,
                    reports,
                    packages_resolved,
                },
            emit_machine_failure,
        }) => compile_trace::carry(
            RegisteredReportDraft::Update(Box::new(UpdateDraft::failed(
                identity,
                packages_resolved,
                Vec::new(),
                *progress,
                reports,
            ))),
            original,
            emit_machine_failure,
        ),
        // Everything else keeps the family its own site froze: an
        // `InstallBarrier` measurement is the substrate's install-shaped
        // barrier failure and stays install-shaped, and a LIFECYCLE
        // measurement is the post-durability stage's own failure.
        Ok(failure) => compile_trace::carry(
            crate::commands::lifecycle::registered_family(
                &identity.project_root,
                failure.measurement,
            ),
            failure.original,
            failure.emit_machine_failure,
        ),
        Err(error) => error,
    }
}

fn classify_success(draft: UpdateDraft) -> CommandExit<RegisteredReportDraft> {
    let parked = draft.delegation.is_some();
    let draft = RegisteredReportDraft::Update(Box::new(draft));
    if parked {
        CommandExit::Parked(draft)
    } else {
        CommandExit::Success(draft)
    }
}

fn run(
    ctx: &output::Context,
    execution: Execution,
    trace: Option<&TraceRun>,
) -> Result<UpdateDraft> {
    let Execution {
        args,
        embedded_root,
        offline,
        lease,
        user_config,
        selection,
        metadata,
    } = execution;
    let identity = UpdateIdentity::from_args(selection.root(), &args);
    let install_args = install_args_from(&args);
    let confirm_gate = install::CliConfirmGate::new(ctx, install_args.assume_yes);
    let install_observer = install::CliInstallObserver::new(ctx, None);
    let sources = install::CliPackageSourceFactory {
        args: &install_args,
    };
    let policy = install_args.policy(offline, &user_config);
    let environment = install::CliRegistryEnvironment::new(move || embedded_root);
    // A whole update owns no lifecycle stage of its own, and admits no
    // manifest-mutating flag: both are the NAMED no-ops.
    let mut world_stage = install::NoAfterDurableWorld;
    // From the root the lease already pinned and the bundle's own snapshot —
    // no locator call.
    let agent: std::sync::Arc<dyn vibe_lifecycle::AgentBackend> =
        std::sync::Arc::new(crate::commands::lifecycle::install_agent_backend_from(
            lease.root(),
            selection.parsed_ref(),
        ));
    let run = install::execute_prepared(
        install::InstallExecution {
            // The delegated resolver arguments carry `trace_compile: false`:
            // this command already owns the one recorder, and a second request
            // at this depth could only ever mean a second owner.
            args: install_args.inputs(),
            environment: &environment,
            policy,
            // The owner's ALREADY-resolved posture, handed in at the delegate's
            // root rung. `install_args_from` sets the delegate's own
            // `--offline` false, and `resolve_offline` is idempotent over a
            // value that already absorbed `VIBE_OFFLINE` and `[net].offline`,
            // so the substrate reaches exactly this boolean without loading a
            // second config.
            lease,
            manifest_mutation: &install::NoManifestMutation,
            selection,
            metadata,
            sources: &sources,
            confirm_gate: &confirm_gate,
            observer: &install_observer,
            agent,
            trace,
        },
        &mut world_stage,
    )?;
    if let Some(delegation) = run.parked.as_ref() {
        crate::commands::lifecycle::check_delegation(delegation)?;
    }
    // Success and park alike report the UPDATE root: the substrate did the
    // work, but the command is `vibe update`, and its handoff resumes with
    // `vibe update`. A whole update moves no version by itself, so it declares
    // no bumps.
    Ok(UpdateDraft::completed(
        &identity,
        run.packages_resolved,
        Vec::new(),
        run.progress,
        run.slot_reports,
        run.parked.as_ref(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::compile_trace::{CommandExit, classify};
    use crate::commands::install::{MeasuredFailure, Measurement};
    use vibe_wire::generated::update_report::UpdateReportScope;

    #[derive(Debug, thiserror::Error)]
    #[error("the resumed row refused")]
    struct Sentinel;

    fn row(point: &str, status: &str) -> vibe_install::SlotLifecycleReport {
        vibe_install::SlotLifecycleReport {
            key: format!("org.demo/tools#{point}"),
            point: point.into(),
            handler: "builtin".into(),
            provider: "org.demo/tools".into(),
            tier: "dependency".into(),
            status: status.into(),
            message: None,
            version: None,
            reference: "spec://org.demo/tools".into(),
            flagged: false,
            stdout: None,
            stderr: None,
            stdout_truncated: false,
            stderr_truncated: false,
            slot_target: None,
        }
    }

    fn identity() -> UpdateIdentity {
        UpdateIdentity {
            project_root: std::path::PathBuf::from("/p"),
            scope: UpdateReportScope::All,
            packages: Vec::new(),
        }
    }

    /// A whole update names the UPDATE family for a neutral resume failure —
    /// this is an update-stage failure, not the Ready apply's install-shaped
    /// `SlotFailed` — and it carries the measurement rather than a default.
    ///
    /// Dropping the transport at either install resume site makes the count
    /// zero, the progress default and the rows empty, on a run whose earlier
    /// row really succeeded.
    #[test]
    fn a_neutral_resume_failure_becomes_a_measured_update_root() {
        let carried = absorb_resume_failure(
            vibe_orchestrator::failure::carry(MeasuredFailure {
                original: anyhow::Error::new(Sentinel).context("finishing the parked slot run"),
                measurement: Measurement::Slot {
                    progress: Box::new(vibe_install::InstallProgress {
                        complete: true,
                        fresh: false,
                        materialised: vec!["vibedeps/org.demo.tools/0.2.0".into()],
                        skipped: Vec::new(),
                        pruned: Vec::new(),
                        nodes_regenerated: vec![".".into()],
                    }),
                    reports: vec![
                        row("slot:pre-install", "ok"),
                        row("slot:post-install", "fail"),
                    ],
                    packages_resolved: 7,
                },
                emit_machine_failure: false,
            }),
            &identity(),
        );
        let CommandExit::Failed {
            report,
            original_error,
            emit_when_trace_disabled,
        } = classify(carried, || panic!("the carrier decides, not the fallback"))
        else {
            panic!("a failure is a failure");
        };
        assert!(
            !emit_when_trace_disabled,
            "an update-stage failure has always been silent with tracing off",
        );
        assert!(original_error.downcast_ref::<Sentinel>().is_some());
        assert!(
            !vibe_orchestrator::failure::is_measured(&original_error),
            "the neutral wrapper never escapes to main",
        );
        let RegisteredReportDraft::Update(draft) = report else {
            panic!("an update-stage failure is update-shaped");
        };
        let built = draft.into_report(None);
        assert!(!built.ok);
        assert!(!built.complete);
        assert_eq!(built.packages_resolved, 7, "the carried count, not a zero");
        assert_eq!(
            built.materialised,
            ["vibedeps/org.demo.tools/0.2.0"],
            "and the carried progress, not a default",
        );
        let statuses: Vec<&str> = built
            .contributions
            .iter()
            .map(|row| row.status.as_str())
            .collect();
        assert_eq!(statuses, ["ok", "fail"], "both rows, in order");
    }

    /// A carried INSTALL failure — the Ready apply's `SlotFailed` — is not a
    /// resume failure and keeps its own family untouched.
    #[test]
    fn a_carried_install_slot_failure_is_left_alone() {
        let carried = compile_trace::carry(
            RegisteredReportDraft::Install(Box::new(
                crate::commands::install::InstallDraft::failed(
                    std::path::Path::new("/p"),
                    InstallProgress::default(),
                    Vec::new(),
                ),
            )),
            anyhow::Error::new(Sentinel),
            true,
        );
        let CommandExit::Failed {
            report,
            emit_when_trace_disabled,
            ..
        } = classify(absorb_resume_failure(carried, &identity()), || {
            panic!("the carrier decides")
        })
        else {
            panic!("a failure is a failure");
        };
        assert!(matches!(report, RegisteredReportDraft::Install(_)));
        assert!(emit_when_trace_disabled, "with its own emission policy");
    }
}
