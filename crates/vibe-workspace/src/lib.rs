//! Workspace discovery and the multi-package member model.
//!
//! Spec: [PROP-007](../../../spec/modules/vibe-workspace/PROP-007-workspace.md).
//!
//! A vibevm **workspace** is a tree of nodes — each a directory carrying one
//! `vibe.toml` — coordinated by a `[workspace]` table. The node that owns the
//! `[workspace]` table at the very top is the **absolute root**; the single
//! `vibe.lock` lives there. Members listed under `[workspace].members` are
//! resolved relative to the manifest that declares them, glob patterns are
//! expanded, and a member may itself be a `[workspace]` — nesting recurses to
//! arbitrary depth (PROP-007 §2.3). Nesting is hierarchical grouping, never
//! an independent resolution domain: the lock and the resolution always live
//! at the absolute root.
//!
//! [`Workspace::discover`] is the entry point. Run from anywhere inside a
//! node, it walks **up** the directory tree, finds the topmost `[workspace]`
//! that transitively includes the starting node, and loads the whole tree.
//! A node with no enclosing `[workspace]` is its own absolute root — a
//! **standalone** workspace with no members. Every existing single-package
//! vibevm project is a standalone workspace, so `discover` is the universal
//! entry point: it degenerates gracefully to "just this one node".
//!
//! Discovery never persists an absolute path. A member is identified by its
//! `rel_path` — a forward-slashed path relative to the absolute root — which
//! is portable across machines. Absolute paths exist only in memory, for the
//! duration of a filesystem walk.

#![forbid(unsafe_code)]

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use thiserror::Error;
use vibe_core::manifest::Manifest;

/// Errors raised while discovering or loading a workspace.
#[derive(Debug, Error)]
pub enum WorkspaceError {
    /// No `vibe.toml` exists at or above the starting directory.
    #[error("no `vibe.toml` found at or above `{}`", .start.display())]
    NoManifest { start: PathBuf },

    /// A node's `vibe.toml` failed to read or validate. The inner error is
    /// boxed — `vibe_core::Error` is large, and an unboxed copy would bloat
    /// every `Result` in this crate (`clippy::result_large_err`).
    #[error("manifest at `{}` is invalid", .path.display())]
    Manifest {
        path: PathBuf,
        #[source]
        source: Box<vibe_core::Error>,
    },

    /// A `[workspace].members` entry — an explicit (non-glob) path —
    /// names a directory that does not exist or carries no `vibe.toml`.
    #[error(
        "workspace member `{pattern}` declared in `{declared_in}` does not exist \
         or carries no vibe.toml"
    )]
    MemberNotFound {
        pattern: String,
        declared_in: String,
    },

    /// A member resolved to a directory outside the absolute root. Every
    /// member must live under the root so its `rel_path` is portable.
    #[error("workspace member `{path}` lies outside the workspace root `{root}`")]
    MemberOutsideRoot { path: String, root: String },

    /// A `[workspace]` transitively lists itself — the member graph is not
    /// a tree.
    #[error("workspace nesting cycle: `{path}` is reached more than once")]
    NestingCycle { path: String },

    /// A `members` glob pattern is syntactically invalid.
    #[error("invalid member glob pattern `{pattern}`: {reason}")]
    BadGlob { pattern: String, reason: String },

    /// A filesystem operation failed.
    #[error("I/O error on `{}`: {reason}", .path.display())]
    Io { path: PathBuf, reason: String },
}

type Result<T> = std::result::Result<T, WorkspaceError>;

/// One member node of a workspace — a package directory carrying its own
/// `vibe.toml`, reached transitively from the absolute root.
#[derive(Debug, Clone)]
pub struct WorkspaceMember {
    /// Path relative to the workspace's absolute root, forward-slashed.
    /// This is the member's portable identity — it is what the lockfile
    /// records, never an absolute path.
    pub rel_path: String,
    /// The member's parsed, validated manifest.
    pub manifest: Manifest,
    /// Nesting depth: `0` for a direct member of the absolute root, `1`
    /// for a member of a nested workspace, and so on.
    pub depth: usize,
}

