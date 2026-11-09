//! The upward search for a declared version file
//! (PROP-075 `#VERSION-FILE-FORMATS`): from the working directory upward,
//! stopping at the project root, or at the filesystem root outside a
//! project; the declaration order of a `names` array breaks a tie inside one
//! directory, and the nearest directory wins over any further one.
//!
//! The search never fails: an unreadable file or a line that is neither a
//! requirement nor a channel is a reported fault, because a prompt hook must
//! never fail a `cd` (PROP-075 `#ACTIVATION-HOOK`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS");

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use vibe_core::manifest::VersionFileDecl;

use super::declared::Declared;
use super::formats::{VersionRequest, plain_line, tool_versions};

/// One path the search tested, with what it saw. An absent path is recorded
/// too: the resolution is only a cache hit while it is still absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Consulted {
    pub path: PathBuf,
    /// The modification time in nanoseconds since the Unix epoch and the
    /// size in bytes; `None` when the file did not exist.
    pub stamp: Option<(u128, u64)>,
}

impl Consulted {
    /// Observe `path` now.
    pub fn observe(path: PathBuf) -> Self {
        let stamp = fs::metadata(&path).ok().and_then(|meta| {
            let modified = meta
                .modified()
                .ok()?
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?
                .as_nanos();
            Some((modified, meta.len()))
        });
        Self { path, stamp }
    }

    /// Whether what is on disk now still matches what was recorded.
    pub fn unchanged(&self) -> bool {
        Self::observe(self.path.clone()).stamp == self.stamp
    }
}

/// What one slot's nearest declared version file says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub slot: String,
    pub path: PathBuf,
    pub request: VersionRequest,
    /// The versions a `.tool-versions` line listed after the one taken.
    pub unused: Vec<String>,
}

/// The whole outcome of one search.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Search {
    /// At most one per slot.
    pub found: Vec<Found>,
    pub consulted: Vec<Consulted>,
    /// A file that could not be read, or whose content is neither a
    /// requirement nor a channel. Reported, never fatal.
    pub faults: Vec<String>,
}

impl Search {
    pub fn of_slot(&self, slot: &str) -> Option<&Found> {
        self.found.iter().find(|found| found.slot == slot)
    }
}

/// The directories to test, nearest first: `from` and each parent, stopping
/// after `stop_at` when `from` is inside it.
fn upward(from: &Path, stop_at: Option<&Path>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut current = Some(from);
    while let Some(dir) = current {
        dirs.push(dir.to_path_buf());
        if stop_at.is_some_and(|stop| dir == stop) {
            break;
        }
        current = dir.parent();
    }
    dirs
}

/// Search for every slot of `declared`, from `from` upward.
pub fn search(from: &Path, stop_at: Option<&Path>, declared: &Declared) -> Search {
    let mut outcome = Search::default();
    // The slots still looking, each with its rows in declaration order.
    let mut wanted: BTreeMap<String, Vec<VersionFileDecl>> = declared
        .slots()
        .into_iter()
        .map(|slot| {
            let rows = declared.for_slot(&slot);
            (slot, rows)
        })
        .filter(|(_, rows)| !rows.is_empty())
        .collect();
    if wanted.is_empty() {
        return outcome;
    }
    for dir in upward(from, stop_at) {
        if wanted.is_empty() {
            break;
        }
        // One directory: every still-wanted slot reads it, so a path is
        // observed once however many slots name it.
        let mut seen: BTreeMap<PathBuf, Consulted> = BTreeMap::new();
        let mut decided: Vec<String> = Vec::new();
        for (slot, rows) in &wanted {
            if let Some(found) = in_directory(&dir, slot, rows, &mut seen, &mut outcome.faults) {
                outcome.found.push(found);
                decided.push(slot.clone());
            }
        }
        outcome.consulted.extend(seen.into_values());
        for slot in decided {
            wanted.remove(&slot);
        }
    }
    outcome.found.sort_by(|a, b| a.slot.cmp(&b.slot));
    outcome
        .consulted
        .sort_by(|a, b| a.path.cmp(&b.path).then(a.stamp.cmp(&b.stamp)));
    outcome.consulted.dedup();
    outcome
}

/// One slot's rows against one directory, in declaration order.
fn in_directory(
    dir: &Path,
    slot: &str,
    rows: &[VersionFileDecl],
    seen: &mut BTreeMap<PathBuf, Consulted>,
    faults: &mut Vec<String>,
) -> Option<Found> {
    for row in rows {
        for name in row.names() {
            let path = dir.join(name);
            let consulted = seen
                .entry(path.clone())
                .or_insert_with(|| Consulted::observe(path.clone()));
            if consulted.stamp.is_none() {
                continue;
            }
            let text = match fs::read_to_string(&path) {
                Ok(text) => text,
                Err(error) => {
                    faults.push(format!("`{}` could not be read: {error}", path.display()));
                    continue;
                }
            };
            let (authored, unused) = match row {
                VersionFileDecl::Plain { .. } => match plain_line(&text) {
                    Some(line) => (line.to_owned(), Vec::new()),
                    None => continue,
                },
                VersionFileDecl::ToolVersions { key } => match tool_versions(&text, key) {
                    Some(parsed) => (parsed.taken, parsed.unused),
                    None => continue,
                },
            };
            match VersionRequest::parse(&authored) {
                Ok(request) => {
                    return Some(Found {
                        slot: slot.to_owned(),
                        path,
                        request,
                        unused,
                    });
                }
                Err(reason) => faults.push(format!("`{}`: {reason}", path.display())),
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_search_walks_upward_and_stops_at_the_project_root() {
        let root = Path::new("/a/b");
        assert_eq!(
            upward(Path::new("/a/b/c/d"), Some(root)),
            vec![
                PathBuf::from("/a/b/c/d"),
                PathBuf::from("/a/b/c"),
                PathBuf::from("/a/b")
            ]
        );
        // Outside a project the walk reaches the filesystem root.
        let free = upward(Path::new("/a/b/c"), None);
        assert_eq!(free.last().unwrap(), Path::new("/"));
        // A stop directory the start is not inside does not truncate the
        // walk, it just never matches.
        assert_eq!(
            upward(Path::new("/x/y"), Some(root)).last().unwrap(),
            Path::new("/")
        );
    }
}
