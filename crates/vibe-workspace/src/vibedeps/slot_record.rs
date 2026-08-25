//! Durable identity and footprint record for a materialised dependency slot.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#SLOT-RECORD");

use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path};

use sha2::{Digest, Sha256};
use vibe_core::{ContentHash, manifest::SpecFormat};
use vibe_wire::generated::slot_record::{
    SlotFile as WireSlotFile, SlotFileDisposition as WireSlotFileDisposition,
    SlotRecord as WireSlotRecord, SlotRecordSpecFormat as WireSpecFormat,
};

/// Name of the identity and footprint record at a materialised slot root.
pub const SLOT_RECORD_FILENAME: &str = ".vibe-slot.toml";
/// Current slot-record wire schema.
pub const SLOT_RECORD_SCHEMA: u32 = 1;

/// Identity and complete materialiser-owned footprint for one dependency slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotRecord {
    pub schema: u32,
    pub source_hash: ContentHash,
    pub spec_format: SpecFormat,
    pub converter_recipe: Option<String>,
    pub derived_hash: Option<ContentHash>,
    pub overlay_hash: Option<ContentHash>,
    pub files: Vec<SlotFile>,
}

/// One materialiser-owned file in a dependency slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotFile {
    pub path: String,
    pub sha256: String,
    pub source: Option<String>,
    pub disposition: Option<SlotFileDisposition>,
}

/// Whether a transformed-slot row was converted or copied unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotFileDisposition {
    Converted,
    Copied,
}

/// Read and validate the slot record rooted at `slot`.
pub fn read_slot_record(slot: &Path) -> Result<SlotRecord, String> {
    let path = slot.join(SLOT_RECORD_FILENAME);
    refuse_record_symlink(&path)?;
    let text = fs::read_to_string(&path).map_err(|error| {
        format!(
            "slot record I/O failure at `{}`: cannot read: {error}",
            path.display()
        )
    })?;
    let wire: WireSlotRecord = toml::from_str(&text)
        .map_err(|error| format!("slot record parse failure at `{}`: {error}", path.display()))?;
    let record = record_from_wire(wire).map_err(|error| format_validation_error(&path, error))?;
    validate_record(&record).map_err(|error| format_validation_error(&path, error))?;
    Ok(record)
}

/// Validate and write `record` as pretty TOML at the root of `slot`.
pub fn write_slot_record(slot: &Path, record: &SlotRecord) -> Result<(), String> {
    let path = slot.join(SLOT_RECORD_FILENAME);
    refuse_record_symlink(&path)?;
    validate_record(record).map_err(|error| format_validation_error(&path, error))?;
    let wire = toml::to_string_pretty(&record_to_wire(record)).map_err(|error| {
        format!(
            "slot record serialization failure at `{}`: {error}",
            path.display()
        )
    })?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".vibe-record-")
        .tempfile_in(slot)
        .map_err(|error| {
            format!(
                "slot record I/O failure at `{}`: cannot stage: {error}",
                path.display()
            )
        })?;
    temporary
        .as_file_mut()
        .write_all(wire.as_bytes())
        .map_err(|error| {
            format!(
                "slot record I/O failure at `{}`: cannot stage: {error}",
                path.display()
            )
        })?;
    temporary.as_file().sync_all().map_err(|error| {
        format!(
            "slot record I/O failure at `{}`: cannot sync: {error}",
            path.display()
        )
    })?;
    temporary.into_temp_path().persist(&path).map_err(|error| {
        format!(
            "slot record I/O failure at `{}`: cannot replace: {}",
            path.display(),
            error.error
        )
    })?;
    Ok(())
}

