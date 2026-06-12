//! Recursive member expansion and `[workspace.versions]` placeholder
//! resolution — the loading machinery behind [`crate::Workspace::load`]
//! (PROP-007 §2.3, §2.6).
//!
//! Split out of the crate root along the discovery-vs-expansion seam;
//! everything here is `pub(crate)` at most, so the public surface is
//! unchanged.

specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-007#nesting");

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_core::manifest::{Manifest, Requires, WorkspaceSection};
use vibe_core::{PackageRef, VersionSpec};

use crate::{
    Result, WorkspaceError, WorkspaceMember, canonical, is_glob_pattern, path_to_slash,
    read_manifest, rel_or_dot,
};

/// Expand one node's `[workspace].members` into [`WorkspaceMember`]s,
/// recursing into nested workspaces (PROP-007 §2.3).
pub(crate) fn expand(
    node_dir: &Path,
    workspace: &WorkspaceSection,
    node_rel: Option<&str>,
    root: &Path,
    depth: usize,
    visited: &mut HashSet<PathBuf>,
    out: &mut Vec<WorkspaceMember>,
) -> Result<()> {
    for pattern in &workspace.members {
        let is_glob = is_glob_pattern(pattern);
        let matched = glob_member_dirs(node_dir, pattern)?;
        let mut found_any = false;

        for member_dir in matched {
            let manifest_path = member_dir.join(Manifest::FILENAME);
            if !manifest_path.is_file() {
                // A glob may legitimately sweep up non-package directories
                // (`packages/.git`, build output) — skip those. An explicit
                // path that names a directory with no manifest is an error.
                if is_glob {
                    continue;
                }
                return Err(WorkspaceError::MemberNotFound {
                    pattern: pattern.clone(),
                    declared_in: rel_or_dot(root, node_dir),
                });
            }
            found_any = true;

            let rel =
                member_dir
                    .strip_prefix(root)
                    .map_err(|_| WorkspaceError::MemberOutsideRoot {
                        path: member_dir.display().to_string(),
                        root: root.display().to_string(),
                    })?;
            let rel_path = path_to_slash(rel);

            if !visited.insert(member_dir.clone()) {
                return Err(WorkspaceError::NestingCycle { path: rel_path });
            }

            let manifest = read_manifest(&member_dir)?;
            // Recurse into a nested workspace before pushing — the recursion
            // borrows `manifest`, then the push moves it. `out` ends up
            // children-before-parent, which the caller's sort normalises.
            if let Some(section) = &manifest.workspace {
                expand(
                    &member_dir,
                    section,
                    Some(&rel_path),
                    root,
                    depth + 1,
                    visited,
                    out,
                )?;
            }
            out.push(WorkspaceMember {
                rel_path,
                manifest,
                depth,
                parent: node_rel.map(str::to_string),
            });
        }

        if !found_any && !is_glob {
            return Err(WorkspaceError::MemberNotFound {
                pattern: pattern.clone(),
                declared_in: rel_or_dot(root, node_dir),
            });
        }
    }
    Ok(())
}

/// Resolve every `version.var` placeholder in the workspace.
///
/// After this pass every node's `[requires].var_packages` is empty and the
/// concrete `PackageRef`s it produced have been folded into `packages`. A
/// placeholder is looked up bottom-up: the node's own `[workspace.versions]`
/// (when the node is itself a workspace), then its declaring workspace, on up
/// to the absolute root — first hit wins. PROP-007 §2.6.
#[spec(
    implements = "spec://vibevm/modules/vibe-workspace/PROP-007#versions",
    r = 1
)]
pub(crate) fn finalize_versions(
    root_manifest: &mut Manifest,
    members: &mut [WorkspaceMember],
) -> Result<()> {
    // Snapshot each node's own [workspace.versions] table and its parent
    // link, keyed by rel_path ("." = the absolute root). The placeholder
    // tables are tiny, so cloning beats fighting the borrow checker.
    let mut own: HashMap<String, BTreeMap<String, String>> = HashMap::new();
    let mut parent: HashMap<String, Option<String>> = HashMap::new();
    if let Some(ws) = &root_manifest.workspace {
        own.insert(".".to_string(), ws.versions.clone());
    }
    parent.insert(".".to_string(), None);
    for m in members.iter() {
        if let Some(ws) = &m.manifest.workspace {
            own.insert(m.rel_path.clone(), ws.versions.clone());
        }
        parent.insert(
            m.rel_path.clone(),
            Some(m.parent.clone().unwrap_or_else(|| ".".to_string())),
        );
    }

    // Walk a node's enclosing chain, nearest first, for the placeholder.
    let resolve = |start: &str, var: &str| -> Option<String> {
        let mut cursor = Some(start.to_string());
        while let Some(node) = cursor {
            if let Some(found) = own.get(&node).and_then(|table| table.get(var)) {
                return Some(found.clone());
            }
            cursor = parent.get(&node).cloned().flatten();
        }
        None
    };

    finalize_one(&mut root_manifest.requires, ".", &resolve)?;
    for m in members.iter_mut() {
        let key = m.rel_path.clone();
        finalize_one(&mut m.manifest.requires, &key, &resolve)?;
    }
    Ok(())
}

/// Fold one node's `var_packages` into `packages`, resolving each placeholder
/// through `resolve`.
fn finalize_one(
    requires: &mut Requires,
    node_key: &str,
    resolve: &impl Fn(&str, &str) -> Option<String>,
) -> Result<()> {
    if requires.var_packages.is_empty() {
        return Ok(());
    }
    let declared_in = if node_key == "." {
        "the workspace root"
    } else {
        node_key
    };
    for dep in std::mem::take(&mut requires.var_packages) {
        let constraint =
            resolve(node_key, &dep.var).ok_or_else(|| WorkspaceError::UnknownVersionVar {
                var: dep.var.clone(),
                declared_in: declared_in.to_string(),
            })?;
        let spec = VersionSpec::parse(&constraint).map_err(|_| WorkspaceError::BadVersionVar {
            var: dep.var.clone(),
            constraint: constraint.clone(),
        })?;
        let pkgref = PackageRef::new(dep.kind, Some(dep.group), dep.name, spec).map_err(|e| {
            WorkspaceError::BadVarDepRef {
                var: dep.var.clone(),
                declared_in: declared_in.to_string(),
                reason: e.to_string(),
            }
        })?;
        requires.packages.push(pkgref);
    }
    Ok(())
}

/// Expand one `members` pattern, relative to `node_dir`, into the set of
/// matching **directories** (canonicalised, sorted, deduplicated).
fn glob_member_dirs(node_dir: &Path, pattern: &str) -> Result<Vec<PathBuf>> {
    let base = node_dir.to_string_lossy().replace('\\', "/");
    let full = format!("{}/{}", base.trim_end_matches('/'), pattern);
    let matches = glob::glob(&full).map_err(|e| WorkspaceError::BadGlob {
        pattern: pattern.to_string(),
        reason: e.to_string(),
    })?;
    let mut dirs: Vec<PathBuf> = Vec::new();
    for entry in matches {
        let path = entry.map_err(|e| WorkspaceError::Io {
            path: node_dir.to_path_buf(),
            reason: e.to_string(),
        })?;
        if path.is_dir() {
            dirs.push(canonical(&path)?);
        }
    }
    dirs.sort();
    dirs.dedup();
    Ok(dirs)
}
