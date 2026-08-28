//! Generated ignore rules for build output created inside dependency slots.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD");

use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use specmark::spec;

use crate::WorkspaceError;
use crate::safe_file::{self, FileIdentity};

/// Recursive build-output rules maintained at the dependency-root boundary.
pub const BUILD_OUTPUT_IGNORES: [&str; 2] = ["**/target/", "**/node_modules/"];

const MANAGED_HEADER: &str = "# Build output produced inside materialised dependency slots.\n\
# Managed by vibe; additional entries are preserved until `vibe clean`.\n";

/// Ensure the dependency root ignores every build-output directory vibe owns.
///
/// Existing bytes are an immutable prefix: comments, custom entries, invalid
/// UTF-8, and line-ending style are never re-rendered. The two managed positive
/// patterns are maintained as the final effective pattern suffix, so a later
/// operator negation or re-inclusion is re-overridden without deleting it. A
/// fresh/empty file receives the managed header and both rules with LF endings.
/// Returns `true` exactly when bytes were appended.
///
/// The first complete read returns without opening a writable handle. An
/// incomplete file is reopened without following links, exclusively locked,
/// identity-checked, and reread before append so concurrent cooperating slot
/// builds cannot duplicate entries or redirect a write through an alias.
///
/// ```
/// use vibe_workspace::vibedeps::{BUILD_OUTPUT_IGNORES, ensure_build_output_ignores};
///
/// let project = tempfile::tempdir().unwrap();
/// let root = project.path().join("vibedeps");
/// assert!(ensure_build_output_ignores(&root).unwrap());
/// assert!(!ensure_build_output_ignores(&root).unwrap());
/// let text = std::fs::read_to_string(root.join(".gitignore")).unwrap();
/// assert!(BUILD_OUTPUT_IGNORES.iter().all(|rule| text.lines().any(|line| line == *rule)));
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-054#IN-SLOT-BUILD")]
pub fn ensure_build_output_ignores(vibedeps_root: &Path) -> Result<bool, WorkspaceError> {
    let path = vibedeps_root.join(".gitignore");
    if let ReadSnapshot::Complete(existing) = read_existing(&path)?
        && rules_to_append(&existing).is_empty()
    {
        return Ok(false);
    }

    fs::create_dir_all(vibedeps_root).map_err(|error| io_err(vibedeps_root, error))?;
    let mut file = open_regular_for_append(&path)?;
    file.lock().map_err(|error| io_err(&path, error))?;
    let result = append_missing_locked(&mut file, &path);
    let unlock = file.unlock().map_err(|error| io_err(&path, error));
    match (result, unlock) {
        (Err(error), _) | (Ok(_), Err(error)) => Err(error),
        (Ok(changed), Ok(())) => Ok(changed),
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ReadSnapshot {
    Absent,
    Contended,
    Complete(Vec<u8>),
}

fn read_existing(path: &Path) -> Result<ReadSnapshot, WorkspaceError> {
    preflight_path_kind(path)?;
    let mut file = match safe_file::open_existing_read(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ReadSnapshot::Absent);
        }
        Err(error) => return Err(io_err(path, error)),
    };
    let identity = handle_identity(&file, path)?;
    let mut bytes = Vec::new();
    match file.read_to_end(&mut bytes) {
        Ok(_) => {}
        // `File::lock` is advisory on Unix but a mandatory byte-range lock on
        // Windows. Another cooperating writer can acquire it between this
        // optional handle open and its read, producing ERROR_LOCK_VIOLATION.
        // The fast path has learned nothing in that case: discard any partial
        // bytes and continue through the writable handle + blocking lock below,
        // where the file is reread after the writer completes.
        Err(error) if safe_file::is_lock_violation(&error) => {
            return Ok(ReadSnapshot::Contended);
        }
        Err(error) => return Err(io_err(path, error)),
    }
    assert_path_identity(path, identity)?;
    Ok(ReadSnapshot::Complete(bytes))
}

fn open_regular_for_append(path: &Path) -> Result<File, WorkspaceError> {
    loop {
        preflight_path_kind(path)?;
        match safe_file::open_existing_append(path) {
            Ok(file) => return Ok(file),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                match safe_file::create_new_append(path) {
                    Ok(file) => return Ok(file),
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(error) => return Err(io_err(path, error)),
                }
            }
            Err(error) => return Err(io_err(path, error)),
        }
    }
}

