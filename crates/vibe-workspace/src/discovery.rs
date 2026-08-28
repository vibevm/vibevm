//! Workspace discovery and loading, including the preloaded-manifest seam.
//!
//! Split out of the crate root along the discovery-vs-model seam: `lib.rs`
//! owns the `Workspace` value and its accessors, this module owns how one is
//! FOUND and BUILT.
//!
//! ## Canonicalise once, then never again
//!
//! Every internal entry point below takes an ALREADY-canonical path. The
//! public entries canonicalise exactly once and hand the result down.
//!
//! That is not tidiness. `discover_with_selected_manifest` builds its override
//! keyed on a canonical directory; if the walk then canonicalised the caller's
//! original spelling a *second* time, a symlink or junction retargeted between
//! the two calls would produce a different `start_node` — one the override no
//! longer matches. The command would silently fall back to reading disk for
//! the very node whose manifest it had already read, which is precisely the
//! double-read this seam exists to prevent.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#nesting");

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use specmark::spec;
use vibe_core::{RelPath, manifest::Manifest};

use crate::{Result, Workspace, WorkspaceError, WorkspaceMember, canonical, expand};

/// One canonical selected-node discovery epoch.
///
/// `selected_root` is the exact canonical path used to discover `workspace`
/// and to obtain its workspace-authored `selected` identity.
#[derive(Debug, Clone)]
#[spec(
    documents = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#workspace-section"
)]
pub struct SelectedWorkspace {
    /// The whole workspace discovered from the exact selected-root epoch.
    pub workspace: Workspace,
    /// The canonical, UNC-stripped node root supplied to discovery and
    /// identity mapping; never reconstructed from `selected`.
    pub selected_root: PathBuf,
    /// The workspace-authored portable identity of `selected_root`.
    pub selected: RelPath,
}

/// One node's manifest, supplied by the caller instead of read from disk.
///
/// `dir` is the CANONICAL, UNC-stripped spelling, because that is the only
/// spelling the loader ever compares against: the same directory reached
/// through `./`, a trailing separator or a relative path must select the same
/// node, and a lexical match would let one of those quietly read disk instead.
pub(crate) struct ManifestOverride<'a> {
    pub(crate) dir: PathBuf,
    pub(crate) manifest: &'a Manifest,
}

/// The override's manifest for `dir`, when it is that exact node.
pub(crate) fn overridden<'a>(
    over: Option<&'a ManifestOverride<'_>>,
    dir: &Path,
) -> Option<&'a Manifest> {
    over.filter(|over| over.dir == dir)
        .map(|over| over.manifest)
}

/// Read a node's manifest, honouring an exact-path override.
///
/// The clone is deliberate: the loader owns every manifest in the tree and
/// then finalises versions across all of them, so it cannot borrow the
/// caller's copy — and the caller's copy must stay RAW, because that is what
/// gets written back to disk if the command rewrites the file.
pub(crate) fn read_manifest_with_override(
    dir: &Path,
    over: Option<&ManifestOverride<'_>>,
) -> Result<Manifest> {
    if let Some(manifest) = overridden(over, dir) {
        return Ok(manifest.clone());
    }
    read_manifest(dir)
}

fn read_manifest(dir: &Path) -> Result<Manifest> {
    let path = dir.join(Manifest::FILENAME);
    Manifest::read(&path).map_err(|source| WorkspaceError::Manifest {
        path,
        source: Box::new(source),
    })
}

impl Workspace {
    /// Discover the workspace enclosing `start` and load the whole tree.
    ///
    /// Walks up from `start` to the topmost `[workspace]` that transitively
    /// includes the starting node (PROP-007 §2.3). A node with no enclosing
    /// `[workspace]` is its own root — a standalone workspace.
    #[spec(
        implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#nesting",
        r = 1
    )]
    pub fn discover(start: impl AsRef<Path>) -> Result<Workspace> {
        let start = start.as_ref();
        discover_canonical(&canonical(start)?, start, None)
    }

    /// Discover `start` and its authored node identity in one canonical epoch.
    ///
    /// The exact canonical input is both the discovery input and the mapping
    /// key. A directory merely inside a workspace node is rejected rather than
    /// guessed to be that node.
    ///
    /// ```
    /// use vibe_core::RelPath;
    /// use vibe_workspace::Workspace;
    ///
    /// let tmp = tempfile::TempDir::new().unwrap();
    /// std::fs::write(
    ///     tmp.path().join("vibe.toml"),
    ///     "[project]\nname = \"solo\"\nversion = \"0.0.1\"\n",
    /// ).unwrap();
    /// let selected = Workspace::discover_selected(tmp.path()).unwrap();
    /// assert_eq!(selected.selected, RelPath::root());
    /// assert_eq!(selected.selected_root, selected.workspace.root);
    /// ```
    #[spec(
        implements = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#nesting",
        r = 1
    )]
    pub fn discover_selected(start: impl AsRef<Path>) -> Result<SelectedWorkspace> {
        let requested = start.as_ref();
        let selected_root = canonical(requested)?;
        let workspace = discover_canonical(&selected_root, requested, None)?;
        let selected = workspace.node_rel_of(&selected_root).ok_or_else(|| {
            WorkspaceError::SelectedPathNotNode {
                selected: selected_root.clone(),
                workspace_root: workspace.root.clone(),
            }
        })?;
        Ok(SelectedWorkspace {
            workspace,
            selected_root,
            selected,
        })
    }

    /// Discover from an exact selected node whose manifest the caller ALREADY
    /// read.
    ///
    /// The command layer reads its selected `vibe.toml` once — before it knows
    /// whether it will even build a workspace — because the answer decides the
    /// run identity, and because the error that read produces is the one the
    /// command is characterised to report, at the point it is characterised to
    /// report it. Discovering afterwards would read the same file a second
    /// time: a second byte version inside one command, one edit (or one
    /// concurrent write) later than the first.
    ///
    /// So the snapshot is handed IN. For the exact selected directory the
    /// loader treats the manifest as present and clones the supplied value
    /// instead of touching disk — including when the file has since been
    /// corrupted or removed. Every other node is read normally, and the whole
    /// tree is version-finalised together, so the clone inside the returned
    /// `Workspace` is finalised exactly like any other node while the caller's
    /// own copy stays raw.
    ///
    /// The path is canonicalised ONCE, into the override, and the walk starts
    /// from that same value — see the module note on why a second
    /// canonicalisation could detach the start node from the override.
    pub fn discover_with_selected_manifest(
        selected_node: impl AsRef<Path>,
        selected_manifest: &Manifest,
    ) -> Result<Workspace> {
        let requested = selected_node.as_ref();
        let over = ManifestOverride {
            dir: canonical(requested)?,
            manifest: selected_manifest,
        };
        discover_canonical(&over.dir, requested, Some(&over))
    }

    /// Load a workspace from a known root directory. The root's `vibe.toml`
    /// is read; if it carries `[workspace]`, members are expanded
    /// recursively. A root without `[workspace]` yields a standalone
    /// workspace with no members.
    pub fn load(root_dir: impl AsRef<Path>) -> Result<Workspace> {
        load_canonical(&canonical(root_dir.as_ref())?, None)
    }
}

