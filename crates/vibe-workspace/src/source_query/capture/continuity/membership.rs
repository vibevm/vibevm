//! Independent original-tree membership, kept apart from slot-record agreement.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-070#CAPTURED-SOURCE-MEMBERSHIP");

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};
use vibe_core::manifest::{Manifest, SpecFormat};
use vibe_core::{ContentHash, PackageRef};
use vibe_safefs::{StableFileSnapshot, split_relative};

use super::{CaptureOwner, SourceQueryUnavailable, WitnessedSourceWorld, read_slot_record_bytes};

/// One original package file observed by the trusted preparation owner. It is
/// not a slot output and cannot authenticate itself by its own local digest.
#[derive(Debug, Clone)]
pub struct CapturedSourceFile {
    relative_path: String,
    snapshot: StableFileSnapshot,
}

impl CapturedSourceFile {
    pub fn new(
        relative_path: String,
        snapshot: StableFileSnapshot,
    ) -> Result<Self, SourceQueryUnavailable> {
        split_relative(&relative_path)
            .map_err(|_| SourceQueryUnavailable::SourceProofUnavailable("source path"))?;
        if snapshot.size != snapshot.bytes.len() as u64
            || snapshot.sha256 != format!("{:x}", Sha256::digest(&snapshot.bytes))
        {
            return Err(SourceQueryUnavailable::SourceProofUnavailable(
                "source snapshot",
            ));
        }
        Ok(Self {
            relative_path,
            snapshot,
        })
    }

    pub fn relative_path(&self) -> &str {
        &self.relative_path
    }
    pub fn snapshot(&self) -> &StableFileSnapshot {
        &self.snapshot
    }
}

/// Immutable, INERT original-file catalogue. Its byte/digest and manifest
/// shape are checked here, but it carries no selected-hash authority and can
/// never promote a query to source-proven on its own. The install replay owner
/// retains this catalogue inside its private proof-capable wrapper.
#[derive(Debug)]
pub struct SourceMembershipIndex {
    files: BTreeMap<String, StableFileSnapshot>,
}

impl SourceMembershipIndex {
    /// Index captured bytes for later use by the canonical replay owner.
    /// Successful construction is not a source proof.
    pub fn from_captured(
        package_pin: &PackageRef,
        captured: Vec<CapturedSourceFile>,
    ) -> Result<Self, SourceQueryUnavailable> {
        if !package_pin.is_qualified()
            || !package_pin.version.is_exact_pin()
            || package_pin.kind.is_none()
        {
            return Err(SourceQueryUnavailable::SourceProofUnavailable(
                "package pin",
            ));
        }
        let mut files = BTreeMap::new();
        for file in captured {
            if files.insert(file.relative_path, file.snapshot).is_some() {
                return Err(SourceQueryUnavailable::SourceProofUnavailable(
                    "duplicate source path",
                ));
            }
        }
        let manifest =
            files
                .get(Manifest::FILENAME)
                .ok_or(SourceQueryUnavailable::SourceProofUnavailable(
                    "original manifest",
                ))?;
        let text = std::str::from_utf8(&manifest.bytes)
            .map_err(|_| SourceQueryUnavailable::SourceProofUnavailable("original manifest"))?;
        let parsed = Manifest::parse_str(text)
            .map_err(|_| SourceQueryUnavailable::SourceProofUnavailable("original manifest"))?;
        let authored_pin = parsed
            .require_package()
            .and_then(|package| package.as_package_ref())
            .map_err(|_| SourceQueryUnavailable::SourceProofUnavailable("original package pin"))?;
        if &authored_pin != package_pin {
            return Err(SourceQueryUnavailable::SourceProofUnavailable(
                "original package pin",
            ));
        }
        Ok(Self { files })
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }
    pub fn contains(&self, relative: &str) -> bool {
        self.files.contains_key(relative)
    }

    pub fn member_matches(&self, relative: &str, observed: &StableFileSnapshot) -> bool {
        self.files.get(relative).is_some_and(|source| {
            source.bytes == observed.bytes && source.sha256 == observed.sha256
        })
    }
}

