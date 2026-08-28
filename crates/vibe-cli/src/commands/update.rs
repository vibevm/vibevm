//! `vibe update [<pkgref>...] [--all]` — re-resolve and re-materialise.
//!
//! `vibe update` with no arguments, or `--all`, re-resolves the whole
//! declared graph — exactly the `vibe install` from-manifest path, so it
//! delegates there.
//!
//! `vibe update <pkgref>...` is **scoped**: only the named packages — and
//! the transitive subtree each pulls — are re-resolved against their
//! declared constraints and re-materialised. Every other package keeps
//! its lockfile version and its `vibedeps/` slot untouched. A package
//! whose version moves has its superseded slot removed, and the boot
//! artifacts are regenerated from the new `vibedeps/` state.
//!
//! ## One command, one owner
//!
//! Both shapes are ONE command, so both run under exactly one
//! [`crate::commands::compile_trace::TracePreparation`] — opened here, before
//! anything compiles, and consumed by exactly one typed exit. The whole-graph
//! shape borrows that recorder into the install substrate rather than letting
//! the substrate open its own; the scoped shape borrows it into its own boot
//! regeneration. A second owner would be a second holder of the project's
//! cooperative trace lock over the same work.
//!
//! Nothing between `prepare` and `finalize` may return with `?`: an open
//! recorder holds that lock and leaves its index `running` on disk, so the
//! whole executed region is a function returning a value, and every error
//! inside it is classified into that value.
//!
//! Spec: spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009-loading-model.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

mod draft;
mod inputs;
pub(crate) mod lifecycle;
mod prepare;
mod scoped;
mod whole;

use std::path::PathBuf;

use anyhow::Result;

use crate::cli::UpdateArgs;
use crate::commands::compile_trace::{self, render_finalized};
use crate::output;

pub(crate) use draft::UpdateDraft;
/// The invocation facts, for the cross-module reds that build an update root
/// by hand. Production builds one inside this module, from the args.
#[cfg(test)]
pub(crate) use draft::UpdateIdentity;

use prepare::PreparedUpdate;

pub fn run(
    ctx: &output::Context,
    args: UpdateArgs,
    embedded_root: Option<PathBuf>,
    root_offline: bool,
) -> Result<()> {
    let PreparedUpdate {
        lease,
        user_config,
        offline,
        metadata,
        selection,
        trace,
    } = prepare::prepare(ctx, &args, root_offline)?;
    // The boundary's OWNER share: the executed regions below consume their
    // own clones, while this one lives to the end of the command — through
    // the final report and the trace finalisation.
    let lease_owner = lease.clone();
    // The ONE decision this boundary makes. `--all` (or no packages) re-resolves
    // the whole declared graph through the install substrate; anything else
    // moves exactly the named subtree. Both are `vibe update`, both report the
    // update root, and both borrow the SAME recorder.
    let whole = args.all || args.packages.is_empty();
    let execution = Execution {
        args,
        embedded_root,
        offline,
        lease,
        user_config,
        selection,
        metadata,
    };
    let exit = if whole {
        whole::execute_after_open(ctx, execution, trace.recorder())
    } else {
        scoped::execute_after_open(ctx, execution, trace.recorder())
    };
    // Consumes the owner: finishes the index, drops the last handle (and with
    // it the cooperative lock), and returns the member to attach.
    let finalized = compile_trace::finalize(trace, exit, &now);
    let rendered = render_finalized(ctx, finalized);
    drop(lease_owner);
    rendered
}

/// The prepared inputs both executed regions own.
///
/// One canonical root, one config, one selected-manifest snapshot, one
/// workspace, one run identity. The execution below consumes exactly these and
/// re-reads none of them, so the tree the trace was locked against is the tree
/// the update works on.
pub(super) struct Execution {
    pub(super) args: UpdateArgs,
    pub(super) embedded_root: Option<PathBuf>,
    /// The command's ONE resolved offline posture — see
    /// [`prepare::PreparedUpdate::offline`]. Nothing below re-resolves it and
    /// nothing below reloads the config it was read from.
    pub(super) offline: bool,
    /// The workspace mutation lease from the prepare epoch — the ONE
    /// acquisition, borrowed by both executed regions below.
    pub(super) lease: std::sync::Arc<vibe_lifecycle::LifecycleLease>,
    pub(super) user_config: vibe_core::user_config::UserConfig,
    /// The ONE selected-world provenance bundle: the canonical root, the
    /// manifest snapshot taken at it, and the tree built from THAT snapshot.
    pub(super) selection: crate::commands::install::PreparedSelection,
    pub(super) metadata: vibe_lifecycle::RunMetadata,
}

/// The injected instant. Both the supersession pass and the finish read time
/// through this one closure, so a test can count its calls and prove the
/// disabled path never asked.
pub(super) fn now() -> vibe_wire::generated::shared::Timestamp {
    chrono::Utc::now()
}