/// A loaded workspace: an absolute root plus every member, transitively.
///
/// Construct one with [`Workspace::discover`] (from anywhere inside the tree)
/// or [`Workspace::load`] (from a known root directory).
#[derive(Debug, Clone)]
pub struct Workspace {
    /// Absolute path of the workspace's root directory. In-memory only —
    /// never persisted; the portable identity of a node is its `rel_path`.
    pub root: PathBuf,
    /// The root node's manifest.
    pub root_manifest: Manifest,
    /// Every member, transitively, sorted by `rel_path` for determinism.
    /// Empty for a standalone node.
    pub members: Vec<WorkspaceMember>,
}

impl Workspace {
    /// Discover the workspace enclosing `start` and load the whole tree.
    ///
    /// Walks up from `start` to the topmost `[workspace]` that transitively
    /// includes the starting node (PROP-007 §2.3). A node with no enclosing
    /// `[workspace]` is its own root — a standalone workspace.
    pub fn discover(start: impl AsRef<Path>) -> Result<Workspace> {
        let start = start.as_ref();
        let start_abs = canonical(start)?;
        let start_node = nearest_manifest_dir(&start_abs).ok_or_else(|| {
            WorkspaceError::NoManifest {
                start: start.to_path_buf(),
            }
        })?;

        // Collect every ancestor (including the start node) whose vibe.toml
        // carries a `[workspace]` table. A malformed / unreadable ancestor
        // manifest is skipped, not fatal — discovery must not break because
        // some unrelated directory higher up has a broken vibe.toml.
        let mut ws_ancestors: Vec<PathBuf> = Vec::new();
        let mut cursor: Option<&Path> = Some(start_node.as_path());
        while let Some(dir) = cursor {
            let mf = dir.join(Manifest::FILENAME);
            if mf.is_file()
                && let Ok(m) = Manifest::read(&mf)
                && m.workspace.is_some()
            {
                ws_ancestors.push(dir.to_path_buf());
            }
            cursor = dir.parent();
        }

        // Topmost first. The first enclosing workspace whose tree contains
        // the start node is the absolute root.
        ws_ancestors.reverse();
        for candidate in &ws_ancestors {
            let ws = Workspace::load(candidate)?;
            if ws.contains_dir(&start_node) {
                return Ok(ws);
            }
        }

        // No enclosing workspace — the start node stands alone.
        Workspace::load(&start_node)
    }

    /// Load a workspace from a known root directory. The root's `vibe.toml`
    /// is read; if it carries `[workspace]`, members are expanded
    /// recursively. A root without `[workspace]` yields a standalone
    /// workspace with no members.
    pub fn load(root_dir: impl AsRef<Path>) -> Result<Workspace> {
        let root = canonical(root_dir.as_ref())?;
        let root_manifest = read_manifest(&root)?;

        let mut members: Vec<WorkspaceMember> = Vec::new();
        if root_manifest.workspace.is_some() {
            let mut visited: HashSet<PathBuf> = HashSet::new();
            visited.insert(root.clone());
            expand(&root, &root_manifest, &root, 0, &mut visited, &mut members)?;
        }
        members.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));

        Ok(Workspace {
            root,
            root_manifest,
            members,
        })
    }

    /// `true` iff this is a standalone node — no `[workspace]` table, no
    /// members. The common shape of a single-package vibevm project.
    pub fn is_standalone(&self) -> bool {
        self.root_manifest.workspace.is_none()
    }

    /// The absolute path of the single `vibe.lock` — always at the root.
    pub fn lockfile_path(&self) -> PathBuf {
        self.root.join("vibe.lock")
    }

    /// Look up a member by its root-relative path (forward-slashed).
    pub fn member_by_rel_path(&self, rel_path: &str) -> Option<&WorkspaceMember> {
        self.members.iter().find(|m| m.rel_path == rel_path)
    }

    /// The absolute on-disk path of a member — `root` joined with its
    /// `rel_path`. In-memory only; do not persist the result.
    pub fn member_abs_path(&self, member: &WorkspaceMember) -> PathBuf {
        join_rel(&self.root, &member.rel_path)
    }

    /// Iterate every node in the workspace — the root first (as `"."`),
    /// then every member — paired with its manifest. The order after the
    /// root is `rel_path`-sorted.
    pub fn iter_nodes(&self) -> impl Iterator<Item = (&str, &Manifest)> {
        std::iter::once((".", &self.root_manifest))
            .chain(self.members.iter().map(|m| (m.rel_path.as_str(), &m.manifest)))
    }

    /// `true` iff `dir` is the root or one of the members.
    fn contains_dir(&self, dir: &Path) -> bool {
        if dir == self.root {
            return true;
        }
        self.members
            .iter()
            .any(|m| self.member_abs_path(m) == dir)
    }
}

