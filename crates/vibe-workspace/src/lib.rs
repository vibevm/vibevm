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
specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-007#nesting");

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use specmark::spec;
use thiserror::Error;
use vibe_core::manifest::{Manifest, Requires, WorkspaceSection};
use vibe_core::{PackageKind, PackageRef, VersionSpec};

pub mod boot;
pub mod boot_artifacts;
pub mod freshness;
pub mod install;
pub mod publish;
pub mod vibedeps;

pub use publish::{
    OriginInfo, PublishNode, Selection, SkippedNode, StagedNode, select_publishable_nodes,
    stage_node, topo_order,
};

/// Errors raised while discovering or loading a workspace.
///
/// Messages carry the offending path or pattern, so the operator knows
/// which manifest to repair:
///
/// ```
/// use vibe_workspace::WorkspaceError;
///
/// let err = WorkspaceError::NestingCycle {
///     path: "packages/a".to_string(),
/// };
/// assert_eq!(
///     err.to_string(),
///     "workspace nesting cycle: `packages/a` is reached more than once",
/// );
/// ```
#[derive(Debug, Error)]
#[spec(implements = "spec://vibevm/modules/vibe-workspace/PROP-007#nesting")]
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

    /// A `version.var` placeholder names no entry in any enclosing
    /// `[workspace.versions]` table.
    #[error(
        "version placeholder `{var}` referenced in `{declared_in}` is defined in no \
         enclosing [workspace.versions]"
    )]
    UnknownVersionVar { var: String, declared_in: String },

    /// A `[workspace.versions]` entry holds an unparseable version constraint.
    #[error("[workspace.versions] placeholder `{var}` has an invalid constraint `{constraint}`")]
    BadVersionVar { var: String, constraint: String },

    /// A `version.var` dependency entry fails `PackageRef` validation when
    /// its placeholder resolves (PROP-007 §2.6) — the `group/name` pair is
    /// not a valid package reference.
    #[error(
        "var-dep for placeholder `{var}` in `{declared_in}` is not a valid \
         package reference: {reason} \
         (violates spec://vibevm/modules/vibe-workspace/PROP-007#versions; \
         fix: use a kebab-case group/name in the [requires] var-dep entry)"
    )]
    BadVarDepRef {
        var: String,
        declared_in: String,
        reason: String,
    },

    /// The generated `spec/boot/INDEX.md` TOML manifest failed to
    /// serialise. Structurally unreachable with today's fixed manifest
    /// shape; routed as an error so a future shape change degrades to a
    /// diagnosis instead of a panic.
    #[error(
        "rendering spec/boot/INDEX.md failed: {reason} \
         (violates spec://vibevm/modules/vibe-workspace/PROP-009#artifacts; \
         fix: the IndexManifest shape no longer serialises as TOML — restore \
         a serialisable shape)"
    )]
    IndexRender { reason: String },

    /// A publish operation referenced a node `rel_path` that names no
    /// node of this workspace — the selection and the loaded workspace
    /// fell out of sync.
    #[error(
        "publish references `{rel_path}`, which is not a node of this workspace \
         (violates spec://vibevm/modules/vibe-workspace/PROP-007#selective-publish; \
         fix: pass a rel_path that names the root `.` or a listed member)"
    )]
    UnknownPublishNode { rel_path: String },

    /// The dependency boot graph handed to the computed-view engine
    /// contains a cycle — a package transitively requires itself.
    #[error("boot dependency cycle among: {packages}")]
    BootDependencyCycle { packages: String },

    /// A `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` carries a malformed
    /// vibevm managed block — not exactly one well-formed `<vibevm>` …
    /// `</vibevm>` pair (PROP-012 §2.3). vibevm never guesses which block
    /// is canonical; the operator repairs the file by hand.
    #[error("malformed <vibevm> block in `{}`: {reason}", .path.display())]
    MalformedRedirectBlock { path: PathBuf, reason: String },
}

type Result<T> = std::result::Result<T, WorkspaceError>;

