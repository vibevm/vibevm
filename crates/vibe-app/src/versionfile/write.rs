//! `--write-version-file`: writing the **resolved** version into a declared
//! version file (PROP-075 `#VERSION-FILE-WRITE`), which is nvm's `--save`.
//!
//! Two shapes, one rule each. A `plain` file is the one line and nothing
//! else. A `keyed` file is an **in-place edit of one line**: every other
//! line — other keys, comments, blank lines, unknown content, the file's
//! own line endings — is preserved byte for byte, which is what lets two
//! slots write one `.sdkmanrc` without either destroying the other's
//! answer. That is why the rewrite goes through
//! [`super::formats::keyed_lines`] rather than through a parse and a
//! re-serialisation: a round trip through a parser would quietly normalise
//! the spacing of every line it did not mean to touch.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE");

use std::fs;
use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_core::manifest::VersionFileDecl;

use super::formats::keyed_lines;
use crate::error::VersionFileError;

/// What a write was asked to do. The caller supplies the facts, so nothing
/// here consults a home, an index or the working directory.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE")]
pub struct WriteRequest<'a> {
    /// The slot whose version is written, for the refusals that name it.
    pub slot: &'a str,
    /// The package providing the slot, for the same reason.
    pub package: &'a str,
    /// The **resolved** version — never the requirement or the channel the
    /// user typed, because a version file that says `lts` tells the next
    /// reader nothing about what was chosen.
    pub version: &'a str,
    /// The rows the slot's package declares, in declaration order.
    pub rows: &'a [VersionFileDecl],
    /// `--write-version-file=PATH`, when a path was given.
    pub path: Option<&'a Path>,
    /// The nearest file the `#VERSION-FILE-FORMATS` search already found
    /// for this slot, when it found one.
    pub found: Option<&'a Path>,
    /// Where a file is created when no path was given and the search found
    /// none: the project root inside a project, the working directory
    /// otherwise.
    pub create_in: &'a Path,
}

/// What one write did (PROP-075 `#VERSION-FILE-WRITE`): reported by path
/// with the version written, in the human line and in `--json`.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    pub path: PathBuf,
    pub version: String,
    /// The `format` of the row that decided how it was written.
    pub format: &'static str,
    /// The file did not exist and was created holding one line.
    pub created: bool,
    /// The file existed and did not carry the key, so the line was
    /// appended.
    pub appended: bool,
}

/// Write `request.version` into the slot's declared version file.
///
/// ```
/// use std::fs;
/// use vibe_app::{WriteRequest, write_version_file};
/// use vibe_core::manifest::VersionFileDecl;
///
/// let dir = tempfile::tempdir().unwrap();
/// let rows = [VersionFileDecl::Plain { names: vec![".nvmrc".into()] }];
/// let written = write_version_file(&WriteRequest {
///     slot: "node",
///     package: "org.vibevm.node/node",
///     version: "22.23.4",
///     rows: &rows,
///     path: None,
///     found: None,
///     create_in: dir.path(),
/// })
/// .unwrap();
/// assert!(written.created);
/// assert_eq!(fs::read_to_string(&written.path).unwrap(), "22.23.4\n");
/// ```
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE",
    r = 1
)]
pub fn write(request: &WriteRequest<'_>) -> Result<Written, VersionFileError> {
    let (path, row) = target(request)?;
    // Every refusal lands before any byte is written.
    if fs::symlink_metadata(&path).is_ok_and(|meta| meta.file_type().is_symlink()) {
        return Err(VersionFileError::Symlink {
            path: path.display().to_string(),
        });
    }
    match row {
        VersionFileDecl::Plain { .. } => {
            // `plain` writes the one line and nothing else, keeping a
            // trailing newline.
            let created = !path.exists();
            write_bytes(&path, format!("{}\n", request.version).as_bytes())?;
            Ok(Written {
                created,
                path,
                version: request.version.to_owned(),
                format: row.format(),
                appended: false,
            })
        }
        VersionFileDecl::Keyed { key, syntax, .. } => {
            let existing = match fs::read_to_string(&path) {
                Ok(text) => Some(text),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(source) => {
                    return Err(VersionFileError::Io {
                        action: "reading",
                        path: path.display().to_string(),
                        source,
                    });
                }
            };
            let text = existing.as_deref().unwrap_or_default();
            let lines = keyed_lines(text, key, *syntax);
            if let [first, second, ..] = lines.as_slice() {
                // An editor that guessed which duplicate to keep would
                // silently discard a declaration.
                return Err(VersionFileError::DuplicateKey {
                    path: path.display().to_string(),
                    key: key.clone(),
                    first: first.line,
                    second: second.line,
                });
            }
            let edited = match lines.first() {
                Some(found) => replace_line(text, found.line, &found.rewritten(request.version)),
                // A key the file does not carry is appended as the last
                // line; an absent file is created holding that one line.
                None => append_line(
                    text,
                    &format!("{key}{}{}", syntax.separator.written(), request.version),
                ),
            };
            write_bytes(&path, edited.as_bytes())?;
            Ok(Written {
                created: existing.is_none(),
                appended: existing.is_some() && lines.is_empty(),
                path,
                version: request.version.to_owned(),
                format: row.format(),
            })
        }
    }
}

