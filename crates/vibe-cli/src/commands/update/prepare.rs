//! Everything `vibe update` reads before it may compile.
//!
//! One canonical root, one `UserConfig`, one selected-manifest snapshot, one
//! workspace built from that snapshot, one lifecycle identity, one recorder.
//! Both shapes of the command — whole and scoped — consume the same epoch, so
//! there is no branch in which "which node is this" or "what does its tree look
//! like" has two answers.
//!
//! The ORDER is the direct-install order, and each step is a different law:
//! the canonical root first (a path that cannot be named is not a command),
//! then the config (a malformed one must still fail before a run directory is
//! allocated), then the manifest snapshot and the tree built from it, then the
//! one identity, then — immediately — the recorder. Compile-trace activation
//! needs the selected manifest, the identity needs the activation, and the
//! metadata needs the identity; reading the manifest a second time to break
//! that chain would be two answers to a question an operator can edit between.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE");

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use vibe_core::user_config::UserConfig;
use vibe_lifecycle::{LifecycleLease, RunMetadata};

use crate::cli::UpdateArgs;
use crate::commands::compile_trace::TracePreparation;
use crate::commands::install::{
    PreparedWorkspace, SelectedManifest, acquire_lease, resolve_project_root,
};
use crate::output;

pub(super) struct PreparedUpdate {
    pub(super) project_root: PathBuf,
    /// The workspace mutation lease, acquired BEFORE the config/manifest/
    /// workspace reads below — the outermost lock of the whole command.
    pub(super) lease: Arc<LifecycleLease>,
    pub(super) user_config: UserConfig,
    /// PROP-010 §2.5, resolved ONCE: root `--offline` > `VIBE_OFFLINE` >
    /// user-config `[net].offline`. `vibe update` carries no `--offline` of
    /// its own, so the CLI rung is the root flag alone.
    ///
    /// Carried rather than recomputed below because the two consumers must
    /// agree: the run metadata records the posture the invocation ran under,
    /// and the resolver enforces it. Re-deriving one of them from the config
    /// alone — as the metadata used to, missing the `VIBE_OFFLINE` rung —
    /// makes the recorded envelope disagree with what the resolver actually
    /// did.
    pub(super) offline: bool,
    pub(super) metadata: RunMetadata,
    pub(super) manifest: SelectedManifest,
    pub(super) workspace: PreparedWorkspace,
    pub(super) trace: TracePreparation,
}

pub(super) fn prepare(
    ctx: &output::Context,
    args: &UpdateArgs,
    root_offline: bool,
) -> Result<PreparedUpdate> {
    let project_root = resolve_project_root(&args.path)?;
    // The OUTERMOST lock, before anything execution-shaped is read: a
    // contended workspace refuses here, typed, having allocated nothing but
    // the infrastructure lock file itself.
    let lease = acquire_lease(&project_root)?;
    // Before the identity, and before anything is allocated: a malformed user
    // config must still fail ahead of the run-directory allocation.
    let user_config = UserConfig::load().context("loading the user config")?;
    // The ONE offline answer — see the field note.
    let offline = output::resolve_offline(root_offline, user_config.net.offline);
    // The ONE read of this command's selected `vibe.toml`, and the ONE tree
    // built from it. Its stored error is consumed inside the executed region,
    // at the boundary that has always reported it.
    let manifest = SelectedManifest::read(&project_root);
    let workspace = manifest.prepare_workspace(&project_root);
    let requested = "update";
    let chain = vec!["install".to_string()];
    // `vibe update` supplies its OWN requested phase, chain and run identity,
    // so the handoff a hosted row publishes resumes with `vibe update` rather
    // than impersonating install.
    let prelude = crate::commands::lifecycle::run_prelude(
        ctx,
        project_root,
        workspace,
        lease.clone(),
        requested,
        &chain,
        // MATERIALISATION scope is what `--all` selects; there is no lifecycle
        // repark force on this command at all.
        false,
        manifest.request(args.trace_compile),
    )
    .context("selecting the update lifecycle run identity")?;
    let metadata = RunMetadata {
        requested: requested.to_string(),
        chain,
        offline,
        assume_yes: args.assume_yes || ctx.is_unattended() || ctx.is_json(),
        agent_mode: ctx.agent_mode(),
        force: false,
        // The EFFECTIVE bit the selector computed — an adopted run's sticky
        // trace bit, not the raw request, so a resume cannot rewrite a traced
        // run's header back to untraced.
        trace_compile: prelude.identity.compile_trace,
        run_id: prelude.identity.run_id.clone(),
        started: prelude.identity.started.clone(),
        selected: prelude.selected.clone(),
    };
    let trace = prelude.prepare_trace(&super::now);
    Ok(PreparedUpdate {
        project_root: prelude.project_root,
        lease: prelude.lease,
        user_config,
        offline,
        metadata,
        manifest,
        workspace: prelude.workspace,
        trace,
    })
}
