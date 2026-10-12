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
use vibe_core::{ContentHash, Group, PackageRef};
use vibe_registry::{
    BoundedContentFile, RecipeId, compute_content_hash, compute_content_hash_legacy0_bounded_exact,
    compute_content_hash_with,
};
use vibe_workspace::install::{ResolvedDep, SlotCheck, SlotVerifier};
use vibe_workspace::source_query::{
    CapturedSourceFile, SourceMembershipIndex, SourceQueryUnavailable, SourceStrength,
    WitnessedSourceWorld,
};

use crate::fetched::Fetched;

/// Replay a complete, caller-captured ORIGINAL package tree under the
/// incumbent bounded recipe, then join it to the independently selected hash.
/// The caller obtains `package_pin` and `selected_hash` from its trusted
/// selected world, never from the editable slot record. No provider or cache
/// port exists here; unsupported recipes and missing capture refuse offline.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-070#CAPTURED-SOURCE-MEMBERSHIP"
)]
pub fn verify_original_source_capture(
    root: &Path,
    package_pin: PackageRef,
    selected_hash: ContentHash,
    captured: Vec<CapturedSourceFile>,
) -> Result<VerifiedSourceMembership, SourceQueryUnavailable> {
    if !selected_hash
        .as_str()
        .starts_with(RecipeId::Legacy0.label())
    {
        return Err(SourceQueryUnavailable::SourceProofUnavailable(
            "unsupported source hash recipe",
        ));
    }
    let mut rows = captured
        .iter()
        .map(|file| BoundedContentFile {
            relative_path: file.relative_path().to_owned(),
            bytes: file.snapshot().size,
            sha256: format!("sha256:{}", file.snapshot().sha256),
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    let replayed =
        compute_content_hash_legacy0_bounded_exact(root, &package_pin, &rows).map_err(|_| {
            SourceQueryUnavailable::SourceProofUnavailable("bounded original capture replay")
        })?;
    if replayed != selected_hash {
        return Err(SourceQueryUnavailable::SourceHashMismatch);
    }
    let index = SourceMembershipIndex::from_captured(&package_pin, captured)?;
    Ok(VerifiedSourceMembership {
        package_pin,
        selected_hash,
        index,
    })
}

/// Proof-capable original capture. Only canonical bounded replay above can
/// construct this value; its fields and constructor are private to this owner.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-070#CAPTURED-SOURCE-MEMBERSHIP"
)]
pub struct VerifiedSourceMembership {
    package_pin: PackageRef,
    selected_hash: ContentHash,
    index: SourceMembershipIndex,
}

impl VerifiedSourceMembership {
    pub fn file_count(&self) -> usize {
        self.index.file_count()
    }
    pub fn contains(&self, path: &str) -> bool {
        self.index.contains(path)
    }

    /// Selected ORIGINAL membership, independent of the installed slot's
    /// mutable record and without claiming transformed output derivation.
    pub fn original_source_member(
        &self,
        world: &WitnessedSourceWorld,
        group: &str,
        name: &str,
        path: &str,
    ) -> Result<bool, SourceQueryUnavailable> {
        let (pin, hash) = world.selected_source_pin(group, name)?;
        if pin != self.package_pin || hash != self.selected_hash {
            return Err(SourceQueryUnavailable::SourceProofUnavailable(
                "selected original capture",
            ));
        }
        Ok(self.index.contains(path))
    }

    fn promote_output(
        &self,
        world: &WitnessedSourceWorld,
        group: &str,
        name: &str,
        output: &str,
        strict: bool,
        local: SourceStrength,
    ) -> Result<SourceStrength, SourceQueryUnavailable> {
        let (pin, hash) = world.selected_source_pin(group, name)?;
        let recorded = world.recorded_output(group, name, output)?;
        if pin == self.package_pin
            && hash == self.selected_hash
            && recorded.as_ref().is_some_and(|row| {
                row.format == SpecFormat::Mixed
                    && row.source_path == output
                    && self.index.member_matches(row.source_path, row.snapshot)
            })
        {
            return Ok(SourceStrength::SourceProven);
        }
        if strict {
            Err(SourceQueryUnavailable::SourceProofUnavailable(
                "original source membership or derivation",
            ))
        } else {
            Ok(local)
        }
    }
}