// ---------------------------------------------------------------------------
// Recursive member expansion
// ---------------------------------------------------------------------------

fn expand(
    node_dir: &Path,
    node_manifest: &Manifest,
    root: &Path,
    depth: usize,
    visited: &mut HashSet<PathBuf>,
    out: &mut Vec<WorkspaceMember>,
) -> Result<()> {
    let workspace = node_manifest
        .workspace
        .as_ref()
        .expect("expand is only called for nodes carrying [workspace]");

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

            let rel = member_dir.strip_prefix(root).map_err(|_| {
                WorkspaceError::MemberOutsideRoot {
                    path: member_dir.display().to_string(),
                    root: root.display().to_string(),
                }
            })?;
            let rel_path = path_to_slash(rel);

            if !visited.insert(member_dir.clone()) {
                return Err(WorkspaceError::NestingCycle { path: rel_path });
            }

            let manifest = read_manifest(&member_dir)?;
            let nested = manifest.workspace.is_some();
            out.push(WorkspaceMember {
                rel_path,
                manifest: manifest.clone(),
                depth,
            });
            if nested {
                expand(&member_dir, &manifest, root, depth + 1, visited, out)?;
            }
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read and validate the `vibe.toml` in `dir`, mapping any error into
/// [`WorkspaceError::Manifest`] with the manifest path attached.
fn read_manifest(dir: &Path) -> Result<Manifest> {
    let path = dir.join(Manifest::FILENAME);
    Manifest::read(&path).map_err(|source| WorkspaceError::Manifest {
        path,
        source: Box::new(source),
    })
}

/// Canonicalise a path, stripping the Windows `\\?\` UNC prefix so paths
/// compare and display cleanly.
fn canonical(path: &Path) -> Result<PathBuf> {
    let canon = path.canonicalize().map_err(|e| WorkspaceError::Io {
        path: path.to_path_buf(),
        reason: e.to_string(),
    })?;
    Ok(strip_unc(canon))
}

#[cfg(windows)]
fn strip_unc(p: PathBuf) -> PathBuf {
    let s = p.as_os_str().to_string_lossy();
    match s.strip_prefix(r"\\?\") {
        Some(rest) => PathBuf::from(rest),
        None => p,
    }
}

#[cfg(not(windows))]
fn strip_unc(p: PathBuf) -> PathBuf {
    p
}

/// Nearest directory at or above `start` that carries a `vibe.toml`.
fn nearest_manifest_dir(start: &Path) -> Option<PathBuf> {
    let mut cursor: Option<&Path> = if start.is_dir() {
        Some(start)
    } else {
        start.parent()
    };
    while let Some(dir) = cursor {
        if dir.join(Manifest::FILENAME).is_file() {
            return Some(dir.to_path_buf());
        }
        cursor = dir.parent();
    }
    None
}

/// `true` iff a `members` entry is a glob pattern rather than an explicit
/// path. Globs match leniently (a non-matching glob is not an error);
/// explicit paths must resolve.
fn is_glob_pattern(pattern: &str) -> bool {
    pattern.contains(['*', '?', '['])
}

/// Join a forward-slashed relative path onto an absolute base.
fn join_rel(root: &Path, rel: &str) -> PathBuf {
    let mut p = root.to_path_buf();
    for segment in rel.split('/').filter(|s| !s.is_empty()) {
        p.push(segment);
    }
    p
}

/// Render a path as a forward-slashed string.
fn path_to_slash(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// `dir` relative to `root` as a forward-slashed string, or `"."` for the
/// root itself. Used only for human-readable error context.
fn rel_or_dot(root: &Path, dir: &Path) -> String {
    match dir.strip_prefix(root) {
        Ok(rel) if rel.as_os_str().is_empty() => ".".to_string(),
        Ok(rel) => path_to_slash(rel),
        Err(_) => dir.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(dir: &Path, rel: &str, body: &str) {
        let path = dir.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    fn project(name: &str) -> String {
        format!("[project]\nname = \"{name}\"\nversion = \"0.0.1\"\n")
    }

    fn package(name: &str, kind: &str) -> String {
        format!("[package]\nname = \"{name}\"\nkind = \"{kind}\"\nversion = \"0.1.0\"\n")
    }

    fn workspace_root(name: &str, members: &[&str]) -> String {
        let list = members
            .iter()
            .map(|m| format!("\"{m}\""))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "[project]\nname = \"{name}\"\nversion = \"0.0.1\"\n\n[workspace]\nmembers = [{list}]\n"
        )
    }

    #[test]
    fn standalone_project_is_its_own_root() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "vibe.toml", &project("solo"));
        let ws = Workspace::discover(tmp.path()).unwrap();
        assert!(ws.is_standalone());
        assert!(ws.members.is_empty());
        assert_eq!(ws.root_manifest.require_project().unwrap().name, "solo");
    }

    #[test]
    fn explicit_members_load() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "vibe.toml",
            &workspace_root("mono", &["packages/flow-wal", "packages/feat-auth"]),
        );
        write(tmp.path(), "packages/flow-wal/vibe.toml", &package("wal", "flow"));
        write(tmp.path(), "packages/feat-auth/vibe.toml", &package("auth", "feat"));

        let ws = Workspace::load(tmp.path()).unwrap();
        assert!(!ws.is_standalone());
        assert_eq!(ws.members.len(), 2);
        // Sorted by rel_path: feat-auth before flow-wal.
        assert_eq!(ws.members[0].rel_path, "packages/feat-auth");
        assert_eq!(ws.members[1].rel_path, "packages/flow-wal");
        assert_eq!(ws.members[0].depth, 0);
        assert_eq!(
            ws.member_by_rel_path("packages/flow-wal")
                .unwrap()
                .manifest
                .require_package()
                .unwrap()
                .name,
            "wal"
        );
    }

    #[test]
    fn glob_members_expand_and_skip_non_packages() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "vibe.toml", &workspace_root("mono", &["packages/*"]));
        write(tmp.path(), "packages/flow-a/vibe.toml", &package("a", "flow"));
        write(tmp.path(), "packages/flow-b/vibe.toml", &package("b", "flow"));
        // A directory under packages/ with no manifest — a glob match must
        // silently skip it.
        fs::create_dir_all(tmp.path().join("packages/scratch")).unwrap();
        write(tmp.path(), "packages/scratch/notes.txt", "ignore me");

        let ws = Workspace::load(tmp.path()).unwrap();
        assert_eq!(ws.members.len(), 2);
        assert_eq!(ws.members[0].rel_path, "packages/flow-a");
        assert_eq!(ws.members[1].rel_path, "packages/flow-b");
    }

    #[test]
    fn nested_workspace_recurses_with_depth() {
        let tmp = TempDir::new().unwrap();
        // Root lists a sub-workspace as a member.
        write(tmp.path(), "vibe.toml", &workspace_root("mono", &["sub"]));
        // The sub node is itself a [workspace] AND a package.
        write(
            tmp.path(),
            "sub/vibe.toml",
            &format!(
                "{}\n[workspace]\nmembers = [\"leaf\"]\n",
                package("sub", "stack")
            ),
        );
        write(tmp.path(), "sub/leaf/vibe.toml", &package("leaf", "flow"));

        let ws = Workspace::load(tmp.path()).unwrap();
        assert_eq!(ws.members.len(), 2);
        let sub = ws.member_by_rel_path("sub").unwrap();
        assert_eq!(sub.depth, 0);
        let leaf = ws.member_by_rel_path("sub/leaf").unwrap();
        assert_eq!(leaf.depth, 1);
    }

    #[test]
    fn discover_from_member_finds_absolute_root() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "vibe.toml", &workspace_root("mono", &["sub"]));
        write(
            tmp.path(),
            "sub/vibe.toml",
            &format!(
                "{}\n[workspace]\nmembers = [\"leaf\"]\n",
                package("sub", "stack")
            ),
        );
        write(tmp.path(), "sub/leaf/vibe.toml", &package("leaf", "flow"));

        // Discovery from the deepest leaf must bubble up to the absolute root.
        let ws = Workspace::discover(tmp.path().join("sub/leaf")).unwrap();
        assert_eq!(ws.root, canonical(tmp.path()).unwrap());
        assert_eq!(ws.members.len(), 2);
        assert!(!ws.is_standalone());
    }

    #[test]
    fn discover_from_member_of_unrelated_workspace_picks_the_enclosing_one() {
        let tmp = TempDir::new().unwrap();
        // The outer [workspace] does NOT list `sub` — it lists `other`.
        write(tmp.path(), "vibe.toml", &workspace_root("outer", &["other"]));
        write(tmp.path(), "other/vibe.toml", &package("other", "flow"));
        // `sub` is its own [workspace], not reachable from `outer`.
        write(
            tmp.path(),
            "sub/vibe.toml",
            &workspace_root("sub-ws", &["leaf"]),
        );
        write(tmp.path(), "sub/leaf/vibe.toml", &package("leaf", "flow"));

        let ws = Workspace::discover(tmp.path().join("sub/leaf")).unwrap();
        // The enclosing workspace is `sub`, not the unrelated `outer`.
        assert_eq!(ws.root, canonical(&tmp.path().join("sub")).unwrap());
        assert_eq!(ws.members.len(), 1);
        assert_eq!(ws.members[0].rel_path, "leaf");
    }

    #[test]
    fn missing_explicit_member_errors() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "vibe.toml",
            &workspace_root("mono", &["packages/ghost"]),
        );
        let err = Workspace::load(tmp.path()).unwrap_err();
        assert!(matches!(err, WorkspaceError::MemberNotFound { .. }), "{err}");
    }

    #[test]
    fn nesting_cycle_is_detected() {
        let tmp = TempDir::new().unwrap();
        // Root lists `sub`; `sub` lists `..` back to the root directory.
        write(tmp.path(), "vibe.toml", &workspace_root("mono", &["sub"]));
        write(
            tmp.path(),
            "sub/vibe.toml",
            &format!("{}\n[workspace]\nmembers = [\"..\"]\n", package("sub", "flow")),
        );
        let err = Workspace::load(tmp.path()).unwrap_err();
        assert!(matches!(err, WorkspaceError::NestingCycle { .. }), "{err}");
    }

    #[test]
    fn iter_nodes_yields_root_then_members() {
        let tmp = TempDir::new().unwrap();
        write(tmp.path(), "vibe.toml", &workspace_root("mono", &["pkg"]));
        write(tmp.path(), "pkg/vibe.toml", &package("pkg", "flow"));
        let ws = Workspace::load(tmp.path()).unwrap();
        let nodes: Vec<&str> = ws.iter_nodes().map(|(p, _)| p).collect();
        assert_eq!(nodes, vec![".", "pkg"]);
        assert_eq!(ws.lockfile_path(), ws.root.join("vibe.lock"));
    }
}