/// Strength of the RETURNED materialized output. A transformed output may
/// have a separately proven original member but needs C3 derivation before its
/// own bytes can be called source-proven.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-070#SOURCE-STRENGTH-LAW")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceStrength {
    Observed,
    RecordConsistent,
    SourceProven,
}

/// Locally agreed selected slot row and held output. It establishes only
/// record consistency; the install replay owner may separately promote an
/// original Mixed source member after checking its private verified index.
pub struct RecordedOutput<'a> {
    pub snapshot: &'a StableFileSnapshot,
    pub source_path: &'a str,
    pub format: SpecFormat,
}

impl WitnessedSourceWorld {
    /// The selected lock pin and independent source hash, after B2 continuity.
    pub fn selected_source_pin(
        &self,
        group: &str,
        name: &str,
    ) -> Result<(PackageRef, ContentHash), SourceQueryUnavailable> {
        self.revalidate_current()?;
        let key = (group.to_owned(), name.to_owned());
        let selected =
            self.lock_rows
                .get(&key)
                .ok_or(SourceQueryUnavailable::SourceProofUnavailable(
                    "selected lock row",
                ))?;
        let pin = selected
            .as_package_ref()
            .map_err(|_| SourceQueryUnavailable::SourceProofUnavailable("selected package pin"))?;
        Ok((pin, selected.content_hash.clone()))
    }

    /// Finite local record/output agreement, without any original-source
    /// authentication. The returned borrow is valid for this prepared world;
    /// a later namespace change is outside the completed B2 check window.
    pub fn recorded_output(
        &self,
        group: &str,
        name: &str,
        output: &str,
    ) -> Result<Option<RecordedOutput<'_>>, SourceQueryUnavailable> {
        let prepared = self.revalidate_current()?;
        let observed = prepared.captured_output(Some((group, name)), output)?;
        let key = (group.to_owned(), name.to_owned());
        let selected =
            self.lock_rows
                .get(&key)
                .ok_or(SourceQueryUnavailable::SourceProofUnavailable(
                    "selected lock row",
                ))?;
        if selected.materialization.is_in_place() {
            return Ok(None);
        }
        let capture = prepared
            .captured
            .get(&CaptureOwner::Dependency {
                group: key.0.clone(),
                name: key.1.clone(),
            })
            .ok_or(SourceQueryUnavailable::SourceProofUnavailable(
                "selected output",
            ))?;
        let Some(map) = &capture.descriptor else {
            return Ok(None);
        };
        let row = map
            .output_row(output)
            .ok_or(SourceQueryUnavailable::SourceProofUnavailable(
                "recorded output",
            ))?;
        let record_snapshot =
            self.slot_records
                .get(&key)
                .ok_or(SourceQueryUnavailable::SourceProofUnavailable(
                    "selected slot record",
                ))?;
        let record = read_slot_record_bytes(
            &capture.root.join(crate::vibedeps::SLOT_RECORD_FILENAME),
            &record_snapshot.bytes,
        )
        .map_err(|_| SourceQueryUnavailable::SourceProofUnavailable("selected slot record"))?;
        let recorded = record.files.iter().find(|file| file.path == output).ok_or(
            SourceQueryUnavailable::SourceProofUnavailable("recorded output"),
        )?;
        if recorded.sha256 != observed.sha256 || row.output_sha256 != observed.sha256 {
            return Err(SourceQueryUnavailable::SourceProofUnavailable(
                "recorded payload",
            ));
        }
        Ok(Some(RecordedOutput {
            snapshot: observed,
            source_path: &row.source,
            format: record.spec_format,
        }))
    }

    /// Public workspace route is deliberately local-strength only. No raw
    /// catalogue or caller-provided digest can promote its answer.
    pub fn output_strength(
        &self,
        owner: Option<(&str, &str)>,
        output: &str,
    ) -> Result<SourceStrength, SourceQueryUnavailable> {
        self.revalidate_current()?.captured_output(owner, output)?;
        let Some((group, name)) = owner else {
            return Ok(SourceStrength::Observed);
        };
        Ok(if self.recorded_output(group, name, output)?.is_some() {
            SourceStrength::RecordConsistent
        } else {
            SourceStrength::Observed
        })
    }
}

#[cfg(test)]
#[path = "m_01_c_2_cases.rs"]
mod m_01_c_2_cases;