/// Which file a write lands in, and the row that decides its format
/// (PROP-075 `#VERSION-FILE-WRITE`). Decided here and nowhere else: the
/// rule for an absent file is the one genuinely ambiguous one, so it is
/// read off the fact rather than re-decided per command.
fn target<'a>(
    request: &'a WriteRequest<'a>,
) -> Result<(PathBuf, &'a VersionFileDecl), VersionFileError> {
    let Some(first) = request.rows.first() else {
        return Err(VersionFileError::NoVersionFile {
            slot: request.slot.to_owned(),
            package: request.package.to_owned(),
        });
    };
    // With a value: exactly that path, whose format is decided by matching
    // its file name against the declared names of every row — never a
    // guess at a format.
    if let Some(path) = request.path {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        return match row_claiming(request.rows, &name) {
            Some(row) => Ok((path.to_path_buf(), row)),
            None => Err(VersionFileError::UnknownName {
                path: path.display().to_string(),
                slot: request.slot.to_owned(),
                names: declared_names(request.rows),
            }),
        };
    }
    // Without a value: the nearest file the search already found, so
    // `--save` edits the file the project is actually using.
    if let Some(found) = request.found {
        let name = found
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        if let Some(row) = row_claiming(request.rows, &name) {
            return Ok((found.to_path_buf(), row));
        }
    }
    // With no such file: the first `names` entry of the first declared row.
    // A row with no name is refused by validation, so a row without one here
    // came from a record this build cannot use — the same answer as a slot
    // that declares nothing.
    let Some(name) = first
        .names()
        .first()
        .copied()
        .filter(|name| !name.is_empty())
    else {
        return Err(VersionFileError::NoVersionFile {
            slot: request.slot.to_owned(),
            package: request.package.to_owned(),
        });
    };
    Ok((request.create_in.join(name), first))
}

/// The row that claims `name`, if any. Validation has already refused two
/// rows claiming one name, so the first match is the only one.
fn row_claiming<'a>(rows: &'a [VersionFileDecl], name: &str) -> Option<&'a VersionFileDecl> {
    rows.iter().find(|row| row.names().contains(&name))
}

/// The declared names of every row, in declaration order, for the refusal
/// that names them.
fn declared_names(rows: &[VersionFileDecl]) -> String {
    rows.iter()
        .flat_map(|row| row.names())
        .map(|name| format!("`{name}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Replace line `number` (1-based) and nothing else: every other byte of
/// `text`, its line endings included, is carried over untouched.
fn replace_line(text: &str, number: usize, replacement: &str) -> String {
    let mut out = String::with_capacity(text.len() + replacement.len());
    for (index, line) in split_lines(text).into_iter().enumerate() {
        if index + 1 == number {
            let ending = line.len() - line.trim_end_matches(['\r', '\n']).len();
            out.push_str(replacement);
            out.push_str(&line[line.len() - ending..]);
        } else {
            out.push_str(line);
        }
    }
    out
}

/// Append one line, using the ending the file already uses and giving the
/// previous last line the ending it lacked.
fn append_line(text: &str, line: &str) -> String {
    let ending = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out = text.to_owned();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push_str(ending);
    }
    out.push_str(line);
    out.push_str(ending);
    out
}

/// `text` as its lines with their endings still attached, so a rewrite can
/// put every byte back.
fn split_lines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let at = rest.find('\n').map_or(rest.len(), |at| at + 1);
        let (line, tail) = rest.split_at(at);
        lines.push(line);
        rest = tail;
    }
    lines
}

fn write_bytes(path: &Path, bytes: &[u8]) -> Result<(), VersionFileError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| VersionFileError::Io {
            action: "creating",
            path: parent.display().to_string(),
            source,
        })?;
    }
    fs::write(path, bytes).map_err(|source| VersionFileError::Io {
        action: "writing",
        path: path.display().to_string(),
        source,
    })
}

#[cfg(test)]
#[path = "write_tests.rs"]
mod tests;
