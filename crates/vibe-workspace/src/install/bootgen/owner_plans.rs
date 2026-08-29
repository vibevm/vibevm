//! The durable extension world of one boot regeneration, and the
//! owner-scoped transform plan each lane owner is compiled with (R4
//! architecture §§4–5, `R4-TRANSFORM-PLAN-ABI` §1).
//!
//! **This seam OWNS NO EPOCH, and that decides everything below.** R4
//! architecture §4 is explicit: an adapter "orders the epoch a command owns;
//! it never chooses or invents one". Boot regeneration is not a command — it
//! runs inside `install`, `check`, `reinstall`, `uninstall` and `init`, and
//! no caller tells it which lock value is its authority. So the absolute
//! root `vibe.lock` it can read here is EVIDENCE about the installed world,
//! never authority over it.
//!
//! The distinction is not academic; it is observable. During `vibe install`
//! the boot lane is written BEFORE the resolution's lock is published, so
//! the file on disk is the PRE-install epoch: it does not yet list the
//! package the node now requires, and observing a world against it produces
//! a world that never existed. Post-install paths (`check`, `reinstall`,
//! `uninstall`, and every regeneration from the materialised tree) read a
//! lock that does agree with the tree, and there the observation is real.
//!
//! Two rules follow, and they are different rules:
//!
//! 1. **A world that cannot be observed is not a fault here.** No lock, an
//!    unreadable lock, or a lock that disagrees with the tree all mean the
//!    same thing at a seam with no epoch: nothing to scope by. The lane is
//!    then written with [`TransformPlan::empty`] — the exact historical
//!    schedule, bytes and errors (`R4-TRANSFORM-PLAN-ABI` §7). The durable
//!    adapter's own strictness is untouched: it still refuses a malformed
//!    world, and its tests still prove it. What is relaxed is this seam's
//!    right to call a disagreement malformed.
//! 2. **A world that IS observed is judged strictly.** Once a view exists,
//!    a collection refusal or a lowering refusal is a real declaration
//!    defect — independent of which lock produced the view — and refuses.
//!
//! <!-- REVIEW: rule 1 should narrow to "no lock at all" once the commands
//! that DO own an epoch thread their lock value in (R4 architecture §4: ready
//! apply overlays its resolution on the pre-apply lock order; uninstall
//! compiles the remaining future world from the in-memory lock). Until then
//! an install-time compile-point extension is not observed at all. Doing it
//! needs a signature change on `regenerate_boot_from_traced`, whose callers
//! are `vibe-cli` and `vibe-orchestrator` — outside T10B's write perimeter,
//! and the same migration §5.3 already names as follow-up. -->
//!
//! **Activation authority follows the artifact being written** (PROP-054
//! `##COMPILE-ACTIVATION`): a node lane is scoped by that node's own
//! manifest, and a package's unit lane by that package's own. So the two
//! entries here are separate by construction, not one plan reused.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION");

use vibe_core::PackageName;
use vibe_extension_registry::{DependencyProviderId, ExtensionRegistry, ExtensionWorld};

use crate::extension_world::ExtensionWorldError;

use super::*;

/// The durable root lock, when this tree has one that reads.
///
/// A missing file is the ordinary pre-install state. A file that does not
/// read — an unsupported schema, a truncated write — is deliberately treated
/// the same way rather than propagated: boot regeneration must still produce
/// a lane for a tree whose lock this binary cannot interpret, and the lane it
/// produces is byte-identical to the historical one because an owner with no
/// world contributes no transform.
pub(super) fn read_durable_lock(workspace_root: &Path) -> Option<Lockfile> {
    let path = workspace_root.join(Lockfile::FILENAME);
    if !path.is_file() {
        return None;
    }
    Lockfile::read(&path).ok()
}

/// Snapshot the durable world of one selected node, when one is observable.
///
/// One snapshot per lane owner group: every participating manifest is parsed
/// once inside it, and dependency order comes from the root lock and nothing
/// else. A snapshot that does not come out — a locked package with no slot, a
/// slot contradicting the lock — means the lock and the tree disagree, which
/// at a seam that owns no epoch is "nothing observable", not "malformed"
/// (module doc, rule 1). The adapter's own refusal law is untouched and
/// still tested; this is only about who may call a disagreement a fault.
pub(super) fn durable_world(
    workspace_root: &Path,
    node_root: &Path,
    node_manifest: &Manifest,
    lock: Option<&Lockfile>,
) -> Option<DurableExtensionWorld> {
    let lock = lock?;
    DurableExtensionWorld::from_lock(workspace_root, node_root, node_manifest, lock).ok()
}

