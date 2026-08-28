//! The `vibe install` command boundary: prepare, execute under one borrowed
//! recorder, finalise, render.
//!
//! This is one of exactly TWO places in the binary that may call the command
//! core's `prepare`/`finalize` (the other is `lifecycle::execute`). Everything
//! below only ever borrows `Option<&TraceRun>`.
//!
//! ## Why preparation is its own cell
//!
//! Compile-trace activation needs the selected manifest, and the run identity
//! needs the activation, and the metadata needs the identity — while the
//! existing order (config, then identity, then manifest) is characterised, and
//! `vibe install --git` REWRITES the manifest a few lines later. Doing this
//! inline meant either moving the manifest read (changing which error a
//! malformed manifest produces and when a run directory is allocated) or
//! reading the file twice (racing the command's own rewrite).
//!
//! So preparation takes one snapshot, answers activation from it, selects one
//! identity, opens at most one recorder, and hands the snapshot on. The
//! execution below consumes it at the original boundary. `main` reads no
//! config, no manifest, no workspace and no clock.
//!
//! ## Why nothing may escape between `prepare` and `finalize`
//!
//! An open recorder holds the project's cooperative lock and leaves its index
//! `running` on disk. A `?` that jumped out between the two would release the
//! lock by dropping the handle while leaving a run that claims, forever, to be
//! in progress. So the whole executed region is one function returning
//! [`CommandExit`] — a value, not a `Result` — and every error inside it is
//! classified into that value.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE");

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use vibe_core::user_config::UserConfig;
use vibe_lifecycle::{LifecycleLease, RunMetadata};

use crate::cli::InstallArgs;
use crate::commands::compile_trace::{
    self, CommandExit, RegisteredReportDraft, TracePreparation, render_finalized,
};
use crate::output;

use super::{
    InstallDisposition, InstallDraft, InstallExecution, InstallRun, InstallRunContext,
    PreparedWorkspace, SelectedManifest, acquire_lease, execute_prepared, resolve_project_root,
};

/// Everything `vibe install` reads before it may compile.
///
/// One config load, one manifest snapshot, one workspace. Execution below
/// consumes exactly these — it re-reads none of them, so the tree the trace
/// was locked against is the tree the install works on.
struct PreparedInstall {
    project_root: PathBuf,
    /// The workspace mutation lease, acquired BEFORE the config/manifest/
    /// workspace reads below: the locator epoch discovers the root, the
    /// lease pins it, and only then may execution read the tree.
    lease: Arc<LifecycleLease>,
    user_config: UserConfig,
    metadata: RunMetadata,
    manifest: SelectedManifest,
    workspace: PreparedWorkspace,
    trace: TracePreparation,
}

/// Read once, select one identity, open at most one recorder.
fn prepare_direct_install(
    ctx: &output::Context,
    args: &InstallArgs,
    root_offline: bool,
) -> Result<PreparedInstall> {
    let project_root = resolve_project_root(&args.path)?;
    // The OUTERMOST lock, before the config, manifest, workspace and identity
    // reads below: everything execution consumes is a post-acquisition
    // snapshot. A contended workspace refuses here, typed, having allocated
    // nothing but the infrastructure lock file itself.
    let lease = acquire_lease(&project_root)?;
    // Before the identity, and before anything is allocated: a malformed user
    // config must still fail ahead of the run-directory allocation.
    let user_config = UserConfig::load().context("loading the user config")?;
    let offline = output::resolve_offline(root_offline || args.offline, user_config.net.offline);
    // The ONE read of this command's selected `vibe.toml`, and the ONE tree
    // built from it. A malformed manifest yields no workspace here and no
    // trace storage below, and its stored error is consumed at the boundary
    // that has always reported it — after the identity is selected.
    let manifest = SelectedManifest::read(&project_root);
    let workspace = manifest.prepare_workspace(&project_root);
    let requested = "install";
    let chain = vec!["validate".to_string(), "install".to_string()];
    let prelude = crate::commands::lifecycle::run_prelude(
        ctx,
        project_root,
        workspace,
        lease.clone(),
        requested,
        &chain,
        args.force,
        manifest.request(args.trace_compile),
    )
    .context("selecting the install lifecycle run identity")?;
    let metadata = RunMetadata {
        requested: requested.to_string(),
        chain,
        offline,
        assume_yes: args.assume_yes || ctx.is_unattended() || ctx.is_json(),
        agent_mode: ctx.agent_mode(),
        force: args.force,
        // The EFFECTIVE bit the selector computed — an adopted run's sticky
        // trace bit, not the raw request, so a resume cannot rewrite a traced
        // run's header back to untraced.
        trace_compile: prelude.identity.compile_trace,
        run_id: prelude.identity.run_id.clone(),
        started: prelude.identity.started.clone(),
        selected: prelude.selected.clone(),
    };
    let trace = prelude.prepare_trace(&now);
    Ok(PreparedInstall {
        project_root: prelude.project_root,
        lease: prelude.lease,
        user_config,
        metadata,
        manifest,
        workspace: prelude.workspace,
        trace,
    })
}

