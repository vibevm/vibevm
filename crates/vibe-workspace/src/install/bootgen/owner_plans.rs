//! Owner-scoped transform plans lowered from one explicit extension-world
//! epoch (R5.4 `EPOCH-WORLD`).
//!
//! Boot regeneration receives the command's exact ordered resolution and
//! builds its [`ExtensionWorldEpoch`] before lowering anything. This seam does
//! not read a lock and has no optional-world state: an empty package sequence
//! is explicit and lawful; every malformed closure or unknown package owner is
//! a typed refusal.
//!
//! **Activation authority follows the artifact being written** (PROP-054
//! `##COMPILE-ACTIVATION`): a node lane is scoped by that node's own
//! manifest, and a package's unit lane by that package's own. So the two
//! entries here are separate by construction, not one plan reused.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION");

use vibe_core::PackageName;
use vibe_extension_registry::{DependencyProviderId, ExtensionRegistry, ExtensionWorld};

use crate::boot::hybrid::UnitInput;
use crate::extension_world::{ExtensionWorldEpoch, ExtensionWorldError};

use super::*;

/// The NODE lane's own transform plan.
///
/// The node is the host of its own view, so its declarations and its
/// controls are the live ones and every package in its closure sits inert
/// beside them. Presets are deliberately empty here: package-skill presets
/// belong to the orchestrator's migration onto this adapter, and inventing a
/// preset tier for boot would activate contributions no boot path declares.
pub(super) fn node_owner_plan(
    epoch: &ExtensionWorldEpoch,
    node_root: &Path,
    node_manifest: &Manifest,
    node_rel: &str,
) -> Result<TransformPlan, WorkspaceError> {
    let view = epoch
        .node_owner_view(node_root, node_manifest)
        .map_err(world_error)?;
    lower_owner_view(world_registry(view)?, node_rel)
}

/// EVERY table unit's owner plan, lowered exactly ONCE per run.
///
/// This is the ordering the fingerprint frame forces (R4 architecture §7.1):
/// a unit's owner-plan digest is an INPUT to its boot-graph fingerprint, and
/// the fingerprint decides whether that unit is emitted at all — so the plans
/// must exist before the fingerprints, not be lowered inside the emission
/// loop. The same map is then handed to emission, so one declaration is
/// lowered once and judged once. Two lowerings of one owner in one run would
/// be two refusal surfaces for one declaration.
///
/// Every unit in the table is lowered, not only the emitted ones, because
/// every unit in the table is FINGERPRINTED and a static parent hashes its
/// child's fingerprint. Consequence, accepted: an owner whose compile
/// declaration cannot be lowered now refuses the run even when that owner
/// emits no artifact of its own — its plan is part of its parents' identity,
/// so there is no honest reading under which it stays unjudged.
///
/// Units are walked in canonical `(group, name)` order so a refusal on a tree
/// with several bad owners names the same one every run.
pub(super) fn unit_owner_plans(
    epoch: &ExtensionWorldEpoch,
    table: &HashMap<UnitId, UnitInput>,
) -> Result<HashMap<UnitId, TransformPlan>, WorkspaceError> {
    let mut ordered: Vec<&UnitId> = table.keys().collect();
    ordered.sort();
    let mut plans = HashMap::with_capacity(ordered.len());
    for id in ordered {
        plans.insert(id.clone(), unit_owner_plan(epoch, id)?);
    }
    Ok(plans)
}

/// The boot-graph fingerprint's owner-plan frames: each unit's plan digest as
/// lowercase sha256 hex, present ONLY for a nonempty plan.
///
/// The absence IS the law (R4 architecture §7.1): an owner that activates
/// nothing contributes no frame, so its unit keeps the exact fingerprint it
/// had before this frame existed. `TransformPlan::digest_hex` answers `None`
/// for the empty plan, and that `None` is what this filter reads — the
/// emptiness is never re-derived here from a length or a flag.
pub(super) fn plan_digest_frames(
    plans: &HashMap<UnitId, TransformPlan>,
) -> HashMap<UnitId, String> {
    plans
        .iter()
        .filter_map(|(id, plan)| plan.digest_hex().map(|digest| (id.clone(), digest)))
        .collect()
}

/// Package P's unit-lane transform plan.
///
/// P takes the host seat through the kernel's own dependency-seat→owner-seat
/// projection, so P's own controls become that lane's live controls. A unit
/// absent from the supplied epoch is a caller/world disagreement and refuses.
pub(super) fn unit_owner_plan(
    epoch: &ExtensionWorldEpoch,
    unit: &UnitId,
) -> Result<TransformPlan, WorkspaceError> {
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
    let view = epoch.package_owner_view(&owner).map_err(world_error)?;
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

pub(super) fn world_error(source: ExtensionWorldError) -> WorkspaceError {
    WorkspaceError::ExtensionWorld {
        source: Box::new(source),
    }
}

#[cfg(test)]
#[path = "owner_plans_tests.rs"]
mod tests;