fn preflight_path_kind(path: &Path) -> Result<(), WorkspaceError> {
    safe_file::preflight_absent_or_regular(path).map_err(|error| io_err(path, error))
}

fn append_missing_locked(file: &mut File, path: &Path) -> Result<bool, WorkspaceError> {
    let opened_identity = handle_identity(file, path)?;
    assert_path_identity(path, opened_identity)?;
    file.seek(SeekFrom::Start(0))
        .map_err(|error| io_err(path, error))?;
    let mut existing = Vec::new();
    file.read_to_end(&mut existing)
        .map_err(|error| io_err(path, error))?;
    let rules = rules_to_append(&existing);
    if rules.is_empty() {
        assert_path_identity(path, opened_identity)?;
        return Ok(false);
    }

    let appended = render_append(&existing, &rules);
    if handle_identity(file, path)? != opened_identity {
        return invariant(
            path,
            "open `.gitignore` handle identity changed before append",
        );
    }
    assert_path_identity(path, opened_identity)?;
    file.write_all(&appended)
        .and_then(|()| file.flush())
        .map_err(|error| io_err(path, error))?;
    if handle_identity(file, path)? != opened_identity {
        return invariant(
            path,
            "open `.gitignore` handle identity changed after append",
        );
    }
    assert_path_identity(path, opened_identity)?;
    Ok(true)
}

fn handle_identity(file: &File, path: &Path) -> Result<FileIdentity, WorkspaceError> {
    safe_file::identity(file).map_err(|error| io_err(path, error))
}

fn assert_path_identity(path: &Path, expected: FileIdentity) -> Result<(), WorkspaceError> {
    preflight_path_kind(path)?;
    let file = safe_file::open_existing_read(path).map_err(|error| io_err(path, error))?;
    let actual = handle_identity(&file, path)?;
    if actual != expected {
        return invariant(
            path,
            "pathname no longer names the locked `.gitignore` handle; refusing a substituted path",
        );
    }
    Ok(())
}

fn rules_to_append(existing: &[u8]) -> Vec<&'static str> {
    let effective: Vec<&[u8]> = existing
        .split(|byte| *byte == b'\n')
        .filter_map(effective_pattern)
        .collect();
    let target = BUILD_OUTPUT_IGNORES[0].as_bytes();
    let node = BUILD_OUTPUT_IGNORES[1].as_bytes();

    if effective.len() >= 2 {
        let tail = &effective[effective.len() - 2..];
        if matches!(tail, [left, right] if (*left == target && *right == node) || (*left == node && *right == target))
        {
            return Vec::new();
        }
    }
    match effective.last().copied() {
        Some(last) if last == target => vec![BUILD_OUTPUT_IGNORES[1]],
        Some(last) if last == node => vec![BUILD_OUTPUT_IGNORES[0]],
        _ => BUILD_OUTPUT_IGNORES.to_vec(),
    }
}

fn effective_pattern(line: &[u8]) -> Option<&[u8]> {
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    if line.is_empty() || line.iter().all(|byte| *byte == b' ') || line.starts_with(b"#") {
        None
    } else {
        Some(line)
    }
}

fn render_append(existing: &[u8], rules: &[&str]) -> Vec<u8> {
    if existing.is_empty() {
        let mut output = MANAGED_HEADER.as_bytes().to_vec();
        for rule in BUILD_OUTPUT_IGNORES {
            output.extend_from_slice(rule.as_bytes());
            output.push(b'\n');
        }
        return output;
    }

    let newline: &[u8] = if existing.windows(2).any(|window| window == b"\r\n") {
        b"\r\n"
    } else {
        b"\n"
    };
    let mut output = Vec::new();
    if existing.ends_with(b"\r") {
        output.push(b'\n');
    } else if !existing.ends_with(b"\n") {
        output.extend_from_slice(newline);
    }
    for rule in rules {
        output.extend_from_slice(rule.as_bytes());
        output.extend_from_slice(newline);
    }
    output
}

fn invariant<T>(path: &Path, reason: &str) -> Result<T, WorkspaceError> {
    Err(WorkspaceError::Io {
        path: path.to_path_buf(),
        reason: reason.to_string(),
    })
}

fn io_err(path: &Path, error: std::io::Error) -> WorkspaceError {
    WorkspaceError::Io {
        path: PathBuf::from(path),
        reason: error.to_string(),
    }
}

#[cfg(test)]
#[path = "build_ignore/tests.rs"]
mod tests;
