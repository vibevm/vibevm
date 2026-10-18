//! The install orchestrator — the plan → apply pipeline behind
//! `vibe install`, extracted from the CLI (VIBEVM-SPEC §5.6, §9.1,
//! §11.1; the 2026-06-12 audit's sketch, built by SHRINK-PLAN v0.2).
//!
//! The crate owns the install *transaction*: deriving the effective
//! root set, the PROP-011 freshness fast path, driving the depsolver,
//! fetching and feature-pinning every node, the PROP-003 §2.6.1
//! conditional-dependency fixpoint, and recording the outcome into
//! `vibe.toml` / `vibe.lock`. It deliberately does NOT own:
//!
//! - **Cell construction** — the R-001 registry. The caller builds its
//!   registry/solver cells and hands them in behind [`InstallSource`];
//!   construction sites live in the shared `vibe-package-source`
//!   composition (since R7.4 A15a) — no surface constructs a cell of its
//!   own.
//! - **Interaction** — confirmation prompts, TTY detection, and report
//!   rendering. [`plan`] returns a [`Plan`] the caller presents and
//!   confirms; [`apply`] runs only after the caller said yes. Progress
//!   during planning surfaces through typed [`PlanEvent`]s, not
//!   prints.
//!
//! The split mirrors the original M0 crate of the same name
//! (`plan_install` / `apply_install`), whose materialisation half
//! moved into `vibe-workspace` under the loading model — what returns
//! here is the orchestration layer the audit found tangled into the
//! CLI.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::collections::BTreeSet;
use std::path::Path;

use vibe_core::PackageRef;
use vibe_core::manifest::Manifest;
use vibe_registry::{CachedPackage, InPlaceMaterialised, RegistryError};
use vibe_resolver::{ResolvedGraph, SolveError};

mod apply;
mod authored_configuration;
pub use authored_configuration::{authored_configuration, bind_authored_request};
mod error;
mod events;
pub mod evolution;
mod fetched;
mod lifecycle;
#[cfg(test)]
mod m_05_f_1_cases;
#[cfg(test)]
mod m_05_f_2_cases;
mod plan;
mod preconditions;
mod record;
pub mod removal;
pub use record::{RequestFreshness, configuration_definition, request_freshness, requested_record};
mod slot_verify;
pub use slot_verify::{
    DerivedMaterializationProof, PreparedPackageOverlay, VerifiedSourceMembership, source_strength,
    verify_original_source_capture,
};
mod visibility_projection;

pub use apply::{
    ApplyReport, PreparedApplyReport, SlotLifecycleSeams, apply, apply_with_spec_format,
    apply_with_spec_format_and_hook_output, apply_with_spec_format_and_lifecycle_observed,
    apply_with_spec_format_and_lifecycle_observed_traced,
    apply_with_spec_format_and_lifecycle_observed_traced_prepared,
};
pub use error::Error;
pub use events::{NullObserver, PlanEvent, PlanObserver};
pub use fetched::{Fetched, NodeInstallMeta};
pub use lifecycle::{
    CapturedSlotTarget, InstallProgress, InstallSlotLifecycle, NoSlotLifecycleObserver,
    SlotLifecycleObserver, SlotLifecyclePlan, SlotLifecyclePlanEntry, SlotLifecycleReport,
    plan_captured_slot_targets,
};
pub use plan::{
    InstallRequest, Plan, PlannedInstall, plan, plan_prepared_with_spec_format,
    plan_with_spec_format,
};
pub use record::{
    exact_pinned_pkgref, finalize_pkgref_for_manifest, merge_manifest_requires,
    merge_root_dependencies, record_git_source,
};
pub use visibility_projection::{
    FilteringDepProvider, Projection, ProjectionInput, metadata_manifest,
    prepare_visible_source_authority, project, solve_with_visibility_mask,
};

