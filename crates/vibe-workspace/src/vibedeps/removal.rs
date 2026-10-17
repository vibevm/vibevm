//! Whole-slot deletion requires a proved, unchanged materializer footprint.
use super::{SLOT_RECORD_FILENAME, read_slot_record, verify_recorded_files};
use crate::WorkspaceError;
use std::{collections::BTreeSet, fs, path::Path};

/// Refuse a missing/unknown/modified footprint, symlinks and unrecorded user
/// files before a caller starts its multi-file removal transaction.
pub fn validate_owned_slot_removal(slot: &Path) -> Result<(), WorkspaceError> {
    let fail = |reason: String| WorkspaceError::Io {
        path: slot.to_path_buf(),
        reason,
    };
    let record = read_slot_record(slot).map_err(&fail)?;
    verify_recorded_files(slot, &record).map_err(&fail)?;
    let allowed = record
        .files
        .iter()
        .map(|file| file.path.as_str())
        .chain(std::iter::once(SLOT_RECORD_FILENAME))
        .collect::<BTreeSet<_>>();
    fn visit(root: &Path, path: &Path, allowed: &BTreeSet<&str>) -> Result<(), String> {
        let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
        if metadata.file_type().is_dir() {
            if path != root {
                let relative = crate::configuration_transaction::portable_relative_path(
                    path.strip_prefix(root).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                let prefix = format!("{relative}/");
                if !allowed.iter().any(|file| file.starts_with(&prefix)) {
                    return Err(format!(
                        "unrecorded directory `{relative}` prevents whole-slot deletion"
                    ));
                }
            }
            for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
                visit(root, &entry.map_err(|e| e.to_string())?.path(), allowed)?;
            }
        } else if metadata.file_type().is_file() {
            let relative = crate::configuration_transaction::portable_relative_path(
                path.strip_prefix(root).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            if !allowed.contains(relative.as_str()) {
                return Err(format!(
                    "unrecorded state `{relative}` prevents whole-slot deletion; preserve it before retrying"
                ));
            }
        } else {
            return Err(format!(
                "non-regular state `{}` prevents whole-slot deletion",
                path.display()
            ));
        }
        Ok(())
    }
    visit(slot, slot, &allowed).map_err(fail)
}
