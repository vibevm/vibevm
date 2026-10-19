//! Read-only host probes before reconciliation mutates a slot. These do not
//! replace containment checks or promise that later filesystem I/O will succeed.

use std::{fs, io, path::Path};

use super::PreparedSlotFile;
use crate::WorkspaceError;

pub(crate) const STAGING_RANDOM_BYTES: usize = 8;

pub(super) fn preflight(slot: &Path, files: &[PreparedSlotFile]) -> Result<(), WorkspaceError> {
    probe(slot, "slot directory")?;
    probe(&slot.join(super::SLOT_RECORD_FILENAME), "slot record")?;
    probe(&slot.join(".vibe-record-00000000"), "record staging")?;
    for file in files {
        let output = slot.join(&file.path);
        probe(&output, "payload destination")?;
        if let Some(parent) = output.parent() {
            probe(&parent.join(".vibe-place-00000000"), "payload staging")?;
        }
    }
    Ok(())
}

fn probe(path: &Path, stage: &str) -> Result<(), WorkspaceError> {
    match fs::symlink_metadata(path) {
        // Missing paths are expected for new slots. Other failures retain
        // their existing owner/stage; only native length errors are diagnosed here.
        Err(error) if is_length_error(&error) => Err(length_error(path, stage, error)),
        _ => Ok(()),
    }
}

pub(super) fn io_error(path: &Path, error: io::Error) -> WorkspaceError {
    if is_length_error(&error) {
        length_error(
            path,
            "slot reconciliation I/O (state may be partial)",
            error,
        )
    } else {
        super::super::io_err(path, error)
    }
}

fn is_length_error(error: &io::Error) -> bool {
    // ERROR_FILENAME_EXCED_RANGE / ENAMETOOLONG. Do not classify permission,
    // missing-parent or nesting failures by their localized message text.
    #[cfg(windows)]
    let code = 206;
    #[cfg(target_os = "linux")]
    let code = 36;
    #[cfg(target_os = "macos")]
    let code = 63;
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    return false;
    #[cfg(any(windows, target_os = "linux", target_os = "macos"))]
    {
        error.raw_os_error() == Some(code)
    }
}

fn length_error(path: &Path, stage: &str, error: io::Error) -> WorkspaceError {
    #[cfg(windows)]
    let measured = {
        use std::os::windows::ffi::OsStrExt;
        format!(
            "{} UTF-16 code units excluding NUL",
            path.as_os_str().encode_wide().count()
        )
    };
    #[cfg(not(windows))]
    let measured = format!(
        "{} bytes excluding NUL",
        path.as_os_str().as_encoded_bytes().len()
    );
    WorkspaceError::SpecMaterialization {
        path: path.to_path_buf(),
        reason: format!(
            "path-length refusal during {stage}: `{}` ({measured}); native OS code {:?}: {error}. \
             The host did not report its exact effective path/component limit. \
             Shorten the project root and retry the same package; if a single filename is too long, \
             root shortening cannot fix that component. No filename is silently shortened.",
            path.display(),
            error.raw_os_error()
        ),
    }
}
