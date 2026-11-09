//! Symbolic links inside an archive payload (`PROP-059#installer-separation`).
//!
//! Runtime tarballs carry links such as Node.js's
//! `bin/npm -> ../lib/node_modules/npm/bin/npm-cli.js`. A link is admitted
//! only when it cannot reach, or write, anything outside the payload:
//!
//! * its target is relative, with portable components, and resolves inside
//!   the payload root — a `..` that would climb above it is refused;
//! * neither the link's own path nor its target passes **through** another
//!   link (no write and no resolution through a link), so the lexical
//!   resolution checked here is the one the operating system performs;
//! * a chain of links ends at a file or directory the archive holds, with
//!   no cycle and never at a directory that contains the link itself;
//! * it is created after every plain entry, at a path nothing occupies.
//!
//! Where the platform has no unprivileged links (Windows) a link to a file
//! is materialised as a copy of that file; a link to a directory is not
//! materialised, and the extraction says so in a note.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation");

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};

use specmark::spec;
use vibe_core::manifest::is_windows_unsafe_component;

use super::refuse;
use crate::error::InstallerError;

/// The longest link target an archive may declare.
pub(super) const MAX_TARGET_BYTES: u64 = 4096;

/// How an admitted link becomes part of the payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#installer-separation")]
pub enum LinkMode {
    /// A relative symbolic link with the archive's own target text.
    Symlink,
    /// A copy of the linked file; a directory link is left out with a note.
    Copy,
}

impl LinkMode {
    /// Symbolic links where creating one needs no privilege, copies
    /// elsewhere (Windows).
    pub fn native() -> Self {
        if cfg!(unix) {
            Self::Symlink
        } else {
            Self::Copy
        }
    }
}

/// One link entry of the archive, by its path below the unpack root.
#[derive(Debug, Clone)]
pub(super) struct Link {
    pub path: PathBuf,
    pub name: String,
    pub target: String,
}

/// Validate every link below `base` (the declared payload root, or the
/// whole archive) and create it under `root`, which is the unpacked
/// `base`. Links outside `base` are not part of the payload and are
/// ignored. Returns a note per link the mode could not materialise.
pub(super) fn place(
    root: &Path,
    base: Option<&Path>,
    links: &[Link],
    mode: LinkMode,
) -> Result<Vec<String>, InstallerError> {
    let inside: BTreeMap<PathBuf, &Link> = links
        .iter()
        .filter_map(|link| match base {
            Some(base) => link
                .path
                .strip_prefix(base)
                .ok()
                .map(|path| (path.to_path_buf(), link)),
            None => Some((link.path.clone(), link)),
        })
        .filter(|(path, _)| !path.as_os_str().is_empty())
        .collect();
    let resolved = inside
        .iter()
        .map(|(path, link)| {
            if let Some(outer) = path
                .ancestors()
                .skip(1)
                .find(|ancestor| inside.contains_key(*ancestor))
            {
                return Err(refuse(&format!(
                    "link `{}` lies inside the link `{}`",
                    link.name,
                    outer.display()
                )));
            }
            let target = resolve(root, &inside, path)?;
            if path.starts_with(&target.path) {
                return Err(refuse(&format!(
                    "link `{}` points at a directory that contains it",
                    link.name
                )));
            }
            Ok((path, *link, target))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut notes = Vec::new();
    for (path, link, target) in resolved {
        let destination = root.join(path);
        if fs::symlink_metadata(&destination).is_ok() {
            return Err(refuse(&format!(
                "link `{}` collides with another entry of the archive",
                link.name
            )));
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|source| super::super::io("creating", parent, source))?;
        }
        match mode {
            LinkMode::Symlink => symlink(&link.target, &destination)
                .map_err(|source| super::super::io("linking", &destination, source))?,
            LinkMode::Copy if target.directory => notes.push(format!(
                "the archive link `{}` -> `{}` names a directory; directory links are not \
                 materialised on this platform, so `{}` is absent from the payload",
                link.name,
                link.target,
                path.display()
            )),
            LinkMode::Copy => {
                fs::copy(root.join(&target.path), &destination)
                    .map_err(|source| super::super::io("copying", &destination, source))?;
            }
        }
    }
    Ok(notes)
}

/// The final entry a link names, below the payload root.
#[derive(Debug)]
struct Resolved {
    path: PathBuf,
    directory: bool,
}

/// Follow `start` through any chain of links to a file or directory the
/// archive holds.
fn resolve(
    root: &Path,
    links: &BTreeMap<PathBuf, &Link>,
    start: &Path,
) -> Result<Resolved, InstallerError> {
    let names: BTreeSet<&Path> = links.keys().map(PathBuf::as_path).collect();
    let mut seen = BTreeSet::from([start.to_path_buf()]);
    let mut current = start.to_path_buf();
    loop {
        let Some(link) = links.get(&current) else {
            // Unreachable: `current` is always a key, but a refusal is
            // the safe answer rather than a panic.
            return Err(refuse(&format!("`{}` is not a link", current.display())));
        };
        let next = step(&current, link, &names)?;
        if names.contains(next.as_path()) {
            if !seen.insert(next.clone()) {
                return Err(refuse(&format!(
                    "link `{}` is part of a cycle of links",
                    link.name
                )));
            }
            current = next;
            continue;
        }
        let metadata = fs::symlink_metadata(root.join(&next)).map_err(|_| {
            refuse(&format!(
                "link `{}` -> `{}` names no entry of the payload",
                link.name, link.target
            ))
        })?;
        if !metadata.is_file() && !metadata.is_dir() {
            return Err(refuse(&format!(
                "link `{}` names a special entry",
                link.name
            )));
        }
        return Ok(Resolved {
            path: next,
            directory: metadata.is_dir(),
        });
    }
}

/// Resolve one link's target lexically from its own directory: relative,
/// portable, never above the payload root and never through another link.
fn step(path: &Path, link: &Link, links: &BTreeSet<&Path>) -> Result<PathBuf, InstallerError> {
    let target = link.target.as_str();
    let unsafe_target =
        |reason: &str| refuse(&format!("link `{}` -> `{target}` {reason}", link.name));
    if target.is_empty() || target.starts_with('/') || target.contains('\\') || target.contains(':')
    {
        return Err(unsafe_target("is not a relative portable target"));
    }
    let mut stack: Vec<OsString> = path
        .parent()
        .map(|parent| {
            parent
                .components()
                .map(|part| part.as_os_str().to_owned())
                .collect()
        })
        .unwrap_or_default();
    let parts: Vec<Component<'_>> = Path::new(target).components().collect();
    for (index, part) in parts.iter().enumerate() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                if stack.pop().is_none() {
                    return Err(unsafe_target("climbs out of the payload"));
                }
            }
            Component::Normal(name) => {
                if name.to_str().is_none_or(is_windows_unsafe_component) {
                    return Err(unsafe_target("has an unsafe component"));
                }
                stack.push(name.to_os_string());
                let prefix: PathBuf = stack.iter().collect();
                if index + 1 < parts.len() && links.contains(prefix.as_path()) {
                    return Err(unsafe_target("resolves through another link"));
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(unsafe_target("is not a relative portable target"));
            }
        }
    }
    Ok(stack.iter().collect())
}

#[cfg(unix)]
fn symlink(target: &str, destination: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, destination)
}

#[cfg(not(unix))]
fn symlink(_target: &str, _destination: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "symbolic links are not created on this platform",
    ))
}
