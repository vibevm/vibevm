//! The command's ONE selected-world provenance bundle: which root, which
//! manifest snapshot, and the tree built from THAT snapshot — as a single
//! unforgeable value.
//!
//! ## Why this is one value and not three fields
//!
//! Every execution entry point used to take a `SelectedManifest` A, a
//! `PreparedWorkspace` B and a canonical root C as three independent
//! parameters. Each was individually honest, and the triple was not: a caller
//! could pass A from one snapshot of a root and B from a second, later snapshot
//! of the same root, and no gate downstream could tell. The lease's root and
//! selected-node gates cannot help — both snapshots agree about the root; they
//! disagree about the CONTENT, which is exactly the thing an install then
//! mutates.
//!
//! So the pair is built once, together, inside this cell:
//!
//! * [`SelectedManifest::read`] is still the only `Manifest::read` on the path;
//! * [`SelectedManifest::prepare`] binds that snapshot to ONE canonical root and
//!   builds the tree FROM it, once, recording exactly what happened;
//! * the fields of [`PreparedSelection`] are private, so nothing outside this
//!   crate can construct one from parts, swap the tree for a fresher one, or
//!   pass a root the tree was not built against;
//! * [`PreparedSelection::prove`] is the single way out, and it hands back
//!   another closed value rather than a tuple a caller could recombine.
//!
//! What remains public is the set of read projections the surfaces genuinely
//! need BEFORE the handoff: the compile-trace request, the `[llm]` sidecar
//! borrow, and the loaded root/tree the trace home and the prelude are derived
//! from.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use specmark::spec;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use vibe_core::manifest::Manifest;
use vibe_workspace::Workspace;

/// The command's ONE read of its selected node's `vibe.toml`.
///
/// Every command in this family already read that file exactly once, at a
/// point whose failure is characterised: `vibe install` refuses a malformed
/// manifest *after* it has selected a run identity, and tests assert both the
/// message and that ordering. Compile-trace activation now wants the same file
/// EARLIER — before identity selection, because the identity carries the
/// effective trace bit.
///
/// The wrong fix is a second read for the activation question. Two reads are
/// two answers: the second races an edit between them, and — worse — it races
/// this command's own `--git` rewrite of the very file. It also has to decide
/// what an unreadable manifest means, and the only quiet answer available to
/// it is "requests nothing", which swallows a parse error the command was
/// about to report.
///
/// So there is one snapshot, taken once, carrying the `Result`. It answers the
/// activation question here and is then BOUND to a root by
/// [`Self::prepare`] — after which the manifest and the tree can no longer be
/// separated.
#[spec(documents = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail")]
pub struct SelectedManifest {
    root: PathBuf,
    result: Result<Manifest, vibe_core::Error>,
}

impl SelectedManifest {
    /// Take the snapshot. The only `Manifest::read` on this path.
    ///
    /// ```
    /// use vibe_orchestrator::SelectedManifest;
    /// let missing = SelectedManifest::read(std::path::Path::new("/definitely/absent"));
    /// assert!(missing.parsed_ref().is_none());
    /// ```
    #[must_use]
    pub fn read(project_root: &Path) -> Self {
        Self {
            root: project_root.to_path_buf(),
            result: Manifest::read(project_root.join(Manifest::FILENAME)),
        }
    }

    /// The effective compile-trace request, decided purely from the snapshot:
    /// the surface flag OR the selected project's own `[compile] trace`.
    ///
    /// See [`Self::read`]; an unreadable manifest carries no STANDING request,
    /// so the flag alone decides and the stored error is only deferred.
    #[must_use]
    #[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
    pub fn request(&self, flag: bool) -> bool {
        request_from(&self.result, flag)
    }

    /// The snapshot, borrowed — the `[llm]` sidecar a surface reads to build
    /// its own agent backend before the handoff.
    ///
    /// See [`Self::read`].
    #[must_use]
    pub fn parsed_ref(&self) -> Option<&Manifest> {
        self.result.as_ref().ok()
    }