/// Visibility-sealed source authority. Only the install composition function
/// can construct it; workspace's raw capture binding is not a query grant.
///
/// A member's private transitive dependency remains unreadable even if the
/// caller requests it and supplied real captured bytes. The positive request
/// for the member's direct dependency uses those same prepared snapshots.
///
/// ```
/// use std::collections::{BTreeMap, BTreeSet};
/// use std::fs;
/// use vibe_core::manifest::Manifest;
/// use vibe_core::{ContentHash, Group, PackageKind, PackageRef};
/// use vibe_install::{prepare_visible_source_authority, PreparedVisibleSourceAuthority};
/// use vibe_resolver::{ResolvedGraph, ResolvedNode};
/// use vibe_safefs::Project;
/// use vibe_workspace::{Workspace, WorkspaceMember};
/// use vibe_workspace::extension_world::ExtensionWorldEpoch;
/// use vibe_workspace::install::ResolvedDep;
/// use vibe_workspace::source_query::{PreparedOwnerCapture, SourceQueryUnavailable};
/// let temp = tempfile::tempdir().unwrap();
/// let member_text = "[package]\ngroup='org.demo'\nname='member'\nkind='flow'\nversion='1.0.0'\n[requires.packages]\n'org.demo/a'='=1.0.0'\n";
/// let a_text = "[package]\ngroup='org.demo'\nname='a'\nkind='flow'\nversion='1.0.0'\n[requires.packages]\n'org.demo/b'={version='=1.0.0',access='private'}\n";
/// let b_text = "[package]\ngroup='org.demo'\nname='b'\nkind='flow'\nversion='1.0.0'\n";
/// let member_manifest = Manifest::parse_str(member_text).unwrap();
/// let workspace = Workspace {
///     root: temp.path().to_path_buf(),
///     root_manifest: Manifest::parse_str("[project]\nname='root'\nversion='1.0.0'\n").unwrap(),
///     members: vec![WorkspaceMember { rel_path: "member".into(), manifest: member_manifest,
///         depth: 0, parent: None }],
/// };
/// let group = Group::parse("org.demo").unwrap();
/// let version = semver::Version::parse("1.0.0").unwrap();
/// let member_root = workspace.node_abs_path("member");
/// let a_root = workspace.vibedeps_slot(&group, "a", &version);
/// let b_root = workspace.vibedeps_slot(&group, "b", &version);
/// let newer = workspace.vibedeps_slot(&group, "a", &semver::Version::parse("2.0.0").unwrap());
/// let unselected = workspace.vibedeps_slot(&group, "stray", &version);
/// fs::create_dir_all(&newer).unwrap();
/// fs::create_dir_all(&unselected).unwrap();
/// for (root, text) in [(&member_root, member_text), (&a_root, a_text), (&b_root, b_text)] {
///     fs::create_dir_all(root.join("spec/common")).unwrap();
///     fs::write(root.join("vibe.toml"), text).unwrap();
///     fs::write(root.join("spec/common/DOC.md"), "# Doc {#root}\nCAPTURED_BODY\n").unwrap();
/// }
/// let hash = ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap();
/// let dep = |name: &str, text: &str, requires: Vec<(Group, String)>| ResolvedDep {
///     kind: PackageKind::Flow, group: group.clone(), name: name.into(), version: version.clone(),
///     content_dir: workspace.vibedeps_slot(&group, name, &version), source_hash: Some(hash.clone()),
///     embedded_sources: vec![], manifest: Manifest::parse_str(text).unwrap(), requires,
///     admitted_by: None, via_override: None, source_mutable: false, in_place_changed: None,
/// };
/// let epoch = ExtensionWorldEpoch::from_resolution(temp.path(), &[
///     dep("a", a_text, vec![(group.clone(), "b".into())]), dep("b", b_text, vec![]),
/// ]).unwrap();
/// let snapshot = |root: &std::path::Path, path: &str| Project::open(root).unwrap()
///     .read_file_snapshot_bounded(path, 1024 * 1024).unwrap().unwrap();
/// let outputs = |root: &std::path::Path| BTreeMap::from([
///     ("spec/common/DOC.md".into(), snapshot(root, "spec/common/DOC.md"))]);
/// let captures = vec![
///     PreparedOwnerCapture::self_node(member_root.clone(), snapshot(&member_root, "vibe.toml"), outputs(&member_root), None),
///     PreparedOwnerCapture::dependency("org.demo", "a", a_root.clone(), snapshot(&a_root, "vibe.toml"), outputs(&a_root), None),
///     PreparedOwnerCapture::dependency("org.demo", "b", b_root.clone(), snapshot(&b_root, "vibe.toml"), outputs(&b_root), None),
/// ];
/// let graph = ResolvedGraph { packages: vec![
///     ResolvedNode { group: group.clone(), name: "a".into(), version: version.clone(),
///         dependencies: vec![PackageRef::parse("org.demo/b@=1.0.0").unwrap()], is_root: true },
///     ResolvedNode { group: group.clone(), name: "b".into(), version,
///         dependencies: vec![], is_root: false },
/// ]};
/// let private = BTreeSet::from(["org.demo/b".into()]);
/// let refusal = prepare_visible_source_authority(&workspace, "member", &epoch, &graph,
///     &private, captures.clone()).unwrap_err();
/// assert!(matches!(refusal, SourceQueryUnavailable::Visibility(reason)
///     if reason.contains("not visible")));
/// let mut workspace_union = graph.clone();
/// workspace_union.packages[1].is_root = true;
/// let widened = prepare_visible_source_authority(&workspace, "member", &epoch,
///     &workspace_union, &private, captures.clone()).unwrap_err();
/// assert!(matches!(widened, SourceQueryUnavailable::Visibility(reason)
///     if reason.contains("graph roots differ")));
/// let mut wrong_edges = graph.clone();
/// wrong_edges.packages[0].dependencies.clear();
/// let edges = prepare_visible_source_authority(&workspace, "member", &epoch,
///     &wrong_edges, &private, captures.clone()).unwrap_err();
/// assert!(matches!(edges, SourceQueryUnavailable::Visibility(reason)
///     if reason.contains("graph edges differ")));
/// let mut wrong_version = graph.clone();
/// wrong_version.packages[1].version = semver::Version::parse("2.0.0").unwrap();
/// let version = prepare_visible_source_authority(&workspace, "member", &epoch,
///     &wrong_version, &private, captures.clone()).unwrap_err();
/// assert!(matches!(version, SourceQueryUnavailable::Visibility(reason)
///     if reason.contains("graph version differs")));
/// let direct = BTreeSet::from(["org.demo/a".into()]);
/// let admitted: PreparedVisibleSourceAuthority = prepare_visible_source_authority(
///     &workspace, "member", &epoch, &graph, &direct, captures).unwrap();
/// let summary = format!("{admitted:?}");
/// assert!(summary.contains("owner_count: 2") && summary.contains("visible_count: 1"));
/// assert!(!summary.contains("CAPTURED_BODY"));
/// ```
pub struct PreparedVisibleSourceAuthority {
    bound: vibe_workspace::source_query::BoundSourceInputs,
    visibility: vibe_core::visibility::Analysis,
}

