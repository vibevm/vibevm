//! Linux: the parent image is `/proc/<ppid>/exe`, read once.

use std::io;
use std::path::PathBuf;

pub(super) fn parent_image() -> Option<PathBuf> {
    let image =
        std::fs::read_link(format!("/proc/{}/exe", std::os::unix::process::parent_id())).ok()?;
    // procfs marks an unlinked executable; that is not a usable path.
    let deleted = image
        .as_os_str()
        .as_encoded_bytes()
        .ends_with(b" (deleted)");
    (!deleted).then_some(image)
}

pub(super) fn fallback() -> io::Result<PathBuf> {
    Ok(PathBuf::from("/bin/sh"))
}