    /// Bind this snapshot to ONE canonical root and build the tree FROM it.
    ///
    /// The single point at which a manifest and a workspace become one value.
    /// After this call there is no API — inside this crate or out of it — that
    /// can pair this manifest with a different tree, or this tree with a
    /// different root.
    ///
    /// ```no_run
    /// use vibe_orchestrator::SelectedManifest;
    /// # fn call(root: std::path::PathBuf) {
    /// let selection = SelectedManifest::read(&root).prepare();
    /// let _ = selection.loaded_root();
    /// # }
    /// ```
    ///
    /// A snapshot cannot be rebound to a second root:
    ///
    /// ```compile_fail
    /// use vibe_orchestrator::SelectedManifest;
    /// let first = std::path::Path::new("/first");
    /// let second = std::path::PathBuf::from("/second");
    /// let _ = SelectedManifest::read(first).prepare(second);
    /// ```
    #[must_use]
    pub fn prepare(self) -> PreparedSelection {
        let Self { root, result } = self;
        let workspace = match result.as_ref().ok() {
            None => WorkspaceLoad::SelectedManifestInvalid,
            Some(manifest) => match Workspace::discover_with_selected_manifest(&root, manifest) {
                Ok(workspace) => WorkspaceLoad::Loaded(Box::new(workspace)),
                Err(error) => WorkspaceLoad::DiscoveryFailed(Box::new(error)),
            },
        };
        PreparedSelection {
            root,
            manifest: result,
            workspace,
        }
    }
}

/// The effective trace request of a stored snapshot — shared by the raw
/// snapshot and the bound bundle so the two can never drift.
fn request_from(result: &Result<Manifest, vibe_core::Error>, flag: bool) -> bool {
    match result {
        Ok(manifest) => flag || manifest.compile_trace_enabled(),
        // Not "false": the flag still speaks, and the stored error is still
        // owed to the consuming boundary.
        Err(_) => flag,
    }
}

/// What the ONE workspace load produced.
///
/// PRIVATE to this cell. The distinction between the arms is the whole point,
/// and `Option<Workspace>` could not express it — a `None` meant "no prepared
/// world", which an execution seam then read as "so discover one", and a second
/// attempt can SUCCEED where the first failed. But the distinction is also
/// nobody else's business: outside this cell the only questions are "did it
/// load" (the read projections) and "prove it" ([`PreparedSelection::prove`]).
/// The public enum this replaces carried a fourth `DiscoverHere` arm that
/// nothing constructed and three sites matched on defensively.
enum WorkspaceLoad {
    /// The snapshot itself did not parse, so no load was even attempted. The
    /// stored manifest error is the failure, and it is the one the command has
    /// always reported.
    SelectedManifestInvalid,
    /// The tree this command works on.
    Loaded(Box<Workspace>),
    /// The manifest parsed and the tree did not load. THIS error is returned,
    /// never a fresher one from a retry.
    DiscoveryFailed(Box<vibe_workspace::WorkspaceError>),
}

/// One canonical root, the ONE manifest snapshot taken at it, and the ONE tree
/// built from that exact snapshot — inseparable.
///
/// See the module note for why the three are one value. The fields are private
/// and there is no public constructor other than [`SelectedManifest::prepare`],
/// so a surface cannot assemble a bundle whose parts came from different
/// moments.
#[spec(documents = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail")]
pub struct PreparedSelection {
    root: PathBuf,
    manifest: Result<Manifest, vibe_core::Error>,
    workspace: WorkspaceLoad,
}

impl PreparedSelection {
    /// The ONE canonical selected root this bundle was prepared over.
    ///
    /// ```no_run
    /// # fn call(selection: &vibe_orchestrator::PreparedSelection) {
    /// let _: &std::path::Path = selection.root();
    /// # }
    /// ```
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The effective compile-trace request — see [`SelectedManifest::request`].
    #[must_use]
    #[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-054#OBS-TRACE")]
    pub fn request(&self, flag: bool) -> bool {
        request_from(&self.manifest, flag)
    }

