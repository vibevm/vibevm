//! Read a descriptor-owned installed slot into a resolver map without treating
//! filename suffixes as source interpretation.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-REPRESENTATION-COLLISION");

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use vibe_core::layout;
use vibe_core::manifest::{Manifest, SelectedSource, SourceInterpretation, SpecFormat};
use vibe_core::{ContentHash, Group};
use vibe_safefs::Project;
use vibe_spec::{DescriptorOutputRow, ValidatedDescriptorMap, canonical_doc_path};

use super::slot_record::{SLOT_RECORD_FILENAME, SlotFileDisposition, read_slot_record_bytes};

const MANIFEST_CAP: usize = 1024 * 1024;
const RECORD_CAP: usize = 8 * 1024 * 1024;
const OUTPUT_CAP: u64 = 64 * 1024 * 1024;
const MAX_SELECTED_OUTPUTS: usize = 4096;
const MAX_TOTAL_OUTPUT_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Clone, Copy)]
pub(super) struct DescriptorMapLimits {
    pub max_outputs: usize,
    pub max_total_bytes: u64,
}

/// `None` is a proved no-descriptor package and retains the legacy resolver
/// route. A descriptor-owned slot lacking any record or witness always refuses.
/// The source witness is package-wide: converted original-file SHA values are
/// not retained by the current slot record and are not claimed here.
pub fn read_validated_descriptor_map(
    slot: &Path,
    group: &Group,
    name: &str,
    version: &semver::Version,
    expected_source_hash: &ContentHash,
    expected_format: SpecFormat,
) -> Result<Option<ValidatedDescriptorMap>, String> {
    read_validated_descriptor_map_optional_hash(
        slot,
        group,
        name,
        version,
        Some(expected_source_hash),
        expected_format,
    )
}

/// Boot-only reconstruction may have no independently selected source hash.
/// That is acceptable for a legacy package, but a descriptor-owned slot must
/// refuse rather than treating its own record as the expected witness.
pub fn read_validated_descriptor_map_optional_hash(
    slot: &Path,
    group: &Group,
    name: &str,
    version: &semver::Version,
    expected_source_hash: Option<&ContentHash>,
    expected_format: SpecFormat,
) -> Result<Option<ValidatedDescriptorMap>, String> {
    read_validated_descriptor_map_with_limits(
        slot,
        group,
        name,
        version,
        expected_source_hash,
        expected_format,
        DescriptorMapLimits {
            max_outputs: MAX_SELECTED_OUTPUTS,
            max_total_bytes: MAX_TOTAL_OUTPUT_BYTES,
        },
    )
}

/// Citation lookup has an independent lock hash and version, but the lock
/// does not pin the target representation. A copied slot must retain a record
/// even when its current manifest has no descriptor rows; an in-place legacy
/// source may have no record and retains that weaker compatibility route.
pub fn read_validated_descriptor_map_for_citation(
    slot: &Path,
    group: &Group,
    name: &str,
    version: &semver::Version,
    expected_source_hash: &ContentHash,
    require_legacy_record: bool,
) -> Result<Option<ValidatedDescriptorMap>, String> {
    read_validated_descriptor_map_with_optional_format(
        slot,
        group,
        name,
        version,
        Some(expected_source_hash),
        None,
        require_legacy_record,
        DescriptorMapLimits {
            max_outputs: MAX_SELECTED_OUTPUTS,
            max_total_bytes: MAX_TOTAL_OUTPUT_BYTES,
        },
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "testable descriptor-only resource ceiling"
)]
pub(super) fn read_validated_descriptor_map_with_limits(
    slot: &Path,
    group: &Group,
    name: &str,
    version: &semver::Version,
    expected_source_hash: Option<&ContentHash>,
    expected_format: SpecFormat,
    limits: DescriptorMapLimits,
) -> Result<Option<ValidatedDescriptorMap>, String> {
    read_validated_descriptor_map_with_optional_format(
        slot,
        group,
        name,
        version,
        expected_source_hash,
        Some(expected_format),
        false,
        limits,
    )
}

