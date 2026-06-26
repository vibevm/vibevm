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

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use specmark::spec;
use thiserror::Error;
use vibe_core::manifest::Manifest;
use vibe_core::{PackageKind, RelPath};

pub mod boot;
pub mod boot_artifacts;
pub mod freshness;
pub mod hooks;
pub mod install;
pub mod materialization;
pub mod publish;
pub mod vibedeps;

mod expand;

pub use publish::{
    OriginInfo, PublishNode, Selection, SkippedNode, StagedNode, select_publishable_nodes,
    stage_node, topo_order,
};

/// Errors raised while discovering or loading a workspace.
///
/// Messages carry the offending path or pattern, so the operator knows
/// which manifest to repair, and every display string ends with the
/// Class-F machine tail — `(violates spec://…; fix: …)` — so a failing
/// run is navigable back to the requirement without source access:
///
/// ```
/// use vibe_workspace::WorkspaceError;
///
/// let err = WorkspaceError::NestingCycle {
///     path: "packages/a".to_string(),
/// };
/// assert_eq!(
///     err.to_string(),
///     "workspace nesting cycle: `packages/a` is reached more than once \
///      (violates spec://vibevm/modules/vibe-workspace/PROP-007#nesting; \
///      fix: remove the members entry that re-lists an ancestor workspace)",
/// );
/// ```
#[derive(Debug, Error)]
#[spec(implements = "spec://vibevm/modules/vibe-workspace/PROP-007#nesting")]
pub enum WorkspaceError {
    /// No `vibe.toml` exists at or above the starting directory.
    #[error(
        "no `vibe.toml` found at or above `{}` \
         (violates spec://vibevm/modules/vibe-workspace/PROP-007#nesting; \
         fix: run inside a vibevm project or create a `vibe.toml` at the \
         project root)",
        .start.display()
    )]
    NoManifest { start: PathBuf },

    /// A node's `vibe.toml` failed to read or validate. The inner error is
    /// boxed — `vibe_core::Error` is large, and an unboxed copy would bloat
    /// every `Result` in this crate (`clippy::result_large_err`).
    #[error(
        "manifest at `{}` is invalid \
         (violates spec://vibevm/modules/vibe-workspace/PROP-007#unified-manifest; \
         fix: repair that vibe.toml — the underlying error names the defect)",
        .path.display()
    )]
    Manifest {
        path: PathBuf,
        #[source]
        source: Box<vibe_core::Error>,
    },

    /// A `[workspace].members` entry — an explicit (non-glob) path —
    /// names a directory that does not exist or carries no `vibe.toml`.
    #[error(
        "workspace member `{pattern}` declared in `{declared_in}` does not exist \
         or carries no vibe.toml \
         (violates spec://vibevm/modules/vibe-workspace/PROP-007#workspace-section; \
         fix: create the member directory with a vibe.toml or drop the entry \
         from [workspace].members)"
    )]
    MemberNotFound {
        pattern: String,
        declared_in: String,
    },

    /// A member resolved to a directory outside the absolute root. Every
    /// member must live under the root so its `rel_path` is portable.
    #[error(
        "workspace member `{path}` lies outside the workspace root `{root}` \
         (violates spec://vibevm/modules/vibe-workspace/PROP-007#nesting; \
         fix: move the member under the workspace root or drop it from \
         [workspace].members)"
    )]
    MemberOutsideRoot { path: String, root: String },

    /// A `[workspace]` transitively lists itself — the member graph is not
    /// a tree.
    #[error(
        "workspace nesting cycle: `{path}` is reached more than once \
         (violates spec://vibevm/modules/vibe-workspace/PROP-007#nesting; \
         fix: remove the members entry that re-lists an ancestor workspace)"
    )]
    NestingCycle { path: String },

    /// A `members` glob pattern is syntactically invalid.
    #[error(
        "invalid member glob pattern `{pattern}`: {reason} \
         (violates spec://vibevm/modules/vibe-workspace/PROP-007#workspace-section; \
         fix: correct the glob in [workspace].members)"
    )]
    BadGlob { pattern: String, reason: String },

    /// A filesystem operation failed.
    #[error(
        "I/O error on `{}`: {reason} \
         (violates spec://vibevm/modules/vibe-workspace/PROP-007#nesting; \
         fix: check that the path exists and is readable, then retry)",
        .path.display()
    )]
    Io { path: PathBuf, reason: String },

    /// A `version.var` placeholder names no entry in any enclosing
    /// `[workspace.versions]` table.
    #[error(
        "version placeholder `{var}` referenced in `{declared_in}` is defined in no \
         enclosing [workspace.versions] \
         (violates spec://vibevm/modules/vibe-workspace/PROP-007#versions; \
         fix: define `{var}` in a [workspace.versions] table of an enclosing \
         workspace)"
    )]
    UnknownVersionVar { var: String, declared_in: String },

    /// A `[workspace.versions]` entry holds an unparseable version constraint.
    #[error(
        "[workspace.versions] placeholder `{var}` has an invalid constraint \
         `{constraint}` \
         (violates spec://vibevm/modules/vibe-workspace/PROP-007#versions; \
         fix: give the placeholder a parseable constraint such as `0.0.1` or `^0.3`)"
    )]
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
    #[error(
        "boot dependency cycle among: {packages} \
         (violates spec://vibevm/modules/vibe-workspace/PROP-009#effective-boot; \
         fix: break the [requires] cycle among the listed packages)"
    )]
    BootDependencyCycle { packages: String },

    /// A `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` carries a malformed
    /// vibevm managed block — not exactly one well-formed `<vibevm>` …
    /// `</vibevm>` pair (PROP-012 §2.3). vibevm never guesses which block
    /// is canonical; the operator repairs the file by hand.
    #[error(
        "malformed <vibevm> block in `{}`: {reason} \
         (violates spec://vibevm/modules/vibe-workspace/PROP-012#markers; \
         fix: repair the file by hand to exactly one <vibevm>/</vibevm> pair)",
        .path.display()
    )]
    MalformedRedirectBlock { path: PathBuf, reason: String },

    /// A package's install hook (PROP-020) failed: no usable interpreter, a
    /// spawn error, an untrusted run, or a `pre-install` non-zero exit. The
    /// wrapped hook error already carries its own Class-F `(violates …;
    /// fix: …)` tail, so this delegates its display transparently. For a
    /// `pre-install` failure the materialised slot is rolled back before
    /// this surfaces (PROP-020 §2.5).
    #[error(transparent)]
    Hook(#[from] crate::hooks::HookError),
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
///     rel_path: "packages/flow-wal".into(),
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
    pub rel_path: RelPath,
    /// The member's parsed, validated manifest, with `[workspace.versions]`
    /// placeholders already resolved.
    pub manifest: Manifest,
    /// Nesting depth: `0` for a direct member of the absolute root, `1`
    /// for a member of a nested workspace, and so on.
    pub depth: usize,
    /// The `rel_path` of the workspace node that declared this member, or
    /// `None` if it was declared directly by the absolute root. Drives the
    /// recursive `[workspace.versions]` placeholder lookup. PROP-007 §2.6.
    pub parent: Option<RelPath>,
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
            expand::expand(&root, section, None, &root, 0, &mut visited, &mut members)?;
        }
        members.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));

        // Resolve every `version.var` placeholder against the recursive
        // `[workspace.versions]` chain — after this pass the in-memory
        // manifests carry only concrete versions (PROP-007 §2.6).
        expand::finalize_versions(&mut root_manifest, &mut members)?;

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
        join_rel(&self.root, member.rel_path.as_str())
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
#[path = "lib/tests.rs"]
mod tests;