impl std::fmt::Debug for PreparedVisibleSourceAuthority {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PreparedVisibleSourceAuthority")
            .field("owner_count", &self.bound.owner_count())
            .field("visible_count", &self.visibility.effective.len())
            .finish_non_exhaustive()
    }
}

/// A query projection may only use this member's exact retained resolution,
/// including transitive membership and ordered selected edges.
fn check_member_graph(
    epoch: &vibe_workspace::extension_world::ExtensionWorldEpoch,
    node_root: &std::path::Path,
    node_manifest: &vibe_core::manifest::Manifest,
    graph: &vibe_resolver::ResolvedGraph,
) -> Result<(), vibe_workspace::source_query::SourceQueryUnavailable> {
    use std::collections::BTreeSet;
    use vibe_workspace::source_query::SourceQueryUnavailable;

    let view = epoch.node_owner_view(node_root, node_manifest)?;
    let expected = view
        .installed
        .iter()
        .map(|row| row.provider.id.to_string())
        .collect::<BTreeSet<_>>();
    let actual = graph
        .iter()
        .map(|node| format!("{}/{}", node.group, node.name))
        .collect::<BTreeSet<_>>();
    if actual.len() != graph.packages.len() || expected != actual {
        return Err(SourceQueryUnavailable::Visibility(
            "resolved graph package set differs from selected member epoch".into(),
        ));
    }
    for node in graph.iter() {
        let id = format!("{}/{}", node.group, node.name);
        let installed = view
            .installed
            .iter()
            .find(|row| row.provider.id.to_string() == id)
            .ok_or_else(|| {
                SourceQueryUnavailable::Visibility(format!("graph package `{id}` is absent"))
            })?;
        if installed.provider.version != node.version.to_string() {
            return Err(SourceQueryUnavailable::Visibility(format!(
                "graph version differs from selected epoch for `{id}`"
            )));
        }
        let expected_edges = epoch
            .package_edges(&installed.provider.id)?
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let actual_edges = node
            .dependencies
            .iter()
            .map(vibe_core::PackageRef::qualified_name)
            .collect::<Vec<_>>();
        if expected_edges != actual_edges {
            return Err(SourceQueryUnavailable::Visibility(format!(
                "graph edges differ from selected epoch for `{id}`"
            )));
        }
    }
    Ok(())
}