/// `vibe install`, end to end.
pub(crate) fn run(
    ctx: &output::Context,
    args: InstallArgs,
    embedded_root: Option<PathBuf>,
    root_offline: bool,
) -> Result<()> {
    let PreparedInstall {
        project_root,
        lease,
        user_config,
        metadata,
        manifest,
        workspace,
        trace,
    } = prepare_direct_install(ctx, &args, root_offline)?;
    // The boundary's OWNER share: the executed region consumes its own clones
    // into the run and the callback, while this one lives to the end of the
    // command — through the final report and the trace finalisation.
    let lease_owner = lease.clone();
    let exit = execute_after_open(
        ctx,
        Execution {
            args,
            embedded_root,
            root_offline,
            lease,
            user_config,
            manifest,
            workspace,
            metadata,
            project_root,
        },
        trace.recorder(),
    );
    // Consumes the owner: finishes the index, drops the last handle (and with
    // it the cooperative lock), and returns the member to attach.
    let finalized = compile_trace::finalize(trace, exit, &now);
    let rendered = render_finalized(ctx, finalized);
    drop(lease_owner);
    rendered
}

/// The prepared inputs the executed region owns.
struct Execution {
    args: InstallArgs,
    embedded_root: Option<PathBuf>,
    root_offline: bool,
    lease: Arc<LifecycleLease>,
    user_config: UserConfig,
    manifest: SelectedManifest,
    workspace: PreparedWorkspace,
    metadata: RunMetadata,
    project_root: PathBuf,
}

/// The one boundary: everything after `prepare` and before `finalize`.
///
/// No `?` and no `return Err` leaves this function — the inner `Result` is
/// classified into the typed exit instead.
fn execute_after_open(
    ctx: &output::Context,
    execution: Execution,
    trace: Option<&vibe_workspace::compile_trace::TraceRun>,
) -> CommandExit<RegisteredReportDraft> {
    let Execution {
        args,
        embedded_root,
        root_offline,
        lease,
        user_config,
        manifest,
        workspace,
        metadata,
        project_root,
    } = execution;
    // The failure draft below names the same root the execution used, so it
    // is cloned rather than re-resolved: two canonicalisations are two answers
    // to "which node did this command act on".
    let failed_root = project_root.clone();
    let confirm_gate = super::CliConfirmGate::new(ctx, args.assume_yes);
    let outcome = execute_prepared(
        ctx,
        InstallExecution {
            args,
            embedded_root,
            root_offline,
            lease,
            project_root,
            user_config,
            manifest,
            workspace,
            metadata,
            resolver_factory: &super::CliResolverFactory,
            confirm_gate: &confirm_gate,
            lifecycle_output: None,
            trace,
        },
        |root, disposition, run: InstallRunContext, workspace: &vibe_workspace::Workspace| {
            crate::commands::lifecycle::after_direct_install(ctx, root, disposition, run, workspace)
        },
    );
    match outcome {
        Ok(run) => classify_success(run),
        // A failure the substrate MEASURED arrives carried: the slot rows and
        // progress it froze, plus its own emission policy. Anything else is a
        // generic stage failure and takes the default install draft — the same
        // registered family this command has always reported, with the
        // historical silence of stages that never emitted one.
        Err(error) => compile_trace::classify(absorb_resume_failure(error, &failed_root), || {
            RegisteredReportDraft::Install(Box::new(InstallDraft::failed(
                &failed_root,
                vibe_install::InstallProgress::default(),
                Vec::new(),
            )))
        }),
    }
}

