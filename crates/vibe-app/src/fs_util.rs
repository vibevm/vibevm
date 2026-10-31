//! The one recursive directory copy, shared by a cross-filesystem payload
//! move and a `vibe storage move`, and the merging copy of a `copy` step.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#move");

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The failed step of a copy; each caller maps it into its own layer error.
#[derive(Debug)]
pub(crate) struct CopyFailure {
    pub action: &'static str,
    pub path: PathBuf,
    pub source: io::Error,
}

fn failed(action: &'static str, path: &Path) -> impl FnOnce(io::Error) -> CopyFailure {
    let path = path.to_path_buf();
    move |source| CopyFailure {
        action,
        path,
        source,
    }
}

/// Copy a directory tree; `to` must not exist. Symbolic links are copied
/// as links, file permissions travel with `fs::copy` and each directory
/// keeps the permissions of its source.
pub(crate) fn copy_tree(from: &Path, to: &Path) -> Result<(), CopyFailure> {
    fs::create_dir(to).map_err(failed("creating", to))?;
    for entry in fs::read_dir(from).map_err(failed("reading", from))? {
        let entry = entry.map_err(failed("reading", from))?;
        let source_path = entry.path();
        let target = to.join(entry.file_name());
        let kind = entry.file_type().map_err(failed("reading", &source_path))?;
        if kind.is_dir() {
            copy_tree(&source_path, &target)?;
        } else if kind.is_symlink() {
            copy_link(&source_path, &target)?;
        } else {
            fs::copy(&source_path, &target).map_err(failed("copying", &source_path))?;
        }
    }
    let permissions = fs::metadata(from)
        .map_err(failed("reading", from))?
        .permissions();
    fs::set_permissions(to, permissions).map_err(failed("copying", from))
}

/// Copy a file or a directory tree to `to`, creating parents and merging
/// into existing directories; an existing file is overwritten.
pub(crate) fn copy_into(from: &Path, to: &Path) -> Result<(), CopyFailure> {
    let kind = fs::symlink_metadata(from).map_err(failed("reading", from))?;
    if !kind.is_dir() {
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).map_err(failed("creating", parent))?;
        }
        return fs::copy(from, to)
            .map(|_| ())
            .map_err(failed("copying", from));
    }
    fs::create_dir_all(to).map_err(failed("creating", to))?;
    for entry in fs::read_dir(from).map_err(failed("reading", from))? {
        let entry = entry.map_err(failed("reading", from))?;
        copy_into(&entry.path(), &to.join(entry.file_name()))?;
    }
    Ok(())
}

#[cfg(unix)]
fn copy_link(from: &Path, to: &Path) -> Result<(), CopyFailure> {
    let link = fs::read_link(from).map_err(failed("reading", from))?;
    std::os::unix::fs::symlink(link, to).map_err(failed("copying", from))
}

#[cfg(windows)]
fn copy_link(from: &Path, to: &Path) -> Result<(), CopyFailure> {
    let link = fs::read_link(from).map_err(failed("reading", from))?;
    let created = if fs::metadata(from).is_ok_and(|metadata| metadata.is_dir()) {
        std::os::windows::fs::symlink_dir(link, to)
    } else {
        std::os::windows::fs::symlink_file(link, to)
    };
    created.map_err(failed("copying", from))
}

#[cfg(not(any(unix, windows)))]
fn copy_link(from: &Path, to: &Path) -> Result<(), CopyFailure> {
    fs::copy(from, to)
        .map(|_| ())
        .map_err(failed("copying", from))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn copies_files_links_and_directory_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let from = dir.path().join("from");
        fs::create_dir_all(from.join("bin")).unwrap();
        fs::write(from.join("bin/tool"), b"tool").unwrap();
        fs::set_permissions(from.join("bin/tool"), fs::Permissions::from_mode(0o755)).unwrap();
        std::os::unix::fs::symlink("bin/tool", from.join("link")).unwrap();
        fs::set_permissions(&from, fs::Permissions::from_mode(0o750)).unwrap();
        let to = dir.path().join("to");
        copy_tree(&from, &to).unwrap();
        assert_eq!(fs::read(to.join("bin/tool")).unwrap(), b"tool");
        let mode = fs::metadata(to.join("bin/tool"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o755);
        assert_eq!(
            fs::read_link(to.join("link")).unwrap(),
            Path::new("bin/tool")
        );
        let mode = fs::metadata(&to).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o750);
        let refused = copy_tree(&from, &to).unwrap_err();
        assert_eq!(refused.action, "creating");
    }
}
