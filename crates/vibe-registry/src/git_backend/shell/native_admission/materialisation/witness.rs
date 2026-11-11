//! The witness of a staged tree: what a directory holds, and an exact copy of
//! it. Split out of `materialisation` to hold the file-length budget — these
//! two are the mechanics the staging rules are written on, not the rules.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION");
use super::super::*;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::git_backend::shell::native_admission) struct Entry {
    pub(in crate::git_backend::shell::native_admission) bytes: Vec<u8>,
    pub(in crate::git_backend::shell::native_admission) mode: u32,
    identity: (u64, u64),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::git_backend::shell::native_admission) struct Tree(
    pub(in crate::git_backend::shell::native_admission) BTreeMap<PathBuf, Entry>,
);
pub(in crate::git_backend::shell::native_admission) fn tree(
    path: &Path,
) -> Result<Option<Tree>, GitError> {
    if !path.try_exists().map_err(io)? {
        return Ok(None);
    }
    let mut entries = BTreeMap::new();
    for entry in walkdir::WalkDir::new(path).follow_links(false) {
        let entry = entry.map_err(|_| unsupported("cannot witness materialisation tree"))?;
        let metadata = fs::symlink_metadata(entry.path()).map_err(io)?;
        #[cfg(unix)]
        let mode = {
            use std::os::unix::fs::MetadataExt;
            metadata.mode()
        };
        #[cfg(not(unix))]
        let mode = u32::from(metadata.permissions().readonly());
        #[cfg(unix)]
        let identity = {
            use std::os::unix::fs::MetadataExt;
            (metadata.dev(), metadata.ino())
        };
        // Windows: creation and last-write times stand in for the inode
        // (PROP-073 `#NATIVE-WINDOWS`); a replaced file has new ones.
        #[cfg(windows)]
        let identity = {
            use std::os::windows::fs::MetadataExt;
            (metadata.creation_time(), metadata.last_write_time())
        };
        #[cfg(not(any(unix, windows)))]
        let identity = (0, 0);
        let bytes = if metadata.is_file() {
            let mut file = fs::File::open(entry.path()).map_err(io)?;
            let mut digest = Sha256::new();
            std::io::copy(&mut file, &mut digest).map_err(io)?;
            digest.finalize().to_vec()
        } else if metadata.file_type().is_symlink() {
            fs::read_link(entry.path())
                .map_err(io)?
                .as_os_str()
                .as_encoded_bytes()
                .to_vec()
        } else if metadata.is_dir() {
            Vec::new()
        } else {
            return Err(unsupported("special files cannot be staged"));
        };
        entries.insert(
            entry
                .path()
                .strip_prefix(path)
                .map_err(|_| unsupported("invalid tree witness path"))?
                .into(),
            Entry {
                bytes,
                mode,
                identity,
            },
        );
    }
    Ok(Some(Tree(entries)))
}
pub(in crate::git_backend::shell::native_admission) fn copy(
    from: &Path,
    to: &Path,
    objects: bool,
) -> Result<(), GitError> {
    let mut modes = Vec::new();
    for entry in walkdir::WalkDir::new(from).follow_links(false) {
        let entry = entry.map_err(|_| unsupported("cannot copy retained source"))?;
        let relative = entry
            .path()
            .strip_prefix(from)
            .map_err(|_| unsupported("invalid copy path"))?;
        let target = to.join(relative);
        if entry.file_type().is_dir() {
            let existed = target.exists();
            fs::create_dir_all(&target).map_err(io)?;
            if !objects || !existed {
                modes.push((
                    target,
                    fs::metadata(entry.path()).map_err(io)?.permissions(),
                ));
            }
        } else if entry.file_type().is_file() {
            if target.exists() {
                if fs::read(entry.path()).map_err(io)? != fs::read(&target).map_err(io)? {
                    let parts: Vec<_> = relative.components().collect();
                    let loose = parts.len() == 2
                        && parts[0].as_os_str().as_encoded_bytes().len() == 2
                        && matches!(parts[1].as_os_str().as_encoded_bytes().len(), 38 | 62)
                        && parts.iter().all(|p| {
                            p.as_os_str()
                                .as_encoded_bytes()
                                .iter()
                                .all(u8::is_ascii_hexdigit)
                        });
                    if !objects || !loose {
                        return Err(unsupported(
                            "retained object metadata collision during staging",
                        ));
                    }
                    // Compression is not object identity. Both repositories are
                    // fsck-validated below before this retained object is trusted.
                }
            } else {
                fs::copy(entry.path(), &target).map_err(io)?;
            }
        } else if entry.file_type().is_symlink() {
            #[cfg(unix)]
            std::os::unix::fs::symlink(fs::read_link(entry.path()).map_err(io)?, &target)
                .map_err(io)?;
            #[cfg(not(unix))]
            return Err(unsupported("symlink staging is qualified only on Unix"));
        } else {
            return Err(unsupported("special files cannot be staged"));
        }
    }
    for (path, mode) in modes.into_iter().rev() {
        fs::set_permissions(path, mode).map_err(io)?;
    }
    Ok(())
}
