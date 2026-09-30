//! Read a descriptor-owned installed slot into a resolver map without treating
//! filename suffixes as source interpretation.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-REPRESENTATION-COLLISION");

use std::fs;
use std::path::{Path, PathBuf};

use vibe_core::layout;
use vibe_core::manifest::{Manifest, SelectedSource, SourceInterpretation, SpecFormat};
use vibe_core::{ContentHash, Group};
use vibe_safefs::Project;
use vibe_spec::{DescriptorOutputRow, ValidatedDescriptorMap, canonical_doc_path};

use super::slot_record::{SLOT_RECORD_FILENAME, read_slot_record_bytes};

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

#[allow(
    clippy::too_many_arguments,
    reason = "testable descriptor-only resource ceiling"
)]
pub(super) fn read_validated_descriptor_map_with_limits(
    slot: &Path,
    group: &Group,
    name: &str,
    version: &semver::Version,
    expected_source_hash: &ContentHash,
    expected_format: SpecFormat,
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
    if !authored {
        return Ok(None);
    }
    let package = Manifest::parse_str(text)
        .map_err(|error| format!("installed descriptor manifest: {error}"))?
        .package
        .ok_or_else(|| "installed descriptors require [package]".to_owned())?;
    if &package.group != group || package.name != name || &package.version != version {
        return Err("installed descriptor package differs from selected coordinate".into());
    }
    let record_before = project
        .read_file_snapshot_bounded(SLOT_RECORD_FILENAME, RECORD_CAP)
        .map_err(|error| format!("slot record: {error:#}"))?
        .ok_or_else(|| "descriptor-owned slot has no retained slot record".to_owned())?;
    let record = read_slot_record_bytes(&slot.join(SLOT_RECORD_FILENAME), &record_before.bytes)?;
    if &record.source_hash != expected_source_hash || record.spec_format != expected_format {
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
        let interpretation = match selection {
            SelectedSource::Unclassified { .. } => None,
            SelectedSource::Descriptor { descriptor, .. } => Some(descriptor.interpretation()),
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
        selected.push((file, source.to_owned(), interpretation));
        if selected.len() > limits.max_outputs {
            return Err(format!(
                "descriptor map has more than {} selected outputs",
                limits.max_outputs
            ));
        }
    }
    let mut rows = Vec::with_capacity(selected.len());
    let mut observed_outputs = Vec::with_capacity(selected.len());
    let mut remaining = limits.max_total_bytes;
    for (file, source, interpretation) in selected {
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
        expected_source_hash.as_str().to_owned(),
        manifest.sha256,
        rows,
    )
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
