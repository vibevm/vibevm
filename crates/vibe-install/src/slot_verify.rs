//! The `slot_integrity = verify` spot-check (PROP-011 §2.3/§5.2): before
//! the §2.3 fast path trusts a present slot, hash it and compare against
//! the `content_hash` the resolution carries — the lockfile pin on the
//! fresh-lock path, the fetch's own hash on a re-resolve (which becomes
//! the next lockfile pin either way). A match accepts the slot WITHOUT
//! the re-copy: the spot-check REPLACES the work, it does not add to it —
//! the always-re-copy behaviour `verify` shipped with was stricter and
//! costlier than the contract.
//!
//! [`RegistrySlotVerifier`] is the implementation side of the seam:
//! vibe-workspace (where the skip decision lives) deliberately depends on
//! neither hash crate, so it takes the check as the
//! [`SlotVerifier`](vibe_workspace::install::SlotVerifier) seam and THIS
//! crate — which already depends on `vibe-registry` — supplies the
//! hasher. The registry's native [`compute_content_hash`] is the right
//! function for a second reason: it computes recipe 0 (`sha256:`), the
//! recipe every lockfile hash is written with, so the comparison is
//! same-recipe by construction; a `sha256-tree/1:` pin (PROP-044 §4.7)
//! dispatches to the tree recipe so both wire forms `ContentHash` accepts
//! verify correctly.

use std::collections::HashMap;
use std::path::Path;

use vibe_core::Group;
use vibe_core::manifest::SpecFormat;
use vibe_registry::{RecipeId, compute_content_hash, compute_content_hash_with};
use vibe_workspace::install::{ResolvedDep, SlotCheck, SlotVerifier};

use crate::fetched::Fetched;

/// The [`SlotVerifier`] for the apply phase: hash a present slot with
/// `vibe-registry`'s content hasher and compare against the hash recorded
/// for the resolved package in the fetched set. A package the resolution
/// did not fetch (or whose slot cannot be hashed) reports
/// [`SlotCheck::Unverifiable`] and falls back to the re-copy discipline.
pub(crate) struct RegistrySlotVerifier {
    /// The expected `content_hash` per resolved `(group, name)` — the
    /// fetch's own hash, i.e. the value the next `vibe.lock` pins.
    expected: HashMap<(Group, String), String>,
}

impl RegistrySlotVerifier {
    /// Build the expected-hash table from the resolution's fetched set.
    pub(crate) fn from_fetched(fetched: &[Fetched]) -> Self {
        Self {
            expected: fetched
                .iter()
                .map(|f| {
                    (
                        (
                            f.cached.resolved.group.clone(),
                            f.cached.resolved.name.clone(),
                        ),
                        f.cached.content_hash.clone(),
                    )
                })
                .collect(),
        }
    }
}

impl SlotVerifier for RegistrySlotVerifier {
    fn source_hash<'a>(&'a self, dep: &ResolvedDep) -> Option<&'a str> {
        self.expected
            .get(&(dep.group.clone(), dep.name.clone()))
            .map(String::as_str)
    }

    fn verify_slot(&self, dep: &ResolvedDep, slot_abs: &Path) -> SlotCheck {
        let Some(expected) = self.expected.get(&(dep.group.clone(), dep.name.clone())) else {
            return SlotCheck::Unverifiable;
        };
        // Dispatch on the pin's recipe label (PROP-044 §4.7): the value
        // says how it was computed, so the slot is hashed the same way.
        // Recipe 0 (`sha256:`) is the registry default and the form every
        // lockfile in existence carries; `sha256-tree/1:` selects the
        // tree recipe.
        let computed = if expected.starts_with(RecipeId::Tree1.label()) {
            compute_content_hash_with(RecipeId::Tree1, slot_abs)
        } else {
            compute_content_hash(slot_abs)
        };
        match computed {
            Ok(actual) if actual == *expected => SlotCheck::Verified,
            Ok(actual) => SlotCheck::Diverged {
                expected: expected.clone(),
                actual,
            },
            // An unhashable slot (locked file, permission denied) cannot be
            // vouched for — the re-copy both repairs and supersedes it.
            Err(_) => SlotCheck::Unverifiable,
        }
    }

    fn verify_slot_for_format(
        &self,
        dep: &ResolvedDep,
        slot_abs: &Path,
        spec_format: SpecFormat,
    ) -> SlotCheck {
        if spec_format == SpecFormat::Mixed {
            return self.verify_slot(dep, slot_abs);
        }
        let Some(expected_source) = self.source_hash(dep) else {
            return SlotCheck::Unverifiable;
        };
        let manifest = match vibe_workspace::vibedeps::read_derived_manifest(slot_abs) {
            Ok(manifest) => manifest,
            Err(reason) => {
                return SlotCheck::DivergedDetail {
                    reason: format!("derived manifest is invalid: {reason}"),
                };
            }
        };
        if manifest.source_hash != expected_source {
            return SlotCheck::DivergedDetail {
                reason: format!(
                    "derived manifest source_hash is {}, fetched source_hash is {expected_source}",
                    manifest.source_hash
                ),
            };
        }
        if manifest.output_format != spec_format {
            return SlotCheck::DivergedDetail {
                reason: format!(
                    "derived manifest output_format is {}, effective format is {}",
                    manifest.output_format.as_str(),
                    spec_format.as_str()
                ),
            };
        }
        if manifest.converter_recipe != vibe_workspace::vibedeps::CONVERTER_RECIPE {
            return SlotCheck::DivergedDetail {
                reason: format!(
                    "derived manifest converter_recipe is {}, current recipe is {}",
                    manifest.converter_recipe,
                    vibe_workspace::vibedeps::CONVERTER_RECIPE
                ),
            };
        }
        let live_overlay_hash = live_overlay_hash(slot_abs);
        if manifest.overlay_hash != live_overlay_hash {
            return SlotCheck::DivergedDetail {
                reason: format!(
                    "derived manifest overlay_hash is {}, live package overlay hashes to {}",
                    option_hash(&manifest.overlay_hash),
                    option_hash(&live_overlay_hash)
                ),
            };
        }
        match vibe_workspace::vibedeps::compute_derived_hash(slot_abs) {
            Ok(actual) if actual == manifest.derived_hash => SlotCheck::Verified,
            Ok(actual) => SlotCheck::DivergedDetail {
                reason: format!(
                    "derived_hash mismatch: manifest has {}, slot hashes to {actual}",
                    manifest.derived_hash
                ),
            },
            Err(reason) => SlotCheck::DivergedDetail {
                reason: format!("derived_hash cannot be computed: {reason}"),
            },
        }
    }
}

fn live_overlay_hash(slot: &Path) -> Option<String> {
    let package_dir = slot.parent()?;
    // The slot sits under the layout's dependency root — which may be
    // MULTI-component (`vibevm/vibedeps`, PROP-052): strip every component
    // of `current_vibedeps_root()` from the tail to reach the project root.
    let mut cursor = package_dir.parent()?;
    let deps_root = vibe_core::layout::current_vibedeps_root();
    let mut components: Vec<_> = deps_root
        .components()
        .filter_map(|c| c.as_os_str().to_str().map(str::to_owned))
        .collect();
    while let Some(expected) = components.pop() {
        if cursor.file_name()?.to_str()? != expected {
            return None;
        }
        cursor = cursor.parent()?;
    }
    let project_root = cursor;
    let package_key = package_dir.file_name()?.to_str()?;
    vibe_facts::overlay_file_hash(project_root, package_key)
}

fn option_hash(hash: &Option<String>) -> &str {
    hash.as_deref().unwrap_or("<none>")
}
