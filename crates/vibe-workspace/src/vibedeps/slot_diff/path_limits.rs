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
    #[cfg(windows)]
    let volume_query = check_windows_components(path, stage)?;
    match fs::symlink_metadata(path) {
        // Missing paths are expected for new slots. Other failures retain
        // their existing owner/stage; only native length errors are diagnosed here.
        Err(error) if is_length_error(&error) => {
            let error = length_error(path, stage, error);
            #[cfg(windows)]
            let error = match error {
                WorkspaceError::SpecMaterialization { path, reason } => {
                    WorkspaceError::SpecMaterialization {
                        path,
                        reason: format!("{reason} Volume component query: {volume_query}."),
                    }
                }
                other => other,
            };
            Err(error)
        }
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

/// Query the nearest existing parent without creating missing directories.
/// Each prospective path is checked separately so an existing nested mount uses
/// its own volume. Missing suffixes cannot hide an oversized component.
#[cfg(windows)]
fn check_windows_components(path: &Path, stage: &str) -> Result<String, WorkspaceError> {
    use std::os::windows::ffi::OsStrExt;

    for parent in path.ancestors().skip(1) {
        let project = match vibe_safefs::Project::open(parent) {
            Ok(project) => project,
            Err(error) => {
                if error.chain().any(|cause| {
                    cause.downcast_ref::<io::Error>().is_some_and(|error| {
                        error.kind() == io::ErrorKind::NotFound || is_length_error(error)
                    })
                }) {
                    continue;
                }
                return Ok(format!("unavailable at `{}`: {error:#}", parent.display()));
            }
        };
        let limit = match project.windows_maximum_component_length() {
            Ok(limit) => limit as usize,
            Err(error) => return Ok(format!("unavailable at `{}`: {error}", parent.display())),
        };
        for component in path
            .strip_prefix(parent)
            .expect("ancestor prefix")
            .components()
        {
            let measured = component.as_os_str().encode_wide().count();
            if measured > limit {
                return Err(WorkspaceError::SpecMaterialization {
                    path: path.to_path_buf(),
                    reason: format!(
                        "path-length refusal during {stage}: `{}`; component `{}` measures \
                         {measured} UTF-16 code units excluding NUL; volume component limit {limit} \
                         WCHARs from GetVolumeInformationByHandleW on pinned parent `{}`; \
                         exceeds limit by {} UTF-16 code units. Root shortening cannot fix this \
                         component; choose a shorter package coordinate/filename or a capable \
                         target volume and retry. No filename is silently shortened. \
                         Pre-effect metadata refusal; no native failure code was generated. \
                         This component bound does not establish a total path limit.",
                        path.display(),
                        component.as_os_str().to_string_lossy(),
                        parent.display(),
                        measured - limit,
                    ),
                });
            }
        }
        return Ok(format!(
            "component limit {limit} WCHARs at `{}`; total path limit unknown",
            parent.display()
        ));
    }
    Ok("unavailable: no existing parent capability".into())
}