    /// The manifest snapshot, borrowed — the `[llm]` sidecar a surface reads to
    /// build its own agent backend.
    #[must_use]
    pub fn parsed_ref(&self) -> Option<&Manifest> {
        self.manifest.as_ref().ok()
    }

    /// The canonical root a trace may be stored under — a LOADED tree alone.
    ///
    /// Every other state has no workspace root to name, and substituting the
    /// selected project root would lock a trace home that is not the one this
    /// run's compiles belong to.
    ///
    /// ```no_run
    /// # fn call(selection: &vibe_orchestrator::PreparedSelection) {
    /// let _: Option<&std::path::Path> = selection.loaded_root();
    /// # }
    /// ```
    #[must_use]
    pub fn loaded_root(&self) -> Option<&Path> {
        match &self.workspace {
            WorkspaceLoad::Loaded(workspace) => Some(workspace.root.as_path()),
            _ => None,
        }
    }

    /// The prepared tree itself, by borrow — a LOADED tree alone.
    ///
    /// The prelude's selected-node identity is derived from THIS bundle (the
    /// one tree the command loaded), never from a second `Workspace::discover`.
    ///
    /// See [`Self::loaded_root`].
    #[must_use]
    pub fn loaded_workspace(&self) -> Option<&Workspace> {
        match &self.workspace {
            WorkspaceLoad::Loaded(workspace) => Some(workspace),
            _ => None,
        }
    }

    /// Rebuild the tree from the SAME stored snapshot at its carried root — the
    /// post-clean reload, and the only one.
    ///
    /// The manifest result is carried across UNCHANGED: a wipe cannot repair a
    /// parse error, and re-reading here would replace this command's own error
    /// with a later, vaguer one. When the snapshot did parse, the reload is
    /// STRICT — the wipe just rewrote the tree, so a workspace that will not
    /// load right afterwards is a real fault and continuing would install into
    /// a world nobody can describe.
    pub fn reload_after_clean(self) -> Result<Self> {
        let Self {
            root,
            manifest,
            workspace: _,
        } = self;
        let workspace = match manifest.as_ref().ok() {
            Some(manifest) => WorkspaceLoad::Loaded(Box::new(
                Workspace::discover_with_selected_manifest(&root, manifest)
                    .context("re-reading the workspace after the clean epoch")?,
            )),
            // An invalid snapshot is unchanged by a wipe, and its stored error
            // is still owed to the command funnel — replacing it with a generic
            // rediscovery error would report the wrong thing.
            None => WorkspaceLoad::SelectedManifestInvalid,
        };
        Ok(Self {
            root,
            manifest,
            workspace,
        })
    }

    /// Rewrap an already-proven bundle for an inner boundary that must keep its
    /// own historical proving point — the phase loop's handoff into the
    /// prerequisite install, whose manifest error has always been raised inside
    /// `execute_prepared` rather than one frame up.
    ///
    /// Crate-internal, like [`ProvenSelection::from_parts`]: no surface can
    /// perform this rebinding.
    pub(crate) fn proven(proven: ProvenSelection) -> Self {
        let (root, manifest, workspace) = proven.into_parts();
        Self {
            root,
            manifest: Ok(manifest),
            workspace: WorkspaceLoad::Loaded(Box::new(workspace)),
        }
    }

