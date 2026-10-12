//! Independent original-tree membership, kept apart from slot-record agreement.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-070#CAPTURED-SOURCE-MEMBERSHIP");

use std::collections::BTreeMap;

use crate::vibedeps::SlotFileDisposition;
use sha2::{Digest, Sha256};
use vibe_core::manifest::{Manifest, SelectedSource, SourceInterpretation, SpecFormat};
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

    /// Captured ORIGINAL bytes; this inert accessor grants no proof by itself.
    /// Only the install replay owner retains this index in its private seal.
    pub fn snapshot(&self, relative: &str) -> Option<&StableFileSnapshot> {
        self.files.get(relative)
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

/// Parser media of the selected OUTPUT, distinct from whole-slot
/// materialization mode (`SpecFormat::Mixed` may hold either form).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterializedVibeMedia {
    Markdown,
    Xml,
}

/// Locally agreed selected slot row and held output. It establishes only
/// record consistency; the install replay owner may separately promote an
/// original Mixed source member after checking its private verified index.
pub struct RecordedOutput<'a> {
    pub slot_root: &'a std::path::Path,
    pub snapshot: &'a StableFileSnapshot,
    pub source_path: &'a str,
    pub slot_format: SpecFormat,
    pub media: Option<MaterializedVibeMedia>,
    pub interpretation: Option<SourceInterpretation>,
    pub converter_recipe: Option<String>,
    pub overlay_hash: Option<ContentHash>,
    pub disposition: Option<SlotFileDisposition>,
}