/// The only public strength route that may return `SourceProven`. A raw
/// workspace catalogue stays inert; missing proof refuses strict requests
/// offline, with no provider/cache/rebuild port.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-070#SOURCE-STRENGTH-LAW")]
pub fn source_strength(
    world: &WitnessedSourceWorld,
    owner: Option<(&str, &str)>,
    output: &str,
    proof: Option<&VerifiedSourceMembership>,
    strict: bool,
) -> Result<SourceStrength, SourceQueryUnavailable> {
    let local = world.output_strength(owner, output)?;
    let Some((group, name)) = owner else {
        return if strict {
            Err(SourceQueryUnavailable::SourceProofUnavailable(
                "development source capture",
            ))
        } else {
            Ok(local)
        };
    };
    let Some(proof) = proof else {
        return if strict {
            Err(SourceQueryUnavailable::SourceProofUnavailable(
                "original source capture",
            ))
        } else {
            Ok(local)
        };
    };
    proof.promote_output(world, group, name, output, strict, local)
}

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

#[cfg(test)]
mod source_membership_cases {
    use std::fs;

    use tempfile::TempDir;
    use vibe_safefs::Project;

    use super::*;

    fn fixture() -> (
        TempDir,
        vibe_core::PackageRef,
        ContentHash,
        Vec<CapturedSourceFile>,
    ) {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("spec")).unwrap();
        fs::write(
            root.path().join("vibe.toml"),
            "[package]\ngroup='org.demo'\nname='dep'\nkind='flow'\nversion='1.0.0'\n",
        )
        .unwrap();
        fs::write(
            root.path().join("spec/DOC.md"),
            b"# Original\nSOURCE_MEMBER_C2\n",
        )
        .unwrap();
        let pin = vibe_core::PackageRef::parse("flow:org.demo/dep@=1.0.0").unwrap();
        // Selected expected value is pinned once from the pristine original,
        // before any candidate slot/source mutation below.
        let expected = ContentHash::parse(&compute_content_hash(root.path()).unwrap()).unwrap();
        let project = Project::open(root.path()).unwrap();
        let captured = ["spec/DOC.md", "vibe.toml"]
            .into_iter()
            .map(|relative| {
                CapturedSourceFile::new(
                    relative.to_owned(),
                    project
                        .read_file_snapshot_bounded(relative, 1024)
                        .unwrap()
                        .unwrap(),
                )
                .unwrap()
            })
            .collect();
        (root, pin, expected, captured)
    }

    #[test]
    fn verified_original_capture_seals_exact_members_only() {
        let (root, pin, expected, captured) = fixture();
        let cache = root.path().join("must-stay-absent-cache");
        let index = verify_original_source_capture(root.path(), pin, expected, captured).unwrap();
        assert_eq!(index.file_count(), 2);
        assert!(index.contains("spec/DOC.md"));
        assert!(!index.contains("spec/UNRECORDED.md"));
        assert!(!cache.exists());
    }

    #[test]
    fn changed_source_or_wrong_independent_pin_cannot_seal() {
        let (root, pin, expected, captured) = fixture();
        let wrong = ContentHash::parse(&format!("sha256:{}", "0".repeat(64))).unwrap();
        assert!(matches!(
            verify_original_source_capture(root.path(), pin.clone(), wrong, captured.clone()),
            Err(SourceQueryUnavailable::SourceHashMismatch)
        ));
        fs::write(root.path().join("spec/DOC.md"), b"# Changed\n").unwrap();
        assert!(matches!(
            verify_original_source_capture(root.path(), pin, expected, captured),
            Err(SourceQueryUnavailable::SourceProofUnavailable(
                "bounded original capture replay"
            ))
        ));
    }

    #[test]
    fn unsupported_recipe_refuses_offline() {
        let (root, pin, _expected, captured) = fixture();
        let tree1 = ContentHash::parse(&format!("sha256-tree/1:{}", "a".repeat(64))).unwrap();
        assert!(matches!(
            verify_original_source_capture(root.path(), pin, tree1, captured),
            Err(SourceQueryUnavailable::SourceProofUnavailable(
                "unsupported source hash recipe"
            ))
        ));
    }
}

#[cfg(test)]
#[path = "slot_verify/m_01_c_2_cases.rs"]
mod m_01_c_2_cases;
