//! The one directory link: a POSIX symlink, a Windows directory junction.
//!
//! A slot's link directory is a *link to a directory*, not a copy, so the
//! payload directory it names is reachable whole and a repoint is one
//! filesystem operation. Rust's standard library can make a POSIX symlink
//! and a Windows symlink, but a Windows symlink needs
//! `SeCreateSymbolicLinkPrivilege` — unavailable to an ordinary user on a
//! managed machine with Developer Mode off, which is exactly the host this
//! feature exists for. A *junction* needs no privilege, so Windows gets a
//! junction, written through `FSCTL_SET_REPARSE_POINT` because no standard
//! API creates one. A junction resolves on the local machine only, so its
//! target must be a local absolute path (`PROP-059#path-directories`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#path-directories");

use std::fs;
use std::path::{Path, PathBuf};

/// What is at a link path, as far as this module can tell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Present {
    /// Nothing is there.
    Absent,
    /// A link to a directory, and where it points.
    Link(PathBuf),
    /// A real file or directory: never vibe's to replace or delete.
    Other,
}

/// Classify the path: a POSIX symlink or a Windows junction reads back as
/// [`Present::Link`]; anything else that exists is [`Present::Other`].
pub(crate) fn inspect(path: &Path) -> std::io::Result<Present> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Present::Absent),
        Err(error) => return Err(error),
    };
    if !metadata.file_type().is_symlink() {
        return Ok(Present::Other);
    }
    // A Windows junction is a reparse point `read_link` resolves, and its
    // target comes back in the `\??\C:\…` form.
    match fs::read_link(path) {
        Ok(target) => Ok(Present::Link(normalise(target))),
        // A reparse point of a kind this host cannot read is not ours.
        Err(_) => Ok(Present::Other),
    }
}

/// Drop the Windows `\\?\` and `\??\` prefixes a reparse point carries, so a
/// recorded target and a read-back target compare equal.
pub(crate) fn normalise(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    for prefix in [r"\\?\", r"\??\"] {
        if let Some(rest) = text.strip_prefix(prefix)
            && !rest.starts_with("UNC\\")
        {
            return PathBuf::from(rest.to_string());
        }
    }
    path
}