/// The package source an install runs against — the seam between the
/// orchestrator and whatever registry topology the caller composed
/// (R-001: cells are constructed at the caller's composition root and
/// arrive here already built).
///
/// Canonical implementation shape:
///
/// ```no_run
/// use std::collections::BTreeSet;
/// use std::path::Path;
/// use vibe_core::{PackageRef, manifest::Manifest};
/// use vibe_install::InstallSource;
/// use vibe_registry::{CachedPackage, LocalRegistry, RegistryError};
/// use vibe_resolver::{ResolvedGraph, SolveError};
///
/// struct LocalSource(LocalRegistry);
///
/// impl InstallSource for LocalSource {
///     fn resolve_and_fetch(
///         &self,
///         pkgref: &PackageRef,
///         store_root: &Path,
///         _expected_hash: Option<&str>,
///     ) -> Result<CachedPackage, RegistryError> {
///         let resolved = self.0.resolve(pkgref)?;
///         self.0.fetch(&resolved, store_root)
///     }
///
///     fn solve(&self, roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
///         // Build the solver from the caller's selected cells.
///         # let _ = roots;
///         # unimplemented!()
///     }
///
///     fn manifest_of(&self, pkg: &PackageRef) -> Result<Manifest, SolveError> {
///         # let _ = pkg;
///         # unimplemented!()
///     }
///
///     fn solve_masked(
///         &self,
///         roots: &[PackageRef],
///         blocked: &BTreeSet<(String, String)>,
///     ) -> Result<ResolvedGraph, SolveError> {
///         # let _ = (roots, blocked);
///         # unimplemented!()
///     }
///
///     fn materialise_in_place(
///         &self,
///         pkgref: &PackageRef,
///         slot: &Path,
///     ) -> Result<vibe_registry::InPlaceMaterialised, RegistryError> {
///         // A local-directory registry has no git backend; a git-backed
///         // source clones / incrementally updates the slot here.
///         # let _ = (pkgref, slot);
///         unimplemented!()
///     }
/// }
/// ```
pub trait InstallSource {
    fn begin_normalized_transaction(
        &self,
        request: &vibe_resolver::resolvo_engine::request::NormalizedRequest,
    ) -> Result<Option<Box<dyn InstallSource + '_>>, SolveError> {
        self.begin_transaction(&request.roots)
    }
    fn solve_conditional(
        &self,
        roots: &[PackageRef],
        edges: &[vibe_resolver::resolvo_engine::request::ConditionalEdge],
    ) -> Result<ResolvedGraph, SolveError> {
        let mut combined = roots.to_vec();
        combined.extend(edges.iter().map(|e| e.dependency.clone()));
        self.solve(&combined)
    }
    fn normalized_request(
        &self,
    ) -> Option<&vibe_resolver::resolvo_engine::request::NormalizedRequest> {
        None
    }
    fn route_resolution(&self) -> Option<vibe_resolver::resolvo_engine::request::RouteResolution> {
        None
    }
    fn finish_route_projection(&self, _graph: &ResolvedGraph) -> Result<(), SolveError> {
        Ok(())
    }
    fn catalog_manifest_of(&self, pkg: &PackageRef) -> Result<Manifest, SolveError> {
        self.manifest_of(pkg)
    }

    /// Capture an exact catalog when this source supports transaction binding.
    /// None preserves legacy embeddings without claiming an exact catalog.
    fn begin_transaction(
        &self,
        _roots: &[PackageRef],
    ) -> Result<Option<Box<dyn InstallSource + '_>>, SolveError> {
        Ok(None)
    }
    fn catalog_inspection(&self) -> Option<&vibe_resolver::resolvo_engine::CatalogInspection> {
        None
    }
    /// Validate the selected source before any install acquisition effects.
    fn validate_catalog_candidate(&self, _pkg: &PackageRef) -> Result<(), SolveError> {
        Ok(())
    }

    /// Resolve `pkgref` and insert its content into the machine-global
    /// store (`~/.vibe/cache/`, PROP-010 §2.7) under `store_root`.
    /// `expected_hash` (typically the lockfile pin) lets a
    /// mirror-aware source skip a source serving disagreeing bytes.
    fn resolve_and_fetch(
        &self,
        pkgref: &PackageRef,
        store_root: &Path,
        expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError>;

    /// Observed sibling of [`Self::resolve_and_fetch`]. Implementations with
    /// internal registry/network stages override this; the compatibility
    /// default preserves every existing source implementation.
    fn resolve_and_fetch_with_progress(
        &self,
        pkgref: &PackageRef,
        store_root: &Path,
        expected_hash: Option<&str>,
        _progress: &vibe_core::progress::Progress,
    ) -> Result<CachedPackage, RegistryError> {
        self.resolve_and_fetch(pkgref, store_root, expected_hash)
    }

    /// Run the depsolver against this source, returning the full
    /// transitive graph the pipeline will fetch and materialise.
    fn solve(&self, roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError>;

    /// Read one solved package's manifest through the source's metadata-only
    /// provider path. This must never fetch or materialise package content.
    fn manifest_of(&self, pkg: &PackageRef) -> Result<Manifest, SolveError>;

    /// Re-run the selected solver cell while masking the named
    /// `parent -> target` edges in every parent manifest it reads.
    fn solve_masked(
        &self,
        roots: &[PackageRef],
        blocked: &BTreeSet<(String, String)>,
    ) -> Result<ResolvedGraph, SolveError>;

    /// Place an `in-place` package (PROP-022 §2.4) directly into its project
    /// `slot`: a fresh `git clone --recurse-submodules` when the slot is
    /// absent, an incremental `git fetch` + checkout when it already carries
    /// `.git`. Bypasses the cache clone + the `.git`-stripped `copy`,
    /// so a version bump on a giant repo transfers only changed objects. Used
    /// by scoped `vibe update <pkg>` and the general `vibe install` re-resolve
    /// (via [`apply`](crate::apply), which defers the in-place fetch past the
    /// plan) for a package the lockfile records as in-place. Returns the slot's
    /// manifest + provenance for the lockfile; a source with no git backend
    /// (the local-directory registry) errors.
    fn materialise_in_place(
        &self,
        pkgref: &PackageRef,
        slot: &Path,
    ) -> Result<InPlaceMaterialised, RegistryError>;
}

pub use visibility_projection::resolve_boot_comparison;
#[cfg(test)]
mod m_06_e_1_cases;
