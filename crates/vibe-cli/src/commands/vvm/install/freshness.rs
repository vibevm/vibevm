//! Conservative source-build reuse evidence, private to the install pipeline.
//! Ignored materialized dependencies and configuration outside the Git root
//! or in separate nested Git worktrees are not covered: callers use `--force`
//! when those inputs change.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#SOURCE-FRESHNESS");

use std::collections::BTreeSet;
#[cfg(unix)]
use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

use super::super::store::open_regular_no_follow;

#[derive(Clone, PartialEq, Eq)]
pub(super) struct SourceSnapshot(pub(super) String);

impl SourceSnapshot {
    /// Any uncertainty disables the optimization, never the ordinary build.
    pub(super) fn capture(
        root: &Path,
        commit: &str,
        build_environment: &[(String, Option<std::ffi::OsString>)],
    ) -> Option<Self> {
        if redirected(&fs::symlink_metadata(root).ok()?) {
            return None;
        }
        let root = root.canonicalize().ok()?;
        let top = git(&root, &["rev-parse", "--show-toplevel"])?;
        let top = path_from_bytes(trim_line(&top))?.canonicalize().ok()?;
        if top != root || trim_line(&git(&root, &["rev-parse", "HEAD"])?) != commit.as_bytes() {
            return None;
        }
        let tracked = git(&root, &["ls-files", "--stage", "-z"])?;
        let untracked = git(&root, &["ls-files", "--others", "--exclude-standard", "-z"])?;
        let mut paths = BTreeSet::new();
        let mut external_worktrees = Vec::new();
        for entry in tracked
            .split(|byte| *byte == 0)
            .filter(|entry| !entry.is_empty())
        {
            let tab = entry.iter().position(|byte| *byte == b'\t')?;
            let header = &entry[..tab];
            // Symlinks, gitlinks and unresolved index stages cannot prove a
            // complete regular-file input tree without following outsiders.
            if !(header.starts_with(b"100644 ") || header.starts_with(b"100755 "))
                || !header.ends_with(b" 0")
            {
                return None;
            }
            paths.insert(entry[tab + 1..].to_vec());
        }
        for path in untracked
            .split(|byte| *byte == 0)
            .filter(|path| !path.is_empty())
        {
            let relative = path_from_bytes(path)?;
            if external_worktree(&root, &relative)? {
                // Git emits a nested repository as one opaque directory,
                // outside this worktree's regular-file namespace.
                external_worktrees.push(relative);
            } else {
                paths.insert(path.to_vec());
            }
        }
        let mut hash = Sha256::new();
        field(&mut hash, b"vvm-source-inputs-v1");
        field(&mut hash, &os_bytes(root.as_os_str())?);
        field(&mut hash, commit.as_bytes());
        // Index object IDs are staging bookkeeping, not build inputs. Keep
        // the raw listing only for the concurrent-index-change check below.
        for (name, value) in build_environment {
            field(&mut hash, name.as_bytes());
            match value {
                Some(value) => {
                    field(&mut hash, b"present");
                    field(&mut hash, &os_bytes(value)?);
                }
                None => field(&mut hash, b"absent"),
            }
        }
        for raw in paths {
            let relative = path_from_bytes(&raw)?;
            if relative.as_os_str().is_empty()
                || relative
                    .components()
                    .any(|c| !matches!(c, Component::Normal(_)))
            {
                return None;
            }
            field(&mut hash, &raw);
            let path = root.join(&relative);
            let mut cursor = root.clone();
            let mut missing = false;
            for component in relative.components() {
                cursor.push(component.as_os_str());
                match fs::symlink_metadata(&cursor) {
                    Ok(meta) if redirected(&meta) => return None,
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        missing = true;
                        break;
                    }
                    Err(_) => return None,
                }
            }
            if missing {
                field(&mut hash, b"deleted");
                continue;
            }
            let (mut file, metadata) = open_regular_no_follow(&path).ok()?;
            field(&mut hash, b"regular");
            field(&mut hash, &metadata.len().to_le_bytes());
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                field(&mut hash, &metadata.permissions().mode().to_le_bytes());
            }
            let mut content = Sha256::new();
            let mut buffer = [0; 64 * 1024];
            loop {
                let read = file.read(&mut buffer).ok()?;
                if read == 0 {
                    break;
                }
                content.update(&buffer[..read]);
            }
            let after = file.metadata().ok()?;
            let current = fs::symlink_metadata(&path).ok()?;
            if !same_file_metadata(&metadata, &after)? || !same_file_metadata(&after, &current)? {
                return None;
            }
            // A regular leaf must still have regular directory ancestors;
            // checking only the leaf could follow an ancestor redirected
            // during hashing.
            let mut cursor = root.clone();
            for component in relative.components() {
                cursor.push(component.as_os_str());
                if redirected(&fs::symlink_metadata(&cursor).ok()?) {
                    return None;
                }
            }
            field(&mut hash, &content.finalize());
        }
        // Do not seal a changing file set or revision.
        if git(&root, &["ls-files", "--stage", "-z"])? != tracked
            || git(&root, &["ls-files", "--others", "--exclude-standard", "-z"])? != untracked
            || trim_line(&git(&root, &["rev-parse", "HEAD"])?) != commit.as_bytes()
        {
            return None;
        }
        for relative in external_worktrees {
            if !external_worktree(&root, &relative)? {
                return None;
            }
        }
        Some(Self(format!("{:x}", hash.finalize())))
    }
}

