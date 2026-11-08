//! The no-op `vibe update --all`: every declared root re-resolved, nothing
//! moved, and therefore nothing written.
//!
//! `--all` routes through the scoped region (B-234), which re-materialises
//! every re-resolved slot under [`super::SCOPED_INTEGRITY`] and regenerates
//! boot. When the resolution moved nothing that was correct but untruthful: the
//! run rewrote every slot and boot artifact and reported `unchanged: false`
//! over a project it left byte-identical. This cell decides, BEFORE region 2
//! performs its first semantic mutation, that the apply would be a no-op, so
//! the command returns the fresh shape instead.
//!
//! A run is a no-op only when ALL of these hold, each proven rather than
//! assumed:
//!
//! - the planned lock equals the loaded one — no version bump, no source or
//!   provenance change, nothing entering or leaving the closure;
//! - no re-resolved package is refreshed in place — an in-place checkout's
//!   working tree can be proven current only by fetching onto it;
//! - every re-resolved slot is present and verifies against its own source
//!   hash under the effective spec format
//!   ([`vibe_install::resolution_slots_verified`]);
//! - the lifecycle state owes nothing — no slot continuation, no delegated or
//!   failed row a real apply would service or retry.
//!
//! Anything this cannot read answers "not a no-op": the ordinary apply then
//! runs and reports whatever it finds, exactly as before.
//!
//! Boot is still regenerated over the unchanged world, exactly as the install
//! fresh path does: the regeneration publishes only files whose bytes change,
//! so current boot artifacts keep their bytes and times while a missing or
//! stale one is repaired. It runs under the same [`InstallRollback`] the apply
//! uses, so a stop during it puts every boot file back.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#command-summary");

use anyhow::{Context, Result};
use vibe_core::manifest::{Lockfile, SpecFormat};
use vibe_install::InstallProgress;
use vibe_lifecycle::LifecycleLease;
use vibe_wire::generated::lifecycle_state::ExecutionRecordStatus;
use vibe_workspace::Workspace;
use vibe_workspace::compile_trace::TraceRun;
use vibe_workspace::install::{InstallRollback, ResolvedDep, regenerate_boot_from_traced};

use crate::output;

use super::super::draft::{UpdateDraft, UpdateIdentity};
use super::Resolved;
use super::stage::PendingInPlace;

/// Everything the no-op decision reads; nothing in it is mutated.
pub(super) struct NoopProbe<'a> {
    pub(super) workspace: &'a Workspace,
    pub(super) lease: &'a LifecycleLease,
    pub(super) old_lock: &'a Lockfile,
    pub(super) planned_lock: &'a Lockfile,
    pub(super) updated: &'a [Resolved],
    pub(super) pending_in_place: &'a [PendingInPlace],
    pub(super) spec_format: SpecFormat,
}

/// What the no-op's boot regeneration runs over.
pub(super) struct NoopBoot<'a> {
    /// The complete (unchanged) world.
    pub(super) full_world: &'a [ResolvedDep],
    pub(super) trace: Option<&'a TraceRun>,
}

/// The completed `unchanged: true` draft when the apply would move nothing,
/// `None` when the ordinary apply must run.
pub(super) fn unchanged_draft(
    ctx: &output::Context,
    probe: &NoopProbe<'_>,
    boot: NoopBoot<'_>,
    identity: &UpdateIdentity,
    resolved: usize,
) -> Result<Option<UpdateDraft>> {
    if !is_noop(probe) {
        return Ok(None);
    }
    let workspace = probe.workspace;
    let cancellation = ctx.progress().cancellation().clone();
    cancellation.checkpoint()?;
    cancellation.publishing();
    let mut rollback =
        InstallRollback::new(&workspace.root).with_cancellation(cancellation.clone());
    let task = ctx.progress().task("Regenerating workspace boot artifacts");
    let regenerated = rollback
        .regenerate_boot(workspace, boot.full_world, || {
            regenerate_boot_from_traced(workspace, boot.full_world, probe.spec_format, boot.trace)
        })
        .context("regenerating boot artifacts")
        .and_then(|nodes| {
            cancellation
                .checkpoint()
                .map(|()| nodes)
                .map_err(Into::into)
        });
    let nodes = match regenerated {
        Ok(nodes) => nodes,
        Err(error) => {
            task.fail("boot regeneration failed");
            return Err(match rollback.restore(None, &[]) {
                Ok(()) => {
                    cancellation.restored();
                    error
                }
                Err(reason) => error.context(NoopRollbackFailed { reason }),
            });
        }
    };
    // Nothing else is published: the lock and every slot stay as they were.
    drop(rollback);
    cancellation.committed();
    task.set_progress(nodes.len() as u64, Some(nodes.len() as u64), "nodes");
    task.finish();
    Ok(Some(UpdateDraft::completed(
        identity,
        resolved,
        Vec::new(),
        InstallProgress::fresh(nodes),
        Vec::new(),
        None,
    )))
}

/// The unchanged update stopped or failed while regenerating boot and could
/// not put the boot files back.
#[derive(Debug, thiserror::Error)]
#[error(
    "restoring the boot artifacts after the unchanged update stopped also failed: {reason} \
     (violates spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail; fix: \
     inspect that path, then run `vibe update --all` again)"
)]
struct NoopRollbackFailed {
    reason: String,
}

fn is_noop(probe: &NoopProbe<'_>) -> bool {
    if probe.planned_lock != probe.old_lock || !probe.pending_in_place.is_empty() {
        return false;
    }
    let rows = super::stage::project_rows(probe.workspace, probe.updated);
    if !vibe_install::resolution_slots_verified(&probe.workspace.root, &rows, probe.spec_format) {
        return false;
    }
    lifecycle_owes_nothing(probe.lease)
}

/// No slot continuation and no delegated or failed row: a state a real apply
/// would leave exactly as it found it.
fn lifecycle_owes_nothing(lease: &LifecycleLease) -> bool {
    match vibe_lifecycle::LifecycleStateStore::peek_with_lease(lease) {
        Ok(None) => true,
        Ok(Some(state)) => {
            state.run.slot_continuation.is_none()
                && state.execution.values().all(|row| {
                    !matches!(
                        row.status,
                        ExecutionRecordStatus::Delegated | ExecutionRecordStatus::Fail
                    )
                })
        }
        Err(_) => false,
    }
}