/// Give a NEUTRAL resume failure this command's registered family.
///
/// The substrate cannot pick one — the same code is a phase verb's prerequisite
/// and `vibe update --all`'s delegate — so it transports the measurement and
/// the outer command decides. For `vibe install` the answer is the install
/// root, with the emission policy a generic resume failure has always had:
/// silence while tracing is off. The error object is never formatted; it is
/// handed straight back to the carrier that returns it.
///
/// Total: an error that is not a transported resume failure is returned exactly
/// as it arrived, so the carried-draft classifier below still sees its own.
fn absorb_resume_failure(error: anyhow::Error, root: &std::path::Path) -> anyhow::Error {
    match crate::commands::install::take_resume_failure(error) {
        Ok(failure) => compile_trace::carry(
            RegisteredReportDraft::Install(Box::new(InstallDraft::failed(
                root,
                failure.progress,
                failure.reports,
            ))),
            failure.original,
            false,
        ),
        Err(error) => error,
    }
}

/// Success, fresh and park all report the SAME registered root; only the
/// deferred-plan rule differs, and it is read from the typed handoff.
fn classify_success(run: InstallRun) -> CommandExit<RegisteredReportDraft> {
    let parked = matches!(run.disposition, InstallDisposition::Parked) || run.parked.is_some();
    let draft = RegisteredReportDraft::Install(Box::new(InstallDraft::from_run(run)));
    if parked {
        CommandExit::Parked(draft)
    } else {
        CommandExit::Success(draft)
    }
}