fn record_from_wire(wire: WireSlotRecord) -> Result<SlotRecord, ValidationError> {
    let source_hash = ContentHash::parse(&wire.source_hash)
        .map_err(|error| ValidationError::Invariant(format!("source_hash is invalid: {error}")))?;
    let derived_hash = wire
        .derived_hash
        .map(|hash| {
            ContentHash::parse(&hash).map_err(|error| {
                ValidationError::Invariant(format!("derived_hash is invalid: {error}"))
            })
        })
        .transpose()?;
    let overlay_hash = wire
        .overlay_hash
        .map(|hash| {
            ContentHash::parse(&hash).map_err(|error| {
                ValidationError::Invariant(format!("overlay_hash is invalid: {error}"))
            })
        })
        .transpose()?;
    Ok(SlotRecord {
        schema: wire.schema,
        source_hash,
        spec_format: match wire.spec_format {
            WireSpecFormat::Mixed => SpecFormat::Mixed,
            WireSpecFormat::Markdown => SpecFormat::Markdown,
            WireSpecFormat::Xml => SpecFormat::Xml,
        },
        converter_recipe: wire.converter_recipe,
        derived_hash,
        overlay_hash,
        files: wire
            .file
            .into_iter()
            .map(|file| SlotFile {
                path: file.path,
                sha256: file.sha256,
                source: file.source,
                disposition: file.disposition.map(|disposition| match disposition {
                    WireSlotFileDisposition::Converted => SlotFileDisposition::Converted,
                    WireSlotFileDisposition::Copied => SlotFileDisposition::Copied,
                }),
            })
            .collect(),
    })
}

fn record_to_wire(record: &SlotRecord) -> WireSlotRecord {
    WireSlotRecord {
        file: record
            .files
            .iter()
            .map(|file| WireSlotFile {
                path: file.path.clone(),
                sha256: file.sha256.clone(),
                disposition: file.disposition.map(|disposition| match disposition {
                    SlotFileDisposition::Converted => WireSlotFileDisposition::Converted,
                    SlotFileDisposition::Copied => WireSlotFileDisposition::Copied,
                }),
                source: file.source.clone(),
            })
            .collect(),
        schema: record.schema,
        source_hash: record.source_hash.as_str().to_string(),
        spec_format: match record.spec_format {
            SpecFormat::Mixed => WireSpecFormat::Mixed,
            SpecFormat::Markdown => WireSpecFormat::Markdown,
            SpecFormat::Xml => WireSpecFormat::Xml,
        },
        converter_recipe: record.converter_recipe.clone(),
        derived_hash: record
            .derived_hash
            .as_ref()
            .map(|hash| hash.as_str().to_string()),
        overlay_hash: record
            .overlay_hash
            .as_ref()
            .map(|hash| hash.as_str().to_string()),
    }
}

/// Return the SHA-256 of `path` as 64 lowercase hexadecimal digits.
pub fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path)
        .map_err(|error| format!("cannot open `{}` for SHA-256: {error}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("cannot read `{}` for SHA-256: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(lower_hex(&hasher.finalize()))
}

/// Verify exactly the payload files named by `record`, ignoring every
/// unrecorded path in the slot.
pub fn verify_recorded_files(slot: &Path, record: &SlotRecord) -> Result<(), String> {
    validate_record(record)
        .map_err(|error| format_validation_error(&slot.join(SLOT_RECORD_FILENAME), error))?;
    for file in &record.files {
        let path = slot.join(&file.path);
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            format!(
                "recorded payload `{}` cannot be inspected: {error}",
                path.display()
            )
        })?;
        if !metadata.file_type().is_file() {
            return Err(format!(
                "recorded payload `{}` is not a regular file",
                path.display()
            ));
        }
        let actual = sha256_file(&path)?;
        if actual != file.sha256 {
            return Err(format!(
                "recorded payload `{}` hashes to {actual}, record expects {}",
                file.path, file.sha256
            ));
        }
    }
    Ok(())
}

/// Compute recipe-0 aggregate identity from exactly the sorted recorded
/// payload paths: `path || NUL || bytes || NUL`.
pub fn compute_recorded_payload_hash(
    slot: &Path,
    files: &[SlotFile],
) -> Result<ContentHash, String> {
    validate_file_rows(files).map_err(|error| match error {
        ValidationError::Invariant(message) => {
            format!("recorded payload invariant failure: {message}")
        }
        ValidationError::Schema(actual) => {
            format!("recorded payload has unexpected schema validation {actual}")
        }
    })?;
    let mut hasher = Sha256::new();
    for file in files {
        let path = slot.join(&file.path);
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            format!(
                "recorded payload `{}` cannot be inspected: {error}",
                path.display()
            )
        })?;
        if !metadata.file_type().is_file() {
            return Err(format!(
                "recorded payload `{}` is not a regular file",
                path.display()
            ));
        }
        let bytes = fs::read(&path).map_err(|error| {
            format!(
                "recorded payload `{}` cannot be read: {error}",
                path.display()
            )
        })?;
        hasher.update(file.path.as_bytes());
        hasher.update([0]);
        hasher.update(bytes);
        hasher.update([0]);
    }
    Ok(ContentHash::from_validated(format!(
        "sha256:{}",
        lower_hex(&hasher.finalize())
    )))
}