/// One member node of a workspace — a package directory carrying its own
/// `vibe.toml`, reached transitively from the absolute root.
///
/// Members are produced by [`Workspace::load`]; the `rel_path` is the
/// member's portable identity, never an absolute path:
///
/// ```
/// use vibe_core::manifest::Manifest;
/// use vibe_workspace::WorkspaceMember;
///
/// let manifest = Manifest::parse_str(
///     "[package]\ngroup = \"org.vibevm\"\nname = \"wal\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
/// ).unwrap();
/// let member = WorkspaceMember {
///     rel_path: "packages/flow-wal".to_string(),
///     manifest,
///     depth: 0,
///     parent: None,
/// };
/// assert_eq!(member.rel_path, "packages/flow-wal");
/// assert_eq!(member.manifest.require_package().unwrap().name, "wal");
/// ```
#[derive(Debug, Clone)]
#[spec(
    implements = "spec://vibevm/modules/vibe-workspace/PROP-007#workspace-section",
    r = 1
)]
pub struct WorkspaceMember {
    /// Path relative to the workspace's absolute root, forward-slashed.
    /// This is the member's portable identity — it is what the lockfile
    /// records, never an absolute path.
    pub rel_path: String,
    /// The member's parsed, validated manifest, with `[workspace.versions]`
    /// placeholders already resolved.
    pub manifest: Manifest,
    /// Nesting depth: `0` for a direct member of the absolute root, `1`
    /// for a member of a nested workspace, and so on.
    pub depth: usize,
    /// The `rel_path` of the workspace node that declared this member, or
    /// `None` if it was declared directly by the absolute root. Drives the
    /// recursive `[workspace.versions]` placeholder lookup. PROP-007 §2.6.
    pub parent: Option<String>,
}

