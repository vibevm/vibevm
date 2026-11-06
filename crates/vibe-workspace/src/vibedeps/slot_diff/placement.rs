//! Place incumbent source/converted payloads and captured source observations.

use super::*;

pub(super) fn place_atomically(slot: &Path, file: &PreparedSlotFile) -> Result<(), WorkspaceError> {
    let destination = slot.join(&file.path);
    let parent = destination
        .parent()
        .expect("a prepared relative file always has a parent");
    fs::create_dir_all(parent).map_err(|error| io_err(parent, error))?;
    match &file.content {
        PreparedContent::Observed {
            observation,
            path,
            mode,
        } => place_observed(observation, path, *mode, &destination, parent, &file.sha256),
        PreparedContent::Bytes(bytes) => {
            let mut temporary = staged_file(parent)?;
            temporary
                .as_file_mut()
                .write_all(bytes)
                .map_err(|error| io_err(temporary.path(), error))?;
            persist_staged(temporary, &destination, &file.sha256)
        }
        PreparedContent::Source {
            path,
            mode: CopyMode::Copy,
        } => {
            let temporary = staged_file(parent)?;
            fs::copy(path, temporary.path()).map_err(|error| io_err(temporary.path(), error))?;
            persist_staged(temporary, &destination, &file.sha256)
        }
        PreparedContent::Source {
            path,
            mode: CopyMode::Hardlink,
        } => place_hardlink(path, &destination, parent, &file.sha256),
    }
}

fn place_observed(
    observation: &SourceObservation,
    source: &Path,
    mode: CopyMode,
    destination: &Path,
    parent: &Path,
    expected_hash: &str,
) -> Result<(), WorkspaceError> {
    place_observed_using_link(
        observation,
        source,
        mode,
        destination,
        parent,
        expected_hash,
        |source, destination| fs::hard_link(source, destination),
    )
}

fn place_observed_using_link(
    observation: &SourceObservation,
    source: &Path,
    mode: CopyMode,
    destination: &Path,
    parent: &Path,
    expected_hash: &str,
    link: impl FnOnce(&Path, &Path) -> std::io::Result<()>,
) -> Result<(), WorkspaceError> {
    if mode == CopyMode::Hardlink {
        let staging = tempfile::Builder::new()
            .prefix(".vibe-place-")
            .tempdir_in(parent)
            .map_err(|error| io_err(parent, error))?;
        let candidate = staging.path().join("payload");
        if link(source, &candidate).is_ok() {
            let result = (|| {
                let project = vibe_safefs::Project::open(staging.path())
                    .map_err(|error| materialisation_error(&candidate, error.to_string()))?;
                let linked = project
                    .read_source_observation("payload")
                    .map_err(|error| materialisation_error(&candidate, error.to_string()))?
                    .ok_or_else(|| {
                        materialisation_error(&candidate, "staged hardlink disappeared")
                    })?;
                if linked.identity() != observation.identity()
                    || linked.bytes() != observation.bytes()
                    || linked.permissions() != observation.permissions()
                {
                    return Err(materialisation_error(
                        destination,
                        "staged hardlink differs from captured source identity, bytes or permissions",
                    ));
                }
                // Never chmod a shared inode. Its captured mode was just checked.
                fs::rename(&candidate, destination).map_err(|error| io_err(destination, error))?;
                Ok(())
            })();
            if let Err(error) = result {
                // Do not let recursive temporary-directory cleanup clear the
                // readonly flag on a shared inode. Unlink only; if refused,
                // retain the diagnostic staging path without touching source.
                let retained = staging.keep();
                match fs::remove_file(&candidate) {
                    Ok(()) => {
                        let _ = fs::remove_dir(retained);
                    }
                    Err(cleanup) if cleanup.kind() == std::io::ErrorKind::NotFound => {
                        let _ = fs::remove_dir(retained);
                    }
                    Err(cleanup) => {
                        return Err(materialisation_error(
                            &candidate,
                            format!(
                                "{error}; rejected shared candidate retained because unlink failed: {cleanup}; source permissions were not changed"
                            ),
                        ));
                    }
                }
                return Err(error);
            }
            return Ok(());
        }
        // Preserve the incumbent hardlink→copy fallback, but consume captured
        // bytes and permissions rather than reopening the source.
    }
    let mut temporary = staged_file(parent)?;
    temporary
        .as_file_mut()
        .write_all(observation.bytes())
        .map_err(|error| io_err(temporary.path(), error))?;
    temporary
        .as_file()
        .set_permissions(observation.permissions().clone())
        .map_err(|error| io_err(temporary.path(), error))?;
    persist_staged(temporary, destination, expected_hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn observed_hardlink_identity_or_mode_drift_does_not_replace_destination_or_chmod_source() {
        use std::os::unix::fs::PermissionsExt;
        for replace in [false, true] {
            let source = tempfile::tempdir().unwrap();
            let output = tempfile::tempdir().unwrap();
            let path = source.path().join("source");
            fs::write(&path, b"same").unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
            let observed = vibe_safefs::Project::open(source.path())
                .unwrap()
                .read_source_observation("source")
                .unwrap()
                .unwrap();
            if replace {
                fs::rename(&path, source.path().join("original")).unwrap();
                fs::write(&path, b"same").unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
            } else {
                fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).unwrap();
            }
            let destination = output.path().join("payload");
            fs::write(&destination, b"old destination").unwrap();
            let mode = fs::metadata(&path).unwrap().permissions();
            assert!(
                place_observed(
                    &observed,
                    &path,
                    CopyMode::Hardlink,
                    &destination,
                    output.path(),
                    &sha256_bytes(observed.bytes())
                )
                .is_err()
            );
            assert_eq!(fs::read(&destination).unwrap(), b"old destination");
            assert_eq!(fs::metadata(&path).unwrap().permissions(), mode);
            assert_eq!(fs::read_dir(output.path()).unwrap().count(), 1);
        }
    }

    #[cfg(unix)]
    #[test]
    fn refused_hardlink_attempt_falls_back_to_captured_copy_with_original_mode() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let source = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        let path = source.path().join("source");
        fs::write(&path, b"captured").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).unwrap();
        let observed = vibe_safefs::Project::open(source.path())
            .unwrap()
            .read_source_observation("source")
            .unwrap()
            .unwrap();
        let destination = output.path().join("payload");
        place_observed_using_link(
            &observed,
            &path,
            CopyMode::Hardlink,
            &destination,
            output.path(),
            &sha256_bytes(observed.bytes()),
            |_, _| {
                Err(std::io::Error::new(
                    std::io::ErrorKind::CrossesDevices,
                    "fixture cross-device",
                ))
            },
        )
        .unwrap();
        assert_eq!(fs::read(&destination).unwrap(), b"captured");
        assert_eq!(
            fs::metadata(&destination).unwrap().permissions().mode() & 0o777,
            0o555
        );
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o555
        );
        assert_ne!(
            fs::metadata(&destination).unwrap().ino(),
            fs::metadata(&path).unwrap().ino()
        );
    }
}
