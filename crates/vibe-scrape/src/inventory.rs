//! Deterministic no-follow project inventory.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-056#IMPL-A");

use sha2::{Digest, Sha256};
use vibe_safefs::{Pinned, Project};

use crate::model::{EntryKind, Inventory, InventoryEntry, ScrapeError};

pub fn collect(project: &Project) -> Result<Inventory, ScrapeError> {
    let root = project.root_dir().map_err(inventory_error)?;
    let mut entries = Vec::new();
    walk(project, &root, "", &mut entries)?;
    entries.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
    let tree_digest = digest_entries(&entries);
    Ok(Inventory {
        entries,
        tree_digest,
    })
}

fn walk(
    project: &Project,
    directory: &Pinned,
    prefix: &str,
    entries: &mut Vec<InventoryEntry>,
) -> Result<(), ScrapeError> {
    let mut names = project.child_names(directory).map_err(inventory_error)?;
    names.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    for name in &names {
        if prefix.is_empty() && name == ".git" {
            continue;
        }
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        match project.stable_file_state_with_identity(&path) {
            Ok(Some((state, identity))) => entries.push(InventoryEntry {
                path,
                kind: EntryKind::File,
                sha256: Some(format!("sha256:{}", state.sha256)),
                bytes: Some(state.bytes),
                unix_mode: state.unix_mode,
                identity: Some(identity),
            }),
            Ok(None) => {
                return Err(ScrapeError::inventory(format!(
                    "`{path}` disappeared during inventory"
                )));
            }
            Err(file_error) => match directory.open_child_checked(name) {
                Ok(Some(child)) => {
                    let identity = child.identity().map_err(inventory_error)?;
                    let unix_mode = child.unix_mode().map_err(inventory_error)?;
                    entries.push(InventoryEntry {
                        path: path.clone(),
                        kind: EntryKind::Directory,
                        sha256: None,
                        bytes: None,
                        unix_mode,
                        identity: Some(identity),
                    });
                    walk(project, &child, &path, entries)?;
                }
                Ok(None) => {
                    return Err(ScrapeError::inventory(format!(
                        "`{path}` disappeared during inventory"
                    )));
                }
                Err(dir_error) => {
                    return Err(ScrapeError::inventory(format!(
                        "`{path}` is not a stable regular file or a no-follow directory (file: {file_error:#}; directory: {dir_error:#})"
                    )));
                }
            },
        }
    }
    // A second listing catches additions/removals while the retained directory
    // capability is live. File stability is independently proved by safefs.
    let mut after = project.child_names(directory).map_err(inventory_error)?;
    after.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    if prefix.is_empty() {
        after.retain(|name| name != ".git");
        names_without_git(&mut names);
    }
    if names != after {
        return Err(ScrapeError::inventory(format!(
            "directory `{}` changed during inventory",
            if prefix.is_empty() { "." } else { prefix }
        )));
    }
    Ok(())
}

fn names_without_git(names: &mut Vec<String>) {
    names.retain(|name| name != ".git");
}

fn digest_entries(entries: &[InventoryEntry]) -> String {
    let mut hash = Sha256::new();
    for entry in entries {
        hash.update(match entry.kind {
            EntryKind::File => b"f\0",
            EntryKind::Directory => b"d\0",
        });
        hash.update(entry.path.as_bytes());
        hash.update(b"\0");
        if let Some(digest) = &entry.sha256 {
            hash.update(digest.as_bytes());
        }
        hash.update(b"\0");
        if let Some(bytes) = entry.bytes {
            hash.update(bytes.to_be_bytes());
        }
        hash.update(b"\0");
        if let Some(mode) = entry.unix_mode {
            hash.update(mode.to_be_bytes());
        }
        hash.update(b"\n");
    }
    format!("sha256:{:x}", hash.finalize())
}

fn inventory_error(error: anyhow::Error) -> ScrapeError {
    ScrapeError::inventory(format!("{error:#}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_is_sorted_and_git_is_reserved() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir(temp.path().join(".git")).unwrap();
        std::fs::write(temp.path().join(".git/config"), "secret").unwrap();
        std::fs::create_dir(temp.path().join("z")).unwrap();
        std::fs::write(temp.path().join("z/b"), "b").unwrap();
        std::fs::write(temp.path().join("a"), "a").unwrap();
        let project = Project::open(temp.path()).unwrap();
        let inventory = collect(&project).unwrap();
        assert_eq!(
            inventory
                .entries
                .iter()
                .map(|e| e.path.as_str())
                .collect::<Vec<_>>(),
            ["a", "z", "z/b"]
        );
    }
}