/// Only an ordinary directory with its own verified Git worktree boundary
/// may be omitted. Unknown directories and redirects stay unprovable.
fn external_worktree(root: &Path, relative: &Path) -> Option<bool> {
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return None;
    }
    let mut path = root.to_path_buf();
    for component in relative.components() {
        path.push(component.as_os_str());
        match fs::symlink_metadata(&path) {
            Ok(metadata) if redirected(&metadata) => return None,
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Some(false),
            Err(_) => return None,
        }
    }
    if !fs::symlink_metadata(&path).ok()?.is_dir() {
        return Some(false);
    }
    let marker = fs::symlink_metadata(path.join(".git")).ok()?;
    if redirected(&marker) {
        return None;
    }
    // A regular gitfile supports linked worktrees; Git itself resolves it.
    // Merely finding a .git marker would also accept malformed repositories
    // or a command that discovered the parent repository instead.
    let top = git(&path, &["rev-parse", "--show-toplevel"])?;
    let top = path_from_bytes(trim_line(&top))?.canonicalize().ok()?;
    if top != path.canonicalize().ok()? {
        return None;
    }
    // Recheck the pathname after Git's inspection before excluding it.
    let mut cursor = root.to_path_buf();
    for component in relative.components() {
        cursor.push(component.as_os_str());
        let metadata = fs::symlink_metadata(&cursor).ok()?;
        if redirected(&metadata) || !metadata.is_dir() {
            return None;
        }
    }
    if redirected(&fs::symlink_metadata(path.join(".git")).ok()?) {
        return None;
    }
    Some(true)
}

fn same_file_metadata(before: &fs::Metadata, after: &fs::Metadata) -> Option<bool> {
    if redirected(after) || !after.file_type().is_file() {
        return Some(false);
    }
    let same = before.len() == after.len()
        && before.modified().ok()? == after.modified().ok()?
        && before.permissions() == after.permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Some(same && before.dev() == after.dev() && before.ino() == after.ino())
    }
    #[cfg(not(unix))]
    Some(same)
}

fn field(hash: &mut Sha256, value: &[u8]) {
    hash.update((value.len() as u64).to_le_bytes());
    hash.update(value);
}

fn git(root: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .ok()?;
    output.status.success().then_some(output.stdout)
}

fn trim_line(value: &[u8]) -> &[u8] {
    let value = value.strip_suffix(b"\n").unwrap_or(value);
    value.strip_suffix(b"\r").unwrap_or(value)
}

#[cfg(unix)]
fn path_from_bytes(value: &[u8]) -> Option<PathBuf> {
    use std::os::unix::ffi::OsStringExt;
    Some(PathBuf::from(OsString::from_vec(value.to_vec())))
}

#[cfg(not(unix))]
fn path_from_bytes(value: &[u8]) -> Option<PathBuf> {
    Some(PathBuf::from(std::str::from_utf8(value).ok()?))
}

#[cfg(unix)]
fn os_bytes(value: &std::ffi::OsStr) -> Option<Vec<u8>> {
    use std::os::unix::ffi::OsStrExt;
    Some(value.as_bytes().to_vec())
}

#[cfg(not(unix))]
fn os_bytes(value: &std::ffi::OsStr) -> Option<Vec<u8>> {
    Some(value.to_str()?.as_bytes().to_vec())
}

fn redirected(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    metadata.file_type().is_symlink()
        || (!metadata.file_type().is_file() && !metadata.file_type().is_dir())
}

#[cfg(test)]
mod tests;
