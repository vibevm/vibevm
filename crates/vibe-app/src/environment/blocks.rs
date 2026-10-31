//! The one managed block vibe keeps in each shell startup file
//! (`PROP-065#env-step`): written on first use, rewritten in place, removed
//! when no entry remains; every byte outside the markers is kept as is.

use std::fs;
use std::path::{Path, PathBuf};

pub(super) const BEGIN: &str =
    "# >>> vibe applications environment (managed by vibe, do not edit) >>>";
pub(super) const END: &str = "# <<< vibe applications environment <<<";

/// The startup files under `home` that receive the block, each with
/// whether it uses fish syntax: `~/.profile` always, the others when they
/// exist.
pub(super) fn startup_files(home: &Path) -> Vec<(PathBuf, bool)> {
    let mut files = vec![(home.join(".profile"), false)];
    for (relative, fish) in [
        (".bashrc", false),
        (".zshrc", false),
        (".config/fish/config.fish", true),
    ] {
        let path = home.join(relative);
        if path.is_file() {
            files.push((path, fish));
        }
    }
    files
}

/// The text of the block around `body`.
fn block(body: &str) -> String {
    format!("{BEGIN}\n{body}\n{END}\n")
}

/// The byte range of the block in `text`, end-of-line included.
fn find(text: &str) -> Option<(usize, usize)> {
    let start = text
        .match_indices(BEGIN)
        .map(|(index, _)| index)
        .find(|index| *index == 0 || text.as_bytes()[index - 1] == b'\n')?;
    let end_marker = text[start..].find(END)? + start;
    let mut end = end_marker + END.len();
    if text[end..].starts_with("\r\n") {
        end += 2;
    } else if text[end..].starts_with('\n') {
        end += 1;
    }
    Some((start, end))
}

/// The file the block is written into: a symbolic link's target, so the
/// link itself stays.
fn target(path: &Path) -> PathBuf {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
        }
        _ => path.to_path_buf(),
    }
}

fn read(path: &Path) -> Result<Option<String>, String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("reading `{}`: {error}", path.display())),
    }
}

/// Write `text` through a sibling renamed over `path`, keeping the
/// original permissions.
fn write(path: &Path, text: &str) -> Result<(), String> {
    let permissions = fs::metadata(path)
        .ok()
        .map(|metadata| metadata.permissions());
    let parent = path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|error| format!("creating `{}`: {error}", parent.display()))?;
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let sibling = parent.join(format!(".{name}.vibe-{}.tmp", std::process::id()));
    let written = fs::write(&sibling, text)
        .and_then(|()| match permissions {
            Some(permissions) => fs::set_permissions(&sibling, permissions),
            None => Ok(()),
        })
        .and_then(|()| fs::rename(&sibling, path));
    written.map_err(|error| {
        let _ = fs::remove_file(&sibling);
        format!("writing `{}`: {error}", path.display())
    })
}

/// Make `path` hold exactly one block around `body`.
pub(super) fn ensure(path: &Path, body: &str) -> Result<(), String> {
    let path = target(path);
    let current = read(&path)?.unwrap_or_default();
    let wanted = block(body);
    let next = match find(&current) {
        Some((start, end)) if current[start..end] == wanted => return Ok(()),
        Some((start, end)) => format!("{}{wanted}{}", &current[..start], &current[end..]),
        None if current.is_empty() || current.ends_with('\n') => format!("{current}{wanted}"),
        None => format!("{current}\n{wanted}"),
    };
    write(&path, &next)
}

/// Remove the block from `path`, if it holds one.
pub(super) fn remove(path: &Path) -> Result<(), String> {
    let path = target(path);
    let Some(current) = read(&path)? else {
        return Ok(());
    };
    let Some((start, end)) = find(&current) else {
        return Ok(());
    };
    write(&path, &format!("{}{}", &current[..start], &current[end..]))
}