/// Remove the link at `path`; a missing one is already removed. Only a link
/// is ever removed here — the caller classifies first.
pub(crate) fn remove(path: &Path) -> std::io::Result<()> {
    let result = if cfg!(windows) {
        // A junction is a directory entry; a POSIX symlink is a file entry.
        fs::remove_dir(path).or_else(|_| fs::remove_file(path))
    } else {
        fs::remove_file(path).or_else(|_| fs::remove_dir(path))
    };
    match result {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// Point the link at `path` to the directory `target`, replacing a link
/// already there. POSIX creates a sibling and renames it over the old link,
/// so a reader never sees the name missing; Windows has no atomic rename
/// over a junction, so the old junction is removed and the new one created
/// under the home's mutation lock.
pub(crate) fn point(path: &Path, target: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    #[cfg(unix)]
    {
        let sibling = path.with_file_name(match path.file_name() {
            Some(name) => format!(".{}.new", name.to_string_lossy()),
            None => ".link.new".to_owned(),
        });
        let _ = fs::remove_file(&sibling);
        std::os::unix::fs::symlink(target, &sibling)?;
        match fs::rename(&sibling, path) {
            Ok(()) => Ok(()),
            Err(error) => {
                let _ = fs::remove_file(&sibling);
                Err(error)
            }
        }
    }
    #[cfg(windows)]
    {
        remove(path)?;
        windows::junction(path, target)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = target;
        Err(std::io::Error::other(
            "this platform has no directory link vibe can create",
        ))
    }
}

/// Whether `target` can be the target of a link on this host. A Windows
/// junction stores a local absolute path and does not follow a UNC share,
/// so a payload on a network path is refused with a clear message instead
/// of a broken link.
pub(crate) fn supported_target(target: &Path) -> Result<(), String> {
    if !target.is_absolute() {
        return Err(format!(
            "`{}` is not an absolute path, so no link can point at it",
            target.display()
        ));
    }
    if cfg!(windows) {
        let text = target.to_string_lossy();
        let unc = text.starts_with(r"\\") || text.starts_with("//");
        if unc {
            return Err(format!(
                "`{}` is on a network path; a Windows directory junction can only point at a \
                 local absolute path, so this payload cannot be exposed on PATH as a directory. \
                 Move the library to a local disk (`vibe storage move`), or install an option \
                 that declares public commands instead",
                target.display()
            ));
        }
    }
    Ok(())
}

#[cfg(windows)]
mod windows {
    //! `FSCTL_SET_REPARSE_POINT` with a `MOUNT_POINT` reparse buffer: the
    //! only way to create a junction, which — unlike a symlink — needs no
    //! privilege. The layout is `REPARSE_DATA_BUFFER` with the mount-point
    //! fields, built here by hand because `windows-sys` exposes the
    //! constants and the call but not the variable-length buffer.

    use std::ffi::c_void;
    use std::fs;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::AsRawHandle;
    use std::path::Path;

    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::System::IO::DeviceIoControl;
    use windows_sys::Win32::System::Ioctl::FSCTL_SET_REPARSE_POINT;

    const IO_REPARSE_TAG_MOUNT_POINT: u32 = 0xA000_0003;
    /// `REPARSE_DATA_BUFFER` up to and including `Reserved`.
    const HEADER: usize = 8;
    /// The four `u16` of the mount-point name fields.
    const MOUNT_POINT_FIELDS: usize = 8;
    /// `MAXIMUM_REPARSE_DATA_BUFFER_SIZE`.
    const MAX_BUFFER: usize = 16 * 1024;

    /// Create the junction at `path` pointing at the directory `target`.
    /// `path` must not exist; the caller removed anything that was there.
    pub(super) fn junction(path: &Path, target: &Path) -> std::io::Result<()> {
        fs::create_dir(path)?;
        match write_reparse_point(path, target) {
            Ok(()) => Ok(()),
            Err(error) => {
                let _ = fs::remove_dir(path);
                Err(error)
            }
        }
    }

    #[specmark::spec(
        deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
        reason = "unsafe-gate: Rust has no standard-library API that creates a Windows directory junction, and a Windows symbolic link needs a privilege an ordinary user on a managed machine does not have (spec://org.vibevm.core/vibevm/common/PROP-059#path-directories); the reparse buffer is built in this function and lives across the one `DeviceIoControl` call, whose handle is owned by the `File` this scope holds."
    )]
    fn write_reparse_point(path: &Path, target: &Path) -> std::io::Result<()> {
        // A junction stores the NT object path of its target.
        let substitute: Vec<u16> = std::ffi::OsStr::new(&format!(r"\??\{}", target.display()))
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let print: Vec<u16> = std::ffi::OsStr::new(&target.display().to_string())
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // Lengths exclude the terminating NUL, as the structure requires.
        let substitute_bytes = (substitute.len() - 1) * 2;
        let print_bytes = (print.len() - 1) * 2;
        let path_buffer = (substitute.len() + print.len()) * 2;
        let data_length = MOUNT_POINT_FIELDS + path_buffer;
        if HEADER + data_length > MAX_BUFFER {
            return Err(std::io::Error::other(format!(
                "the junction target `{}` is too long for a reparse point",
                target.display()
            )));
        }
        let mut buffer = Vec::with_capacity(HEADER + data_length);
        buffer.extend_from_slice(&IO_REPARSE_TAG_MOUNT_POINT.to_le_bytes());
        buffer.extend_from_slice(&(data_length as u16).to_le_bytes());
        buffer.extend_from_slice(&0u16.to_le_bytes());
        buffer.extend_from_slice(&0u16.to_le_bytes());
        buffer.extend_from_slice(&(substitute_bytes as u16).to_le_bytes());
        buffer.extend_from_slice(&((substitute_bytes + 2) as u16).to_le_bytes());
        buffer.extend_from_slice(&(print_bytes as u16).to_le_bytes());
        for unit in substitute.iter().chain(print.iter()) {
            buffer.extend_from_slice(&unit.to_le_bytes());
        }
        let handle = open_reparse_handle(path)?;
        let mut returned = 0u32;
        // SAFETY: `handle` is an open directory handle this call owns for its
        // duration, `buffer` is a live, correctly sized reparse-point buffer
        // whose declared `data_length` matches its own length, and `returned`
        // is a live `u32` the call writes. No pointer outlives the call.
        let ok = unsafe {
            DeviceIoControl(
                handle.as_raw_handle() as HANDLE,
                FSCTL_SET_REPARSE_POINT,
                buffer.as_ptr() as *const c_void,
                buffer.len() as u32,
                std::ptr::null_mut(),
                0,
                &mut returned,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }

    /// An open handle to the directory itself, not to what it contains:
    /// `FILE_FLAG_BACKUP_SEMANTICS` is what lets a directory be opened, and
    /// `FILE_FLAG_OPEN_REPARSE_POINT` keeps the open from following one.
    fn open_reparse_handle(path: &Path) -> std::io::Result<fs::File> {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        fs::OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-059#path-directories")]
    fn a_link_points_repoints_and_is_removed_without_touching_a_real_directory() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first");
        let second = dir.path().join("second");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        fs::write(first.join("tool"), b"first").unwrap();
        fs::write(second.join("tool"), b"second").unwrap();
        let link = dir.path().join("links/0");

        assert_eq!(inspect(&link).unwrap(), Present::Absent);
        point(&link, &first).unwrap();
        assert_eq!(inspect(&link).unwrap(), Present::Link(first.clone()));
        assert_eq!(fs::read(link.join("tool")).unwrap(), b"first");

        // A repoint replaces the link in place and the name never vanishes.
        point(&link, &second).unwrap();
        assert_eq!(inspect(&link).unwrap(), Present::Link(second.clone()));
        assert_eq!(fs::read(link.join("tool")).unwrap(), b"second");

        remove(&link).unwrap();
        assert_eq!(inspect(&link).unwrap(), Present::Absent);
        remove(&link).unwrap();
        // The targets are untouched by any of it.
        assert!(first.join("tool").is_file() && second.join("tool").is_file());

        // A real directory at the link path is not a link, so a caller that
        // asks is told so and never replaces it.
        fs::create_dir_all(&link).unwrap();
        assert_eq!(inspect(&link).unwrap(), Present::Other);
    }

    #[test]
    fn a_relative_target_is_refused_everywhere() {
        assert!(supported_target(Path::new("relative/bin")).is_err());
        let absolute = if cfg!(windows) {
            PathBuf::from(r"C:\apps\node\bin")
        } else {
            PathBuf::from("/apps/node/bin")
        };
        assert!(supported_target(&absolute).is_ok());
    }

    #[test]
    #[cfg(windows)]
    fn a_network_target_is_refused_with_a_message_naming_the_remedy() {
        let error = supported_target(Path::new(r"\\server\share\node\bin")).unwrap_err();
        assert!(error.contains("network path"), "{error}");
        assert!(error.contains("vibe storage move"), "{error}");
    }
}