pub(super) fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for &byte in bytes {
        output.push(HEX[usize::from(byte >> 4)] as char);
        output.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    output
}

#[derive(Debug)]
enum ValidationError {
    Schema(u32),
    Invariant(String),
}

fn validate_record(record: &SlotRecord) -> Result<(), ValidationError> {
    if record.schema != SLOT_RECORD_SCHEMA {
        return Err(ValidationError::Schema(record.schema));
    }
    validate_optional_text("converter_recipe", record.converter_recipe.as_deref())?;

    validate_file_rows(&record.files)?;

    match record.spec_format {
        SpecFormat::Mixed => validate_mixed_shape(record),
        SpecFormat::Markdown | SpecFormat::Xml => validate_transformed_shape(record),
    }
}

fn validate_file_rows(files: &[SlotFile]) -> Result<(), ValidationError> {
    let mut previous: Option<&str> = None;
    for (index, file) in files.iter().enumerate() {
        validate_relative_path(&format!("file[{index}].path"), &file.path)?;
        if file.path == SLOT_RECORD_FILENAME {
            return invariant(format!(
                "file[{index}].path `{SLOT_RECORD_FILENAME}` is reserved for the slot record"
            ));
        }
        validate_sha256(index, &file.sha256)?;
        if let Some(source) = file.source.as_deref() {
            validate_relative_path(&format!("file[{index}].source"), source)?;
        }
        if let Some(prior) = previous {
            if file.path == prior {
                return invariant(format!("file[{index}].path duplicates `{}`", file.path));
            }
            if file.path.as_str() < prior {
                return invariant(format!(
                    "file[{index}].path `{}` is not strictly sorted after `{prior}`",
                    file.path
                ));
            }
        }
        previous = Some(&file.path);
    }
    Ok(())
}

fn validate_optional_text(name: &str, value: Option<&str>) -> Result<(), ValidationError> {
    if value.is_some_and(str::is_empty) {
        return invariant(format!("{name} must be omitted rather than empty"));
    }
    Ok(())
}

fn validate_relative_path(field: &str, value: &str) -> Result<(), ValidationError> {
    if value.is_empty() {
        return invariant(format!("{field} must not be empty"));
    }
    if value.contains('\\') {
        return invariant(format!("{field} `{value}` must use forward slashes"));
    }
    if value.contains('\0') {
        return invariant(format!("{field} contains a NUL byte"));
    }
    let bytes = value.as_bytes();
    let has_windows_drive_prefix =
        bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
    if value.starts_with('/') || has_windows_drive_prefix {
        return invariant(format!("{field} `{value}` must be relative"));
    }
    if value
        .split('/')
        .any(|component| component.is_empty() || matches!(component, "." | ".."))
    {
        return invariant(format!(
            "{field} `{value}` contains an empty, `.` or `..` component"
        ));
    }
    if Path::new(value).components().any(|component| {
        matches!(
            component,
            Component::Prefix(_) | Component::RootDir | Component::CurDir | Component::ParentDir
        )
    }) {
        return invariant(format!("{field} `{value}` must be relative"));
    }
    Ok(())
}

fn refuse_record_symlink(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(format!(
            "slot record I/O failure at `{}`: refusing to follow a symbolic link",
            path.display()
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "slot record I/O failure at `{}`: cannot inspect: {error}",
            path.display()
        )),
    }
}

fn validate_sha256(index: usize, value: &str) -> Result<(), ValidationError> {
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

fn validate_mixed_shape(record: &SlotRecord) -> Result<(), ValidationError> {
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

fn validate_transformed_shape(record: &SlotRecord) -> Result<(), ValidationError> {
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
    }
    Ok(())
}

fn invariant(message: impl Into<String>) -> Result<(), ValidationError> {
    Err(ValidationError::Invariant(message.into()))
}

fn format_validation_error(path: &Path, error: ValidationError) -> String {
    match error {
        ValidationError::Schema(actual) => format!(
            "slot record schema failure at `{}`: schema {actual} is unsupported; expected {SLOT_RECORD_SCHEMA}",
            path.display()
        ),
        ValidationError::Invariant(message) => format!(
            "slot record invariant failure at `{}`: {message}",
            path.display()
        ),
    }
}

#[cfg(test)]
mod tests;