/// The injected instant. Both the supersession pass and the finish read time
/// through this one closure, so a test can count its calls and prove the
/// disabled path never asked.
fn now() -> vibe_wire::generated::shared::Timestamp {
    chrono::Utc::now()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::compile_trace::{CommandExit, classify};
    use crate::commands::install::{ResumeFailure, resume};

    /// A fully-defaulted `InstallArgs` for the preparation-boundary red —
    /// every flag off, exactly one field (the path) set by the caller.
    fn args(path: std::path::PathBuf) -> InstallArgs {
        InstallArgs {
            packages: Vec::new(),
            path,
            registry: None,
            assume_yes: true,
            language: None,
            features: Vec::new(),
            no_default_features: false,
            all_features: false,
            exact: false,
            auth_required: false,
            solver: None,
            git: None,
            tag: None,
            branch: None,
            rev: None,
            git_auth: None,
            git_token_env: None,
            force: false,
            prefer_embedded: false,
            no_prefer_embedded: false,
            no_default_registry: false,
            offline: true,
            embedded_short_circuit: false,
            prefer_local: false,
            no_prefer_local: false,
            trace_compile: false,
        }
    }

    fn quiet_ctx() -> output::Context {
        output::Context::from_flags(true, false, None, true, crate::cli::AgentModeArg::Cli)
    }

    /// The pre-lease-snapshot barrier, pinned for real (R7.4 §2.1): the
    /// safefs `before_lock` race hook fires between `Project::open` and the
    /// OS lock — exactly the window in which a concurrent editor rewrites
    /// the selected manifest — and rewrites a valid manifest into a
    /// SEMANTICALLY DIFFERENT one (a standing `[compile] trace` request).
    ///
    /// The correct order consumes the POST-hook file: the prepared identity
    /// carries the post-hook activation. An order that read the manifest
    /// before acquiring would freeze the pre-hook bytes and this test fails
    /// — which is the discrimination the planted change buys: the file was
    /// valid before AND after, so only the ORDER decides which semantics
    /// the command runs with.
    #[test]
    fn the_selected_manifest_snapshot_is_taken_after_the_lease_acquisition() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("vibe.toml"),
            "[project]
name = \"demo\"
version = \"0.1.0\"
",
        )
        .unwrap();
        let manifest_path = dir.path().join("vibe.toml");
        let rewritten = manifest_path.clone();
        vibe_safefs::arm_before_lock(Some(Box::new(move |_, name| {
            if name == "lifecycle.lock" {
                std::fs::write(
                    &rewritten,
                    "[project]
name = \"demo\"
version = \"0.1.0\"

[compile]
trace = true
",
                )
                .unwrap();
            }
        })));
        // The hook is one-shot: the lifecycle acquisition consumes it, and
        // the compile-trace lock `prepare_trace` may take below finds
        // nothing armed.
        let prepared = prepare_direct_install(&quiet_ctx(), &args(dir.path().to_path_buf()), true)
            .expect("the preparation completes against the post-acquisition tree");
        assert!(
            prepared.metadata.trace_compile,
            "the identity carries the POST-hook activation — the manifest snapshot was \
             taken after the lease, not before it",
        );
        assert!(
            std::fs::read_to_string(&manifest_path)
                .unwrap()
                .contains("trace = true"),
            "the hook really fired inside the acquire window",
        );
    }

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

    fn transported() -> anyhow::Error {
        resume::carry_resume_failure(ResumeFailure {
            original: anyhow::Error::new(Sentinel).context("finishing the parked slot run"),
            progress: vibe_install::InstallProgress {
                complete: true,
                fresh: false,
                materialised: vec!["vibedeps/org.demo.tools/0.1.0".into()],
                skipped: Vec::new(),
                pruned: Vec::new(),
                nodes_regenerated: vec![".".into()],
            },
            reports: vec![
                row("slot:pre-install", "ok"),
                row("slot:post-install", "fail"),
            ],
            packages_resolved: 4,
        })
    }

    /// `vibe install` names the INSTALL family for a neutral resume failure,
    /// keeps the rows and progress the substrate measured, and returns the
    /// exact original error with the historical silence of a generic resume
    /// failure.
    ///
    /// Reducing the transport to `failure.original` — what this site used to do
    /// — makes the fallback report `InstallProgress::default()` and zero rows
    /// over a run that had already finished somebody's parked slot work.
    #[test]
    fn a_neutral_resume_failure_becomes_a_measured_install_root() {
        let root = std::path::PathBuf::from("/p");
        let absorbed = absorb_resume_failure(transported(), &root);
        let CommandExit::Failed {
            report,
            original_error,
            emit_when_trace_disabled,
        } = classify(absorbed, || panic!("the carrier decides, not the fallback"))
        else {
            panic!("a failure is a failure");
        };
        assert!(!emit_when_trace_disabled, "historically silent");
        assert!(original_error.downcast_ref::<Sentinel>().is_some());
        assert!(
            original_error.downcast_ref::<ResumeFailure>().is_none(),
            "the neutral wrapper never escapes to main",
        );
        let RegisteredReportDraft::Install(draft) = report else {
            panic!("this command's own family");
        };
        let built = draft.into_report(None);
        assert!(!built.ok);
        assert_eq!(built.materialised, ["vibedeps/org.demo.tools/0.1.0"]);
        let statuses: Vec<&str> = built
            .contributions
            .iter()
            .map(|row| row.status.as_str())
            .collect();
        assert_eq!(statuses, ["ok", "fail"], "both rows, in order");
    }

    /// Anything else is left exactly as it arrived, so the carried-draft
    /// classifier still sees its own carriers.
    #[test]
    fn an_ordinary_error_passes_through_untouched() {
        let error = absorb_resume_failure(
            anyhow::anyhow!("planning blew up"),
            std::path::Path::new("/p"),
        );
        assert_eq!(error.to_string(), "planning blew up");
    }
}