#[allow(clippy::too_many_arguments)]
fn read_validated_descriptor_map_with_optional_format(
    slot: &Path,
    group: &Group,
    name: &str,
    version: &semver::Version,
    expected_source_hash: Option<&ContentHash>,
    expected_format: Option<SpecFormat>,
    require_legacy_record: bool,
    limits: DescriptorMapLimits,
) -> Result<Option<ValidatedDescriptorMap>, String> {
    if limits.max_outputs == 0 || limits.max_total_bytes == 0 {
        return Err("descriptor map resource limits must be nonzero".into());
    }
    let project = Project::open(slot).map_err(|error| format!("slot root: {error:#}"))?;
    let manifest = project
        .read_file_snapshot_bounded(Manifest::FILENAME, MANIFEST_CAP)
        .map_err(|error| format!("slot manifest: {error:#}"))?
        .ok_or_else(|| "installed slot has no vibe.toml".to_owned())?;
    let text = std::str::from_utf8(&manifest.bytes)
        .map_err(|_| "installed slot manifest is not UTF-8".to_owned())?;
    let value: toml::Value =
        toml::from_str(text).map_err(|error| format!("installed slot manifest TOML: {error}"))?;
    let authored = value
        .get("package")
        .and_then(toml::Value::as_table)
        .is_some_and(|table| table.contains_key("sources") || table.contains_key("source_exports"));
    let package = Manifest::parse_str(text)
        .map_err(|error| format!("installed slot manifest: {error}"))?
        .package
        .ok_or_else(|| "installed slot requires [package]".to_owned())?;
    if &package.group != group || package.name != name || &package.version != version {
        return Err("installed package differs from selected coordinate".into());
    }
    let record_before = project
        .read_file_snapshot_bounded(SLOT_RECORD_FILENAME, RECORD_CAP)
        .map_err(|error| format!("slot record: {error:#}"))?;
    if !authored && record_before.is_none() && !require_legacy_record {
        let after = project
            .read_file_snapshot_bounded(Manifest::FILENAME, MANIFEST_CAP)
            .map_err(|error| format!("manifest recheck: {error:#}"))?
            .ok_or_else(|| "manifest disappeared during legacy lookup".to_owned())?;
        if manifest != after {
            return Err("manifest changed during legacy lookup".into());
        }
        return Ok(None);
    }
    let record_before = record_before.ok_or_else(|| {
        if authored {
            "descriptor-owned slot has no retained slot record"
        } else {
            "selected copied slot has no retained slot record"
        }
        .to_owned()
    })?;
    let record = read_slot_record_bytes(&slot.join(SLOT_RECORD_FILENAME), &record_before.bytes)?;
    if (authored || require_legacy_record) && expected_source_hash.is_none() {
        return Err("descriptor-owned slot has no independent selected source hash".into());
    }
    if expected_source_hash.is_some_and(|hash| &record.source_hash != hash)
        || expected_format.is_some_and(|format| record.spec_format != format)
    {
        return Err("slot record source hash or format differs from selected package".into());
    }
    let manifest_row = record
        .files
        .iter()
        .find(|row| row.path == Manifest::FILENAME)
        .ok_or_else(|| "slot record has no manifest output row".to_owned())?;
    if manifest_row.sha256 != manifest.sha256 {
        return Err("installed manifest bytes differ from the slot record".into());
    }
    if !authored {
        let record_after = project
            .read_file_snapshot_bounded(SLOT_RECORD_FILENAME, RECORD_CAP)
            .map_err(|error| format!("slot record recheck: {error:#}"))?
            .ok_or_else(|| "slot record disappeared during legacy lookup".to_owned())?;
        let manifest_after = project
            .read_file_snapshot_bounded(Manifest::FILENAME, MANIFEST_CAP)
            .map_err(|error| format!("manifest recheck: {error:#}"))?
            .ok_or_else(|| "manifest disappeared during legacy lookup".to_owned())?;
        if record_before != record_after || manifest != manifest_after {
            return Err("slot record or manifest changed during legacy lookup".into());
        }
        return Ok(None);
    }
    let spec_relative = selected_specs_root(slot)?;
    // Decide the exact finite map from validated record/manifest data before
    // opening any output. Unrelated package payloads consume no map budget.
    let mut selected = Vec::new();
    for file in &record.files {
        let source = if record.spec_format == SpecFormat::Mixed {
            file.path.as_str()
        } else {
            file.source
                .as_deref()
                .ok_or_else(|| format!("transformed output `{}` has no source", file.path))?
        };
        let selection = package
            .select_source(source)
            .map_err(|error| format!("selected source `{source}`: {error:?}"))?;
        let (interpretation, original_media) = match selection {
            SelectedSource::Unclassified { .. } => (None, None),
            SelectedSource::Descriptor { descriptor, .. } => (
                Some(descriptor.interpretation()),
                Some(descriptor.media().to_owned()),
            ),
            SelectedSource::NeedsInput {
                declaration_indices,
                ..
            } => {
                return Err(format!(
                    "selected source `{source}` is ambiguous at declaration rows {declaration_indices:?}"
                ));
            }
        };
        let is_vibe = matches!(
            interpretation,
            Some(SourceInterpretation::VibeSpec | SourceInterpretation::VibeDoc)
        );
        if !Path::new(&file.path).starts_with(&spec_relative) && !is_vibe {
            continue;
        }
        let media = if is_vibe {
            if matches!(&file.disposition, Some(SlotFileDisposition::Converted)) {
                Some(match record.spec_format {
                    SpecFormat::Markdown => "text/markdown".to_owned(),
                    SpecFormat::Xml => "application/xml".to_owned(),
                    SpecFormat::Mixed => return Err("mixed copied output claims conversion".into()),
                })
            } else {
                original_media
            }
        } else {
            None
        };
        selected.push((file, source.to_owned(), interpretation, media));
        if selected.len() > limits.max_outputs {
            return Err(format!(
                "descriptor map has more than {} selected outputs",
                limits.max_outputs
            ));
        }
    }
    let mut rows = Vec::with_capacity(selected.len());
    let mut materialized_media = BTreeMap::new();
    let mut observed_outputs = Vec::with_capacity(selected.len());
    let mut remaining = limits.max_total_bytes;
    for (file, source, interpretation, media) in selected {
        if remaining == 0 {
            return Err(format!(
                "descriptor map exceeds {} total output bytes",
                limits.max_total_bytes
            ));
        }
        let state = project
            .stable_file_state_bounded(&file.path, OUTPUT_CAP.min(remaining))
            .map_err(|error| format!("recorded output `{}`: {error:#}", file.path))?
            .ok_or_else(|| format!("recorded output `{}` is absent", file.path))?;
        if state.sha256 != file.sha256 {
            return Err(format!("recorded output `{}` changed bytes", file.path));
        }
        remaining -= state.bytes;
        observed_outputs.push((file.path.clone(), state));
        if let Some(media) = media {
            materialized_media.insert(file.path.clone(), media);
        }
        let address = matches!(
            interpretation,
            Some(SourceInterpretation::VibeSpec | SourceInterpretation::VibeDoc)
        )
        .then(|| canonical_doc_path(&source));
        rows.push(DescriptorOutputRow {
            output: file.path.clone(),
            source,
            interpretation,
            output_sha256: file.sha256.clone(),
            document_address: address,
        });
    }
    let record_after = project
        .read_file_snapshot_bounded(SLOT_RECORD_FILENAME, RECORD_CAP)
        .map_err(|error| format!("slot record recheck: {error:#}"))?
        .ok_or_else(|| "slot record disappeared during map construction".to_owned())?;
    let manifest_after = project
        .read_file_snapshot_bounded(Manifest::FILENAME, MANIFEST_CAP)
        .map_err(|error| format!("manifest recheck: {error:#}"))?
        .ok_or_else(|| "manifest disappeared during map construction".to_owned())?;
    if record_before != record_after || manifest != manifest_after {
        return Err("slot record or manifest changed during map construction".into());
    }
    let mut recheck_remaining = limits.max_total_bytes;
    for (path, before) in observed_outputs {
        if recheck_remaining == 0 {
            return Err(format!(
                "descriptor map exceeds {} total output bytes on recheck",
                limits.max_total_bytes
            ));
        }
        let after = project
            .stable_file_state_bounded(&path, OUTPUT_CAP.min(recheck_remaining))
            .map_err(|error| format!("recorded output `{path}` recheck: {error:#}"))?
            .ok_or_else(|| {
                format!("recorded output `{path}` disappeared during map construction")
            })?;
        if before != after {
            return Err(format!(
                "recorded output `{path}` changed during map construction"
            ));
        }
        recheck_remaining -= after.bytes;
    }
    let map = ValidatedDescriptorMap::from_witnessed_rows(
        slot.to_owned(),
        slot.join(spec_relative),
        expected_source_hash
            .expect("descriptor-owned hash checked above")
            .as_str()
            .to_owned(),
        manifest.sha256,
        rows,
    )
    .and_then(|map| map.with_materialized_media(materialized_media))
    .map_err(|error| error.to_string())?;
    Ok(Some(map))
}

fn selected_specs_root(slot: &Path) -> Result<PathBuf, String> {
    let new = layout::vibespecs_root();
    match fs::symlink_metadata(slot.join(&new)) {
        Ok(metadata) if metadata.file_type().is_dir() => Ok(new),
        Ok(_) => Err("new specs root is not a real directory".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(PathBuf::from(layout::LEGACY_SPECS_DIR))
        }
        Err(error) => Err(format!("new specs root: {error}")),
    }
}