impl WitnessedSourceWorld {
    /// Resolve one source-neutral path only inside this selected descriptor
    /// map. No caller-supplied visible set or directory search widens it.
    pub fn selected_document_output(
        &self,
        group: &str,
        name: &str,
        doc_path: &str,
    ) -> Result<(String, RecordedOutput<'_>), SourceQueryUnavailable> {
        let prepared = self.revalidate_current()?;
        let capture = prepared
            .captured
            .get(&CaptureOwner::Dependency {
                group: group.to_owned(),
                name: name.to_owned(),
            })
            .ok_or(SourceQueryUnavailable::DerivationUnavailable(
                "selected document owner",
            ))?;
        let map =
            capture
                .descriptor
                .as_ref()
                .ok_or(SourceQueryUnavailable::DerivationUnavailable(
                    "selected descriptor map",
                ))?;
        let mut selected = capture.outputs.keys().filter(|output| {
            map.output_row(output).is_some_and(|row| {
                matches!(
                    row.interpretation,
                    Some(SourceInterpretation::VibeSpec | SourceInterpretation::VibeDoc)
                ) && vibe_spec::canonical_doc_path(&row.output) == doc_path
            })
        });
        let output = selected
            .next()
            .ok_or(SourceQueryUnavailable::DerivationUnavailable(
                "materialized document address",
            ))?
            .clone();
        if selected.next().is_some() {
            return Err(SourceQueryUnavailable::DerivationUnavailable(
                "ambiguous materialized document address",
            ));
        }
        let recorded = self.recorded_output(group, name, &output)?.ok_or(
            SourceQueryUnavailable::DerivationUnavailable("selected materialized document"),
        )?;
        Ok((output, recorded))
    }

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
        let manifest_text = std::str::from_utf8(&capture.manifest.bytes).map_err(|_| {
            SourceQueryUnavailable::DerivationUnavailable("selected descriptor manifest")
        })?;
        let manifest = Manifest::parse_str(manifest_text).map_err(|_| {
            SourceQueryUnavailable::DerivationUnavailable("selected descriptor manifest")
        })?;
        let package = manifest.require_package().map_err(|_| {
            SourceQueryUnavailable::DerivationUnavailable("selected descriptor package")
        })?;
        let selected = package.select_source(&row.source).map_err(|_| {
            SourceQueryUnavailable::DerivationUnavailable("selected source descriptor")
        })?;
        let source_media = match selected {
            SelectedSource::Descriptor { descriptor, .. }
                if Some(descriptor.interpretation()) == row.interpretation =>
            {
                match descriptor.media() {
                    "text/markdown" => Some(MaterializedVibeMedia::Markdown),
                    "application/xml" => Some(MaterializedVibeMedia::Xml),
                    _ => None,
                }
            }
            _ => None,
        };
        let media = if !matches!(
            row.interpretation,
            Some(SourceInterpretation::VibeSpec | SourceInterpretation::VibeDoc)
        ) {
            None
        } else if recorded.disposition == Some(SlotFileDisposition::Converted) {
            match record.spec_format {
                SpecFormat::Markdown => Some(MaterializedVibeMedia::Markdown),
                SpecFormat::Xml => Some(MaterializedVibeMedia::Xml),
                SpecFormat::Mixed => None,
            }
        } else {
            // Mixed and copied transformed rows retain the authored media.
            source_media
        };
        Ok(Some(RecordedOutput {
            slot_root: &capture.root,
            snapshot: observed,
            source_path: &row.source,
            slot_format: record.spec_format,
            media,
            interpretation: row.interpretation,
            converter_recipe: record.converter_recipe,
            overlay_hash: record.overlay_hash,
            disposition: recorded.disposition.clone(),
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

/// Semantic identity of one selected node. Its recipe and digest are
/// separately typed so a raw output SHA cannot stand in for this value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalNodeIdentity {
    pub recipe: &'static str,
    pub hash: String,
}

/// Pure semantic projection of the exact selected output observation. It
/// never reads a path or reinterprets the source-span `contentHash`.
pub fn canonical_node_of_output(
    recorded: &RecordedOutput<'_>,
    anchor: &str,
) -> Result<Option<CanonicalNodeIdentity>, SourceQueryUnavailable> {
    let text = std::str::from_utf8(&recorded.snapshot.bytes).map_err(|_| {
        SourceQueryUnavailable::DerivationUnavailable("materialized document UTF-8")
    })?;
    let doc = match (recorded.media, recorded.interpretation) {
        (Some(MaterializedVibeMedia::Xml), Some(SourceInterpretation::VibeDoc)) => {
            vibe_specdoc::from_xml_with(text, vibe_specdoc::doc::Vocabulary::Doc)
        }
        (Some(MaterializedVibeMedia::Xml), Some(SourceInterpretation::VibeSpec)) => {
            vibe_specdoc::from_xml_with(text, vibe_specdoc::doc::Vocabulary::Spec)
        }
        (Some(MaterializedVibeMedia::Markdown), Some(SourceInterpretation::VibeSpec)) => {
            vibe_specdoc::from_markdown(text)
        }
        _ => {
            return Err(SourceQueryUnavailable::DerivationUnavailable(
                "selected Vibe output",
            ));
        }
    }
    .map_err(|_| SourceQueryUnavailable::DerivationUnavailable("materialized SpecDoc IR"))?;
    let recipe = vibe_specdoc::canonical_node::NODE_HASH_RECIPE;
    vibe_specdoc::canonical_node::canonical_node_hash(&doc, anchor)
        .map_err(|_| {
            SourceQueryUnavailable::DerivationUnavailable("ambiguous materialized anchor")
        })?
        .map(|value| {
            value
                .strip_prefix(&format!("{recipe}:"))
                .map(|digest| CanonicalNodeIdentity {
                    recipe,
                    hash: format!("sha256:{digest}"),
                })
                .ok_or(SourceQueryUnavailable::DerivationUnavailable(
                    "canonical hash recipe",
                ))
        })
        .transpose()
}

/// Native XML range under the same selected Spec/Doc vocabulary used for
/// semantic parsing. Doc-only blocks are valid only for selected VibeDoc.
pub fn native_xml_range_of_output(
    recorded: &RecordedOutput<'_>,
    anchor: &str,
) -> Result<Option<(u32, u32)>, SourceQueryUnavailable> {
    let vocabulary = match (recorded.media, recorded.interpretation) {
        (Some(MaterializedVibeMedia::Xml), Some(SourceInterpretation::VibeSpec)) => {
            vibe_specdoc::doc::Vocabulary::Spec
        }
        (Some(MaterializedVibeMedia::Xml), Some(SourceInterpretation::VibeDoc)) => {
            vibe_specdoc::doc::Vocabulary::Doc
        }
        _ => {
            return Err(SourceQueryUnavailable::DerivationUnavailable(
                "selected XML vocabulary",
            ));
        }
    };
    let text = std::str::from_utf8(&recorded.snapshot.bytes).map_err(|_| {
        SourceQueryUnavailable::DerivationUnavailable("materialized document UTF-8")
    })?;
    vibe_specdoc::native_anchor_range_with(text, vocabulary, anchor)
        .map_err(|_| SourceQueryUnavailable::DerivationUnavailable("materialized native XML range"))
}

#[cfg(test)]
#[path = "m_01_c_2_cases.rs"]
mod m_01_c_2_cases;
