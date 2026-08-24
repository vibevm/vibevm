//! The `slot_integrity = verify` spot-check (PROP-011 §2.3/§5.2): before
//! the §2.3 fast path trusts a present slot, validate its slot record and
//! recorded payload against the `content_hash` the resolution carries — the lockfile pin on the
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
//! hasher. Slots predating `.vibe-slot.toml` retain the registry tree-hash
//! path, including recipe dispatch for both historical wire forms.

use std::collections::HashMap;
use std::path::Path;

use vibe_core::manifest::SpecFormat;
use vibe_core::{ContentHash, Group};
use vibe_registry::{RecipeId, compute_content_hash, compute_content_hash_with};
use vibe_workspace::install::{ResolvedDep, SlotCheck, SlotVerifier};

use crate::fetched::Fetched;

/// The [`SlotVerifier`] for the apply phase: prefer the typed slot record and
/// its exact payload list, falling back to historical hash recipes only when
/// no new record exists. A package the resolution did not fetch reports
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

    fn verify_legacy_mixed(&self, dep: &ResolvedDep, slot_abs: &Path) -> SlotCheck {
        let Some(expected) = self.expected.get(&(dep.group.clone(), dep.name.clone())) else {
            return SlotCheck::Unverifiable;
        };
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
            Err(_) => SlotCheck::Unverifiable,
        }
    }

    fn verify_record(
        &self,
        dep: &ResolvedDep,
        slot_abs: &Path,
        spec_format: SpecFormat,
    ) -> SlotCheck {
        let record = match vibe_workspace::vibedeps::read_slot_record(slot_abs) {
            Ok(record) => record,
            Err(reason) => {
                return SlotCheck::DivergedDetail {
                    reason: format!("slot record is invalid: {reason}"),
                };
            }
        };
        let Some(expected_source) = self.source_hash(dep) else {
            return SlotCheck::Unverifiable;
        };
        if record.source_hash.as_str() != expected_source {
            return SlotCheck::DivergedDetail {
                reason: format!(
                    "slot record source_hash is {}, fetched source_hash is {expected_source}",
                    record.source_hash.as_str()
                ),
            };
        }
        if record.spec_format != spec_format {
            return SlotCheck::DivergedDetail {
                reason: format!(
                    "slot record spec_format is {}, effective format is {}",
                    record.spec_format.as_str(),
                    spec_format.as_str()
                ),
            };
        }
        if spec_format.is_transformed() {
            if record.converter_recipe.as_deref()
                != Some(vibe_workspace::vibedeps::CONVERTER_RECIPE)
            {
                return SlotCheck::DivergedDetail {
                    reason: format!(
                        "slot record converter_recipe is {}, current recipe is {}",
                        record.converter_recipe.as_deref().unwrap_or("<none>"),
                        vibe_workspace::vibedeps::CONVERTER_RECIPE
                    ),
                };
            }
            let live_overlay_hash = live_overlay_hash(slot_abs);
            if record.overlay_hash.as_ref().map(ContentHash::as_str) != live_overlay_hash.as_deref()
            {
                return SlotCheck::DivergedDetail {
                    reason: format!(
                        "slot record overlay_hash is {}, live package overlay hashes to {}",
                        option_content_hash(&record.overlay_hash),
                        option_hash(&live_overlay_hash)
                    ),
                };
            }
        }
        if let Err(reason) = vibe_workspace::vibedeps::verify_recorded_files(slot_abs, &record) {
            return SlotCheck::DivergedDetail {
                reason: format!("slot record payload is invalid: {reason}"),
            };
        }
        if spec_format.is_transformed() {
            let Some(expected_derived) = record.derived_hash.as_ref() else {
                return SlotCheck::DivergedDetail {
                    reason: "transformed slot record has no derived_hash".to_string(),
                };
            };
            match vibe_workspace::vibedeps::compute_recorded_payload_hash(slot_abs, &record.files) {
                Ok(actual) if &actual == expected_derived => {}
                Ok(actual) => {
                    return SlotCheck::DivergedDetail {
                        reason: format!(
                            "derived_hash mismatch: record has {}, payload hashes to {}",
                            expected_derived.as_str(),
                            actual.as_str()
                        ),
                    };
                }
                Err(reason) => {
                    return SlotCheck::DivergedDetail {
                        reason: format!("derived_hash cannot be computed: {reason}"),
                    };
                }
            }
        }
        SlotCheck::Verified
    }

    fn verify_legacy_derived(
        &self,
        dep: &ResolvedDep,
        slot_abs: &Path,
        spec_format: SpecFormat,
    ) -> SlotCheck {
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

impl SlotVerifier for RegistrySlotVerifier {
    fn source_hash<'a>(&'a self, dep: &ResolvedDep) -> Option<&'a str> {
        self.expected
            .get(&(dep.group.clone(), dep.name.clone()))
            .map(String::as_str)
    }

    fn verify_slot(&self, dep: &ResolvedDep, slot_abs: &Path) -> SlotCheck {
        self.verify_slot_for_format(dep, slot_abs, SpecFormat::Mixed)
    }

    fn verify_slot_for_format(
        &self,
        dep: &ResolvedDep,
        slot_abs: &Path,
        spec_format: SpecFormat,
    ) -> SlotCheck {
        let record_path = slot_abs.join(vibe_workspace::vibedeps::SLOT_RECORD_FILENAME);
        match std::fs::symlink_metadata(&record_path) {
            Ok(_) => self.verify_record(dep, slot_abs, spec_format),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => match spec_format {
                SpecFormat::Mixed => self.verify_legacy_mixed(dep, slot_abs),
                SpecFormat::Markdown | SpecFormat::Xml => {
                    self.verify_legacy_derived(dep, slot_abs, spec_format)
                }
            },
            Err(error) => SlotCheck::DivergedDetail {
                reason: format!("slot record cannot be inspected: {error}"),
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

fn option_content_hash(hash: &Option<ContentHash>) -> &str {
    hash.as_ref().map(ContentHash::as_str).unwrap_or("<none>")
}
