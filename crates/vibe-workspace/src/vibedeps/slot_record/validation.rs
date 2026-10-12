//! Slot-record shape checks shared by the schema-1 reader and schema-2 writer.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#SLOT-RECORD");

use std::path::Path;

use vibe_core::manifest::SpecFormat;

use super::{
    LEGACY_SLOT_RECORD_SCHEMA, SLOT_RECORD_SCHEMA, SlotFileDisposition, SlotRecord, ValidationError,
};

pub(super) fn validate_sha256(index: usize, value: &str) -> Result<(), ValidationError> {
    let valid = value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase());
    if !valid {
        return invariant(format!(
            "file[{index}].sha256 must be exactly 64 lowercase hexadecimal digits"
        ));
    }
    Ok(())
}

pub(super) fn validate_mixed_shape(record: &SlotRecord) -> Result<(), ValidationError> {
    if record.converter_recipe.is_some()
        || record.derived_hash.is_some()
        || record.overlay_hash.is_some()
    {
        return invariant(
            "mixed records must omit converter_recipe, derived_hash, and overlay_hash",
        );
    }
    for (index, file) in record.files.iter().enumerate() {
        if file.source.is_some() || file.disposition.is_some() {
            return invariant(format!(
                "mixed file[{index}] must omit source and disposition"
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_transformed_shape(record: &SlotRecord) -> Result<(), ValidationError> {
    if record.converter_recipe.is_none() {
        return invariant("transformed records require converter_recipe");
    }
    if record.derived_hash.is_none() {
        return invariant("transformed records require derived_hash");
    }
    for (index, file) in record.files.iter().enumerate() {
        if file.source.is_none() || file.disposition.is_none() {
            return invariant(format!(
                "transformed file[{index}] requires both source and disposition"
            ));
        }
        if matches!(
            file.disposition.as_ref(),
            Some(SlotFileDisposition::CopiedUnconverted(_))
        ) && record.spec_format != SpecFormat::Xml
        {
            return invariant(format!(
                "transformed file[{index}] copied-unconverted requires XML target"
            ));
        }
    }
    Ok(())
}

pub(super) fn invariant(message: impl Into<String>) -> Result<(), ValidationError> {
    Err(ValidationError::Invariant(message.into()))
}

pub(super) fn format_validation_error(path: &Path, error: ValidationError) -> String {
    match error {
        ValidationError::Schema(actual) => format!(
            "slot record schema failure at `{}`: schema {actual} is unsupported; expected {LEGACY_SLOT_RECORD_SCHEMA} or {SLOT_RECORD_SCHEMA}",
            path.display()
        ),
        ValidationError::Invariant(message) => format!(
            "slot record invariant failure at `{}`: {message}",
            path.display()
        ),
    }
}
