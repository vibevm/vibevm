//! Read the historical derived-manifest view without weakening slot-record truth.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-045#materialisation");

use std::fs;
use std::path::Path;

use vibe_core::ContentHash;
use vibe_core::manifest::SpecFormat;

use super::{
    CONVERTER_RECIPE, DERIVED_MANIFEST_FILENAME, DERIVED_MANIFEST_SCHEMA, DerivedFile,
    DerivedFileDisposition, DerivedManifest, SLOT_RECORD_FILENAME, SlotFileDisposition,
    current_overlay_hash_for_slot, read_slot_record,
};

/// Read and validate the typed derived manifest from `slot`. A schema-2
/// copied-unconverted row cannot be flattened into schema-1 Converted/Copied.
pub fn read_derived_manifest(slot: &Path) -> Result<DerivedManifest, String> {
    let record_path = slot.join(SLOT_RECORD_FILENAME);
    match fs::symlink_metadata(&record_path) {
        Ok(_) => {
            let record = read_slot_record(slot)?;
            if record.spec_format == SpecFormat::Mixed {
                return Err("slot record describes a mixed slot, not a derived slot".to_string());
            }
            let converter_recipe = record
                .converter_recipe
                .ok_or_else(|| "transformed slot record has no converter_recipe".to_string())?;
            let derived_hash = record
                .derived_hash
                .ok_or_else(|| "transformed slot record has no derived_hash".to_string())?;
            let files = record
                .files
                .into_iter()
                .map(|file| {
                    let source = file.source.ok_or_else(|| {
                        format!("transformed slot row `{}` has no source", file.path)
                    })?;
                    let disposition = match file.disposition.ok_or_else(|| {
                        format!("transformed slot row `{}` has no disposition", file.path)
                    })? {
                        SlotFileDisposition::Converted => DerivedFileDisposition::Converted,
                        SlotFileDisposition::Copied => DerivedFileDisposition::Copied,
                        SlotFileDisposition::CopiedUnconverted(_) => {
                            return Err(format!(
                                "transformed slot row `{}` has an unconverted source and cannot be represented by the legacy derived manifest",
                                file.path
                            ));
                        }
                    };
                    Ok(DerivedFile {
                        source,
                        output: file.path,
                        disposition,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            return Ok(DerivedManifest {
                schema: DERIVED_MANIFEST_SCHEMA,
                source_hash: record.source_hash.as_str().to_string(),
                output_format: record.spec_format,
                converter_recipe,
                overlay_hash: record.overlay_hash.map(|hash| hash.as_str().to_string()),
                derived_hash: derived_hash.as_str().to_string(),
                files,
            });
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "cannot inspect slot record `{}`: {error}",
                record_path.display()
            ));
        }
    }
    let path = slot.join(DERIVED_MANIFEST_FILENAME);
    let text = fs::read_to_string(&path).map_err(|e| format!("cannot read manifest: {e}"))?;
    let manifest: DerivedManifest =
        toml::from_str(&text).map_err(|e| format!("cannot parse manifest: {e}"))?;
    if manifest.schema != DERIVED_MANIFEST_SCHEMA {
        return Err(format!(
            "schema is {}, current schema is {DERIVED_MANIFEST_SCHEMA}",
            manifest.schema
        ));
    }
    Ok(manifest)
}

/// Cheap freshness gate consulted before presence trust. New slot records are
/// preferred; legacy derived manifests remain readable when no record exists.
pub fn format_is_current(slot: &Path, expected: SpecFormat) -> bool {
    let record_path = slot.join(SLOT_RECORD_FILENAME);
    match fs::symlink_metadata(&record_path) {
        Ok(_) => {
            return read_slot_record(slot).is_ok_and(|record| {
                record.spec_format == expected
                    && match expected {
                        SpecFormat::Mixed => true,
                        SpecFormat::Markdown | SpecFormat::Xml => {
                            record.converter_recipe.as_deref() == Some(CONVERTER_RECIPE)
                                && record.overlay_hash.as_ref().map(ContentHash::as_str)
                                    == current_overlay_hash_for_slot(slot).as_deref()
                        }
                    }
            });
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return false,
    }
    let manifest_path = slot.join(DERIVED_MANIFEST_FILENAME);
    match expected {
        SpecFormat::Mixed => !manifest_path.exists(),
        SpecFormat::Markdown | SpecFormat::Xml => {
            read_derived_manifest(slot).is_ok_and(|manifest| {
                manifest.output_format == expected
                    && manifest.overlay_hash == current_overlay_hash_for_slot(slot)
            })
        }
    }
}