/// The one discovery walk. `start_abs` is already canonical; `requested` is
/// kept only to name the caller's own spelling in a `NoManifest` error.
fn discover_canonical(
    start_abs: &Path,
    requested: &Path,
    over: Option<&ManifestOverride<'_>>,
) -> Result<Workspace> {
    let start_node =
        nearest_manifest_dir(start_abs, over).ok_or_else(|| WorkspaceError::NoManifest {
            start: requested.to_path_buf(),
        })?;

    // Collect every ancestor (including the start node) whose vibe.toml
    // carries a `[workspace]` table. A malformed / unreadable ancestor
    // manifest is skipped, not fatal — discovery must not break because some
    // unrelated directory higher up has a broken vibe.toml. The SELECTED
    // directory is answered from the override, so a caller whose snapshot is
    // sound still finds its own `[workspace]` table even when the file on disk
    // has since gone bad.
    let mut ws_ancestors: Vec<PathBuf> = Vec::new();
    let mut cursor: Option<&Path> = Some(start_node.as_path());
    while let Some(dir) = cursor {
        let declares_workspace = match overridden(over, dir) {
            Some(manifest) => manifest.workspace.is_some(),
            None => {
                let mf = dir.join(Manifest::FILENAME);
                mf.is_file() && Manifest::read(&mf).is_ok_and(|m| m.workspace.is_some())
            }
        };
        if declares_workspace {
            ws_ancestors.push(dir.to_path_buf());
        }
        cursor = dir.parent();
    }

    // Topmost first. The first enclosing workspace whose tree contains the
    // start node is the absolute root. Every candidate is already canonical
    // (it came from the walk), so it loads through the canonical entry.
    ws_ancestors.reverse();
    for candidate in &ws_ancestors {
        let ws = load_canonical(candidate, over)?;
        if ws.contains_dir(&start_node) {
            return Ok(ws);
        }
    }

    // No enclosing workspace — the start node stands alone.
    load_canonical(&start_node, over)
}

/// Load from an ALREADY-canonical root.
fn load_canonical(root: &Path, over: Option<&ManifestOverride<'_>>) -> Result<Workspace> {
    let root = root.to_path_buf();
    let mut root_manifest = read_manifest_with_override(&root, over)?;

    let mut members: Vec<WorkspaceMember> = Vec::new();
    if let Some(section) = &root_manifest.workspace {
        let mut visited: HashSet<PathBuf> = HashSet::new();
        visited.insert(root.clone());
        expand::expand(
            &root,
            section,
            None,
            expand::ExpandContext { root: &root, over },
            0,
            &mut visited,
            &mut members,
        )?;
    }
    members.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));

    // Resolve every `version.var` placeholder against the recursive
    // `[workspace.versions]` chain — after this pass the in-memory manifests
    // carry only concrete versions (PROP-007 §2.6).
    expand::finalize_versions(&mut root_manifest, &mut members)?;

    Ok(Workspace {
        root,
        root_manifest,
        members,
    })
}

/// The nearest ancestor (or `start` itself) that carries a manifest.
fn nearest_manifest_dir(start: &Path, over: Option<&ManifestOverride<'_>>) -> Option<PathBuf> {
    let mut cursor: Option<&Path> = if start.is_dir() {
        Some(start)
    } else {
        start.parent()
    };
    while let Some(dir) = cursor {
        // An overridden node COUNTS as having a manifest even when the file is
        // gone: the caller read it, and walking past it would silently select
        // an ancestor as the node being installed.
        if overridden(over, dir).is_some() || dir.join(Manifest::FILENAME).is_file() {
            return Some(dir.to_path_buf());
        }
        cursor = dir.parent();
    }
    None
}