/// The NODE lane's own transform plan.
///
/// The node is the host of its own view, so its declarations and its
/// controls are the live ones and every package in its closure sits inert
/// beside them. Presets are deliberately empty here: package-skill presets
/// belong to the orchestrator's migration onto this adapter, and inventing a
/// preset tier for boot would activate contributions no boot path declares.
pub(super) fn node_owner_plan(
    world: Option<&DurableExtensionWorld>,
    node_rel: &str,
) -> Result<TransformPlan, WorkspaceError> {
    let Some(world) = world else {
        return Ok(TransformPlan::empty());
    };
    // A closure the lock cannot resolve is the same disagreement `from_lock`
    // reports, one step later: not observable here (module doc, rule 1).
    let Ok(view) = world.node_owner_view() else {
        return Ok(TransformPlan::empty());
    };
    lower_owner_view(world_registry(view)?, node_rel)
}

/// Package P's unit-lane transform plan.
///
/// P takes the host seat through the kernel's own dependency-seat→owner-seat
/// projection, so P's own controls become that lane's live controls. A unit
/// the durable world does not install is outside the extension world
/// entirely (R4 architecture §3's orphan rule) and compiles with the empty
/// plan — the same answer an unobservable world gets, for the same reason.
pub(super) fn unit_owner_plan(
    world: Option<&DurableExtensionWorld>,
    unit: &UnitId,
) -> Result<TransformPlan, WorkspaceError> {
    let Some(world) = world else {
        return Ok(TransformPlan::empty());
    };
    let owner = DependencyProviderId::new(
        unit.0.clone(),
        // The unit table's name half is the install model's bare string; it
        // is parsed through the one grammar here and refused typed, never
        // defaulted into a different owner.
        PackageName::parse(&unit.1).map_err(|error| WorkspaceError::UntypedBootProvenance {
            origin: format!("{}/{}", unit.0, unit.1),
            component: "unit package name",
            spelling: unit.1.clone(),
            reason: error.to_string(),
        })?,
    );
    if !world.lane_owners().any(|installed| installed == &owner) {
        return Ok(TransformPlan::empty());
    }
    let Ok(view) = world.package_owner_view(&owner) else {
        return Ok(TransformPlan::empty());
    };
    lower_owner_view(world_registry(view)?, &owner.to_string())
}

/// Collect one owner-scoped view through the ONE kernel entry.
///
/// Strict, and deliberately so (module doc, rule 2): the view already
/// exists, so a collection refusal is a declaration this manifest could not
/// have meant — a duplicate key, an `applies_to` at an illegal point — and
/// no lock value would have made it lawful.
fn world_registry(view: ExtensionWorld) -> Result<ExtensionRegistry, WorkspaceError> {
    // Presets: see [`node_owner_plan`] — boot declares none.
    collect_owner_view(view, Vec::new()).map_err(world_error)
}

/// Lower one collected owner registry's compile family into its plan.
///
/// `enabled_compile_rows()` is the input contract the lowering states: every
/// enabled `compile:*` row, in the registry's ONE effective order. The
/// filtering happens here, in the crate that owns the world; the refusing
/// happens in `vibe-spec`, which owns the lowering.
fn lower_owner_view(
    registry: ExtensionRegistry,
    owner: &str,
) -> Result<TransformPlan, WorkspaceError> {
    TransformPlan::from_effective_rows(&registry.enabled_compile_rows()).map_err(|source| {
        WorkspaceError::TransformPlan {
            owner: owner.to_owned(),
            source,
        }
    })
}

fn world_error(source: ExtensionWorldError) -> WorkspaceError {
    WorkspaceError::ExtensionWorld {
        source: Box::new(source),
    }
}

#[cfg(test)]
#[path = "owner_plans_tests.rs"]
mod tests;