/// A loaded workspace: an absolute root plus every member, transitively.
///
/// Construct one with [`Workspace::discover`] (from anywhere inside the tree)
/// or [`Workspace::load`] (from a known root directory). Run from a member,
/// discovery walks up to the absolute root — the node where the single
/// `vibe.lock` lives:
///
/// ```
/// use vibe_workspace::Workspace;
///
/// let tmp = tempfile::TempDir::new().unwrap();
/// std::fs::write(
///     tmp.path().join("vibe.toml"),
///     "[project]\nname = \"mono\"\nversion = \"0.0.1\"\n\n\
///      [workspace]\nmembers = [\"pkg\"]\n",
/// ).unwrap();
/// std::fs::create_dir(tmp.path().join("pkg")).unwrap();
/// std::fs::write(
///     tmp.path().join("pkg").join("vibe.toml"),
///     "[package]\ngroup = \"org.vibevm\"\nname = \"pkg\"\nkind = \"flow\"\nversion = \"0.1.0\"\n",
/// ).unwrap();
///
/// let ws = Workspace::discover(tmp.path().join("pkg")).unwrap();
/// assert!(!ws.is_standalone());
/// assert_eq!(ws.members.len(), 1);
/// assert_eq!(ws.members[0].rel_path, "pkg");
/// assert_eq!(ws.lockfile_path(), ws.root.join("vibe.lock"));
/// ```
#[derive(Debug, Clone)]
#[spec(
    implements = "spec://vibevm/modules/vibe-workspace/PROP-007#workspace-section",
    r = 1
)]
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
    #[spec(
        implements = "spec://vibevm/modules/vibe-workspace/PROP-007#nesting",
        r = 1
    )]
    pub fn discover(start: impl AsRef<Path>) -> Result<Workspace> {
        let start = start.as_ref();
        let start_abs = canonical(start)?;
        let start_node =
            nearest_manifest_dir(&start_abs).ok_or_else(|| WorkspaceError::NoManifest {
                start: start.to_path_buf(),
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
        let mut root_manifest = read_manifest(&root)?;

        let mut members: Vec<WorkspaceMember> = Vec::new();
        if let Some(section) = &root_manifest.workspace {
            let mut visited: HashSet<PathBuf> = HashSet::new();
            visited.insert(root.clone());
            expand(&root, section, None, &root, 0, &mut visited, &mut members)?;
        }
        members.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));

        // Resolve every `version.var` placeholder against the recursive
        // `[workspace.versions]` chain — after this pass the in-memory
        // manifests carry only concrete versions (PROP-007 §2.6).
        finalize_versions(&mut root_manifest, &mut members)?;

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

    /// The absolute path of the `vibedeps/` materialisation tree
    /// (PROP-009 §2.1) — always at the workspace root.
    pub fn vibedeps_root(&self) -> PathBuf {
        self.root.join(vibedeps::VIBEDEPS_DIR)
    }

    /// The absolute slot path for a resolved package within this
    /// workspace's `vibedeps/` tree:
    /// `<root>/vibedeps/<kind>-<name>/<version>`.
    pub fn vibedeps_slot(
        &self,
        kind: PackageKind,
        name: &str,
        version: &semver::Version,
    ) -> PathBuf {
        vibedeps::slot_abs_path(&self.root, kind, name, version)
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

    /// The absolute on-disk path of a node by its `rel_path` — `"."` is
    /// the root. In-memory only; do not persist the result.
    pub fn node_abs_path(&self, rel: &str) -> PathBuf {
        if rel == "." {
            self.root.clone()
        } else {
            join_rel(&self.root, rel)
        }
    }

    /// Iterate every node in the workspace — the root first (as `"."`),
    /// then every member — paired with its manifest. The order after the
    /// root is `rel_path`-sorted.
    pub fn iter_nodes(&self) -> impl Iterator<Item = (&str, &Manifest)> {
        std::iter::once((".", &self.root_manifest)).chain(
            self.members
                .iter()
                .map(|m| (m.rel_path.as_str(), &m.manifest)),
        )
    }

    /// `true` iff `dir` is the root or one of the members.
    fn contains_dir(&self, dir: &Path) -> bool {
        if dir == self.root {
            return true;
        }
        self.members.iter().any(|m| self.member_abs_path(m) == dir)
    }
}

// ---------------------------------------------------------------------------
// Recursive member expansion
// ---------------------------------------------------------------------------

fn expand(
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
fn finalize_versions(root_manifest: &mut Manifest, members: &mut [WorkspaceMember]) -> Result<()> {
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
pub(crate) fn path_to_slash(p: &Path) -> String {
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
    use specmark::verifies;
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
        format!(
            "[package]\ngroup = \"org.vibevm\"\nname = \"{name}\"\nkind = \"{kind}\"\nversion = \"0.1.0\"\n"
        )
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
        write(
            tmp.path(),
            "packages/flow-wal/vibe.toml",
            &package("wal", "flow"),
        );
        write(
            tmp.path(),
            "packages/feat-auth/vibe.toml",
            &package("auth", "feat"),
        );

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
        write(
            tmp.path(),
            "vibe.toml",
            &workspace_root("mono", &["packages/*"]),
        );
        write(
            tmp.path(),
            "packages/flow-a/vibe.toml",
            &package("a", "flow"),
        );
        write(
            tmp.path(),
            "packages/flow-b/vibe.toml",
            &package("b", "flow"),
        );
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
    #[verifies("spec://vibevm/modules/vibe-workspace/PROP-007#nesting", r = 1)]
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
    #[verifies("spec://vibevm/modules/vibe-workspace/PROP-007#nesting", r = 1)]
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
        write(
            tmp.path(),
            "vibe.toml",
            &workspace_root("outer", &["other"]),
        );
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
        assert!(
            matches!(err, WorkspaceError::MemberNotFound { .. }),
            "{err}"
        );
    }

    #[test]
    fn nesting_cycle_is_detected() {
        let tmp = TempDir::new().unwrap();
        // Root lists `sub`; `sub` lists `..` back to the root directory.
        write(tmp.path(), "vibe.toml", &workspace_root("mono", &["sub"]));
        write(
            tmp.path(),
            "sub/vibe.toml",
            &format!(
                "{}\n[workspace]\nmembers = [\"..\"]\n",
                package("sub", "flow")
            ),
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

    #[test]
    #[verifies("spec://vibevm/modules/vibe-workspace/PROP-007#versions", r = 1)]
    fn version_var_resolves_from_root_workspace() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "vibe.toml",
            "[project]\nname = \"mono\"\nversion = \"0.0.1\"\n\n\
             [workspace]\nmembers = [\"pkg\"]\n\n\
             [workspace.versions]\ncore = \"^0.2\"\n",
        );
        write(
            tmp.path(),
            "pkg/vibe.toml",
            "[package]\ngroup = \"org.vibevm\"\nname = \"pkg\"\nkind = \"flow\"\nversion = \"0.1.0\"\n\n\
             [requires.packages]\n\"org.vibevm/wal\" = { version.var = \"core\" }\n",
        );
        let ws = Workspace::load(tmp.path()).unwrap();
        let pkg = ws.member_by_rel_path("pkg").unwrap();
        // The placeholder is resolved: var_packages drained into packages.
        assert!(pkg.manifest.requires.var_packages.is_empty());
        assert_eq!(pkg.manifest.requires.packages.len(), 1);
        assert_eq!(
            pkg.manifest.requires.packages[0].to_string(),
            "org.vibevm/wal@^0.2"
        );
    }

    #[test]
    #[verifies("spec://vibevm/modules/vibe-workspace/PROP-007#versions", r = 1)]
    fn version_var_matryoshka_nearest_wins() {
        let tmp = TempDir::new().unwrap();
        // Root defines core = ^0.1; a nested workspace overrides it to ^0.9.
        write(
            tmp.path(),
            "vibe.toml",
            "[project]\nname = \"mono\"\nversion = \"0.0.1\"\n\n\
             [workspace]\nmembers = [\"sub\"]\n\n\
             [workspace.versions]\ncore = \"^0.1\"\n",
        );
        write(
            tmp.path(),
            "sub/vibe.toml",
            "[package]\ngroup = \"org.vibevm\"\nname = \"sub\"\nkind = \"stack\"\nversion = \"0.1.0\"\n\n\
             [workspace]\nmembers = [\"leaf\"]\n\n\
             [workspace.versions]\ncore = \"^0.9\"\n",
        );
        write(
            tmp.path(),
            "sub/leaf/vibe.toml",
            "[package]\ngroup = \"org.vibevm\"\nname = \"leaf\"\nkind = \"flow\"\nversion = \"0.1.0\"\n\n\
             [requires.packages]\n\"org.vibevm/wal\" = { version.var = \"core\" }\n",
        );
        let ws = Workspace::load(tmp.path()).unwrap();
        let leaf = ws.member_by_rel_path("sub/leaf").unwrap();
        // The nearest enclosing [workspace.versions] — sub's — wins.
        assert_eq!(
            leaf.manifest.requires.packages[0].to_string(),
            "org.vibevm/wal@^0.9"
        );
    }

    #[test]
    fn unknown_version_var_errors() {
        let tmp = TempDir::new().unwrap();
        write(
            tmp.path(),
            "vibe.toml",
            "[project]\nname = \"mono\"\nversion = \"0.0.1\"\n\n\
             [workspace]\nmembers = [\"pkg\"]\n",
        );
        write(
            tmp.path(),
            "pkg/vibe.toml",
            "[package]\ngroup = \"org.vibevm\"\nname = \"pkg\"\nkind = \"flow\"\nversion = \"0.1.0\"\n\n\
             [requires.packages]\n\"org.vibevm/wal\" = { version.var = \"ghost\" }\n",
        );
        let err = Workspace::load(tmp.path()).unwrap_err();
        assert!(
            matches!(err, WorkspaceError::UnknownVersionVar { .. }),
            "{err}"
        );
    }
}