    /// PROVE the bundle the way a WORKSPACE LOAD would have failed.
    ///
    /// The pre-clean epoch used to call `Workspace::discover(root)` directly.
    /// That reads the selected manifest itself and, on a malformed one, reports
    /// `WorkspaceError::Manifest { path, source }` — the FILE PATH and, under
    /// it, the TOML line and column an operator needs. [`Self::prove`] returns
    /// the stored `vibe_core::Error` bare, because that is the install core's
    /// own historical shape and its tests assert on it.
    ///
    /// So the pre-clean caller proves through here instead: the stored error is
    /// mapped into the workspace variant FIRST, and the caller's collection
    /// context is added on top, which makes the rendered chain byte-identical
    /// to the discovery this replaced.
    pub fn prove_as_workspace_load(self) -> Result<ProvenSelection> {
        let path = self.root.join(Manifest::FILENAME);
        let manifest = self.manifest.map_err(|source| {
            anyhow::Error::new(vibe_workspace::WorkspaceError::Manifest {
                path,
                source: Box::new(source),
            })
        });
        Self {
            root: self.root,
            manifest: Ok(manifest?),
            workspace: self.workspace,
        }
        .prove()
    }

    /// PROVE the bundle, in the historical order, and hand back a closed value.
    ///
    /// The stored manifest result FIRST: a malformed selected manifest is this
    /// command's error, in its own words, at the point it has always been
    /// raised. Then the ONE workspace answer, returned exactly as it was —
    /// retrying here could succeed against a tree the identity and the trace
    /// were never prepared for.
    ///
    /// This is the only way out of the bundle, and what it returns is another
    /// closed value: a caller cannot take the triple apart and hand a mismatched
    /// pair to an execution entry point.
    pub fn prove(self) -> Result<ProvenSelection> {
        let manifest = self.manifest?;
        let workspace = match self.workspace {
            WorkspaceLoad::Loaded(workspace) => *workspace,
            // The FIRST answer, returned as it was.
            WorkspaceLoad::DiscoveryFailed(error) => {
                return Err(anyhow::Error::new(*error)
                    .context("discovering the workspace enclosing the project"));
            }
            // Unreachable in practice: the line above returns the stored
            // manifest error first. Named rather than merged so that a future
            // state which reports "invalid" while holding a parsed manifest is
            // a compile-time question instead of a silent success.
            WorkspaceLoad::SelectedManifestInvalid => {
                anyhow::bail!(
                    "internal: the selected manifest was reported invalid but its error \
                     was already consumed"
                );
            }
        };
        Ok(ProvenSelection {
            root: self.root,
            manifest,
            workspace,
        })
    }
}

/// A bundle that has been proven: the manifest parsed, the tree loaded, and
/// both belong to the one carried root.
///
/// Still closed. The surfaces read it by borrow; only this crate takes it apart,
/// through the single [`Self::into_parts`] consume, at the one boundary that
/// owns the mutation.
#[spec(documents = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail")]
pub struct ProvenSelection {
    root: PathBuf,
    manifest: Manifest,
    workspace: Workspace,
}

impl ProvenSelection {
    /// Rebind an already-proven triple this crate itself took apart — the phase
    /// loop's handoff of its own consumed pair into the prerequisite install.
    ///
    /// Crate-internal: the whole point of the bundle is that no surface can
    /// perform this rebinding.
    pub(crate) fn from_parts(root: PathBuf, manifest: Manifest, workspace: Workspace) -> Self {
        Self {
            root,
            manifest,
            workspace,
        }
    }

    /// The ONE canonical selected root.
    ///
    /// ```no_run
    /// # fn call(proven: &vibe_orchestrator::ProvenSelection) {
    /// let _: &std::path::Path = proven.root();
    /// # }
    /// ```
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The proven manifest snapshot, borrowed.
    ///
    /// See [`Self::root`].
    #[must_use]
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// The proven tree, borrowed.
    ///
    /// See [`Self::root`].
    #[must_use]
    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    /// The ONE crate-internal consume, at the boundary that owns the mutation.
    pub(crate) fn into_parts(self) -> (PathBuf, Manifest, Workspace) {
        (self.root, self.manifest, self.workspace)
    }
}

#[cfg(test)]
#[path = "selection/tests.rs"]
mod tests;
