//! Everything `vibe reinstall` reads before it may compile.
//!
//! One canonical root, one `UserConfig`, one resolved offline posture, one
//! selected-manifest snapshot, one workspace built from that snapshot, one
//! lifecycle identity, one recorder. Both modes — plain regeneration and
//! `--force` — consume the same epoch.
//!
//! ## Two manifests, and why they are different facts
//!
//! The SELECTED node's manifest decides compile-trace activation and takes
//! part in the one workspace load. It does NOT become the operational manifest:
//! `vibe reinstall` bubbles to the absolute workspace root and regenerates the
//! WHOLE workspace, so its resolver, its slot world and its lifecycle envelope
//! read `workspace.root_manifest` — exactly what they read before, now from the
//! already-loaded tree instead of a second `Manifest::read` of the same file.
//!
//! Confusing the two would change real behaviour: feeding a member's manifest
//! to whole-workspace machinery would alter which contributions are selected,
//! and a root `[compile] trace = true` would start activating tracing for a
//! member invocation that never asked. The selected node is an INPUT; the
//! workspace root is the operational HOST.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE");

use std::sync::Arc;

use anyhow::{Context, Result};
use vibe_core::user_config::UserConfig;
use vibe_lifecycle::{LifecycleLease, RunMetadata};

use crate::cli::ReinstallArgs;
use crate::commands::compile_trace::TracePreparation;
use crate::commands::install::{
    PreparedSelection, SelectedManifest, acquire_lease, resolve_project_root,
};
use crate::output;

pub(super) struct PreparedReinstall {
    /// The workspace mutation lease, acquired BEFORE the config/manifest/
    /// workspace reads below — the outermost lock of the whole command.
    pub(super) lease: Arc<LifecycleLease>,
    pub(super) user_config: UserConfig,
    /// PROP-010 §2.5, resolved ONCE: root `--offline` > `VIBE_OFFLINE` >
    /// user-config `[net].offline`.
    ///
    /// One value, carried everywhere. `run_force` used to resolve it against a
    /// SECOND `UserConfig::load`, and the continuation helper passed a
    /// hard-coded `false` into its own metadata — three answers to one
    /// question, on a file the operator can edit between them.
    pub(super) offline: bool,
    pub(super) metadata: RunMetadata,
    /// The ONE selected-world provenance bundle: the canonical root, the
    /// manifest snapshot taken at it, and the tree built from THAT snapshot.
    /// One value rather than three fields, so the executed region cannot be
    /// handed a manifest from one moment and a tree from another.
    pub(super) selection: PreparedSelection,
    pub(super) trace: TracePreparation,
}

pub(super) fn prepare(
    ctx: &output::Context,
    args: &ReinstallArgs,
    root_offline: bool,
) -> Result<PreparedReinstall> {
    let project_root = resolve_project_root(&args.path)?;
    // The OUTERMOST lock, before anything execution-shaped is read: a
    // contended workspace refuses here, typed, having allocated nothing but
    // the infrastructure lock file itself.
    let lease = acquire_lease(&project_root)?;
    let user_config = UserConfig::load().context("loading the user config")?;
    let offline = output::resolve_offline(root_offline, user_config.net.offline);
    // The ONE read of the SELECTED node's `vibe.toml` — see the module note on
    // what it decides and what it does not.
    let selection = SelectedManifest::read(&project_root).prepare();
    let requested = "reinstall";
    let chain = vec!["install".to_string()];
    // MATERIALISATION force and HOSTED-REPARK force are different things.
    //
    // `--force` is what re-fetches from source and so reaches changed slot
    // callbacks. The generic lifecycle force means something else entirely:
    // "fresh run id, no probe, repark". Setting it here made a forced reinstall
    // unable to satisfy its own task — every resume minted a new id and
    // reparked, forever. So the lifecycle force stays FALSE, in the selector
    // and in the metadata, and `--force` keeps its own unrelated job.
    let trace_request = selection.request(args.trace_compile);
    let prelude = crate::commands::lifecycle::run_prelude(
        ctx,
        selection,
        lease.clone(),
        requested,
        &chain,
        false,
        trace_request,
    )
    .context("selecting the reinstall lifecycle run identity")?;
    let metadata = RunMetadata {
        requested: requested.to_string(),
        chain,
        offline,
        assume_yes: args.assume_yes || ctx.is_unattended() || ctx.is_json(),
        agent_mode: ctx.agent_mode(),
        force: false,
        // The EFFECTIVE bit the selector computed — an adopted run's sticky
        // trace bit, not the raw request.
        trace_compile: prelude.identity.compile_trace,
        run_id: prelude.identity.run_id.clone(),
        started: prelude.identity.started.clone(),
        selected: prelude.selected.clone(),
    };
    let trace = prelude.prepare_trace(&super::now);
    Ok(PreparedReinstall {
        lease: prelude.lease,
        user_config,
        offline,
        metadata,
        selection: prelude.selection,
        trace,
    })
}
