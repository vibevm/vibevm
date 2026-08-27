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

use anyhow::{Context, Result};
use vibe_core::user_config::UserConfig;
use vibe_lifecycle::RunMetadata;

use crate::cli::InstallArgs;
use crate::commands::compile_trace::{
    self, CommandExit, RegisteredReportDraft, TracePreparation, render_finalized,
};
use crate::output;

use super::{
    InstallDisposition, InstallDraft, InstallExecution, InstallRun, InstallRunContext,
    PreparedWorkspace, SelectedManifest, execute_prepared, resolve_project_root,
};

/// Everything `vibe install` reads before it may compile.
///
/// One config load, one manifest snapshot, one workspace. Execution below
/// consumes exactly these — it re-reads none of them, so the tree the trace
/// was locked against is the tree the install works on.
struct PreparedInstall {
    project_root: PathBuf,
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
    };
    let trace = prelude.prepare_trace(&now);
    Ok(PreparedInstall {
        project_root: prelude.project_root,
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
        user_config,
        metadata,
        manifest,
        workspace,
        trace,
    } = prepare_direct_install(ctx, &args, root_offline)?;
    let exit = execute_after_open(
        ctx,
        Execution {
            args,
            embedded_root,
            root_offline,
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
    render_finalized(ctx, finalized)
}

/// The prepared inputs the executed region owns.
struct Execution {
    args: InstallArgs,
    embedded_root: Option<PathBuf>,
    root_offline: bool,
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
    let outcome = execute_prepared(
        ctx,
        InstallExecution {
            args,
            embedded_root,
            root_offline,
            project_root,
            user_config,
            manifest,
            workspace,
            metadata,
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
        Err(error) => compile_trace::classify(error, || {
            RegisteredReportDraft::Install(Box::new(InstallDraft::failed(
                &failed_root,
                vibe_install::InstallProgress::default(),
                Vec::new(),
            )))
        }),
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
