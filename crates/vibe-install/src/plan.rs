//! The planning phase — derive roots and one source-bound metadata closure,
//! including visibility, configuration and conditional activation. Preview
//! returns that selection; installation acquires its exact retained candidates
//! before [`apply`](crate::apply) publishes project state.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use vibe_core::PackageRef;
use vibe_core::manifest::{Lockfile, Manifest, SpecFormat};
use vibe_resolver::FeatureRequest;
use vibe_workspace::Workspace;
use vibe_workspace::install::ResolvedDep;

use crate::InstallSource;
use crate::error::{Error, Result};
use crate::events::{PlanEvent, PlanObserver};
use crate::fetched::{Fetched, build_language_chain, load_or_empty_lockfile};

/// What the caller asked to install. `roots` is empty for the
/// install-from-manifest shape; when non-empty, every pkgref is
/// already parsed and group-qualified (short-name resolution is the
/// caller's input boundary, PROP-008 §2.6).
#[derive(Debug, Clone)]
pub struct InstallRequest {
    /// Explicitly requested roots; empty = install everything the
    /// workspace's `[requires]` tables declare.
    pub roots: Vec<PackageRef>,
    /// The root-package feature request (PROP-003 §2.4).
    pub features: FeatureRequest,
    /// Language override for the PROP-003 §2.7 chain; `None` defers
    /// to the project's `[i18n]`.
    pub language: Option<String>,
    /// `--exact`: pin `=<resolved>` into the manifest instead of the
    /// caret default.
    pub exact: bool,
    /// Lockfile provenance stamp for a freshly created `vibe.lock`,
    /// e.g. `vibe 1.0.0`.
    pub generated_by: String,
    /// Whole-invocation network posture. Embedded-source hydration obeys the
    /// same hard offline boundary as package resolution.
    pub offline: bool,
    /// `--no-recommends`: the solve is the hard solve and the greedy
    /// `[recommends]` expansion is not run at all
    /// (`PROP-077#NO-RECOMMENDS-FLAG`). It changes a solve, never a replay:
    /// a lock already records an accepted recommendation as a non-root graph
    /// member, so `vibe install` with no arguments materialises exactly what
    /// the lock says whichever way the flag is passed.
    pub no_recommends: bool,
}

/// The planning verdict.
#[derive(Debug)]
pub enum Plan {
    /// PROP-011 §2.2 — `vibe.lock` is already a correct resolution of
    /// every node's `[requires]`; nothing to resolve. The caller
    /// regenerates boot artifacts (cheap, self-healing) and reports.
    Fresh,
    /// A real resolution was computed and awaits the caller's
    /// confirmation before [`apply`](crate::apply).
    Ready(Box<PlannedInstall>),
}

/// A confirmed-pending resolution — everything [`apply`](crate::apply)
/// needs, carried by value so the apply phase cannot observe state
/// the plan did not produce.
#[derive(Debug)]
pub struct PlannedInstall {
    pub(crate) project_root: PathBuf,
    pub(crate) request: InstallRequest,
    pub(crate) manifest: Manifest,
    pub(crate) lockfile: Lockfile,
    pub(crate) language_chain: Vec<String>,
    pub(crate) fetched: Vec<Fetched>,
    /// The packages to materialise, in resolution order — the shape
    /// the caller presents for confirmation.
    pub resolution: Vec<ResolvedDep>,
    /// The workspace this plan was computed over, carried by value.
    ///
    /// Apply and the slot lifecycle need the same tree the plan reasoned
    /// about, including a case-c migration delta that exists only in memory
    /// until apply writes it. Rediscovering there would be a second byte
    /// snapshot of a tree the command is midway through changing.
    pub(crate) workspace: Workspace,
    pub(crate) progress: vibe_core::progress::Progress,
    pub(crate) source_candidates: std::collections::BTreeMap<
        (vibe_core::Group, String),
        Vec<vibe_registry::source_candidates::SourceCandidateObservation>,
    >,
    /// What became of this closure's `[recommends]` and `[suggests]` tables
    /// (PROP-077 `#PROJECT-RECOMMENDS`). Read by the caller that renders the
    /// install report; the apply phase does nothing with it.
    pub weak: weak::WeakDependencyReport,
}

/// Plan an install transaction over `source` for the project at
/// `project_root` (which must already contain a `vibe.toml`).
///
/// Read-mostly: case-c migration seeds the in-memory consumer before solving.
/// Its authored write follows successful candidate admission; failed planning
/// preserves the original manifest and the lock snapshot available for retry.
pub fn plan<S: InstallSource + ?Sized>(
    source: &S,
    project_root: &Path,
    request: InstallRequest,
    observer: &dyn PlanObserver,
) -> Result<Plan> {
    plan_with_spec_format(source, project_root, request, SpecFormat::Mixed, observer)
}

/// Plan an install under the effective PROP-045 spec representation.
///
/// The COMPATIBILITY shape: it performs its one legacy read of the selected
/// manifest and its one legacy discovery, then delegates to
/// [`plan_prepared_with_spec_format`], the prepared compatibility entrypoint. A caller
/// that already owns those two values calls the prepared sibling directly and
/// touches the disk for neither.
pub fn plan_with_spec_format<S: InstallSource + ?Sized>(
    source: &S,
    project_root: &Path,
    request: InstallRequest,
    spec_format: SpecFormat,
    observer: &dyn PlanObserver,
) -> Result<Plan> {
    let selected = Workspace::discover_selected(project_root)?;
    let mut workspace = selected.workspace;
    let selected_root = selected.selected_root;
    let mut manifest = Manifest::read(selected_root.join(Manifest::FILENAME))?;
    plan_prepared_with_spec_format(
        source,
        &selected_root,
        &mut manifest,
        &mut workspace,
        request,
        spec_format,
        observer,
    )
}

/// Plan from prepared values and publish any successful legacy migration only
/// after all consumed candidates are admitted. Failed plans leave both the
/// physical manifest and the caller's source snapshots unchanged.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
pub fn plan_prepared_with_spec_format<S: InstallSource + ?Sized>(
    source: &S,
    project_root: &Path,
    manifest: &mut Manifest,
    workspace: &mut Workspace,
    request: InstallRequest,
    spec_format: SpecFormat,
    observer: &dyn PlanObserver,
) -> Result<Plan> {
    let before = manifest.clone();
    let original_workspace = workspace.clone();
    let result = plan_prepared_without_publication(
        source,
        project_root,
        manifest,
        workspace,
        request,
        spec_format,
        observer,
    )?;
    if *manifest != before
        && let Err(error) = crate::apply::publish_admitted_manifest(project_root, &before, manifest)
    {
        *manifest = before;
        *workspace = original_workspace;
        return Err(error);
    }
    Ok(result)
}

/// Plan an install from values the caller ALREADY owns.
///
/// This is the real planning algorithm without authored-file publication.
/// Its operation owner may commit the admitted raw snapshot after success.
/// It reads neither the selected
/// `vibe.toml` nor the workspace tree: `manifest` is the raw selected manifest
/// (unexpanded, the shape `[i18n]`, visibility and the case-c migration write
/// operate on) and `workspace` is the finalised tree (expanded, the shape the
/// root union and the freshness check read). Every place the compatibility
/// wrapper used to re-`discover` now consults the supplied tree instead.
///
/// Both are taken by `&mut` because case-c migration is a real mutation: it
/// seeds the raw manifest from the lockfile in memory and applies the same
/// concrete requirement delta to the exact selected node of the finalised
/// tree. It is a delta and not a wholesale copy on purpose — the finalised
/// node has had `[workspace.versions]` placeholders resolved, and overwriting
/// it with the raw table would undo that.
#[allow(clippy::too_many_arguments)]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
pub fn plan_prepared_without_publication<S: InstallSource + ?Sized>(
    source: &S,
    project_root: &Path,
    manifest: &mut Manifest,
    workspace: &mut Workspace,
    request: InstallRequest,
    spec_format: SpecFormat,
    observer: &dyn PlanObserver,
) -> Result<Plan> {
    let before = manifest.clone();
    let previous_workspace = workspace.clone();
    let result = match prepare_metadata_plan(
        source,
        project_root,
        manifest,
        workspace,
        request,
        spec_format,
        observer,
        PlanningPurpose::Apply,
    )? {
        None => Ok(Plan::Fresh),
        Some(prepared) => {
            acquire_prepared_metadata(source, prepared).map(|plan| Plan::Ready(Box::new(plan)))
        }
    };
    if result.is_err() {
        *manifest = before;
        *workspace = previous_workspace;
    }
    result
}

#[derive(Clone, Copy)]
enum PlanningPurpose<'a> {
    Apply,
    Preview(&'a Lockfile, PreviewSourceStorage<'a>),
}

struct PreparedMetadata {
    project_root: PathBuf,
    request: InstallRequest,
    manifest: Manifest,
    lockfile: Lockfile,
    language_chain: Vec<String>,
    workspace: Workspace,
    progress: vibe_core::progress::Progress,
    closure: metadata::MetadataClosure,
    /// The operation's configured-candidate snapshot. Acquisition re-binds it
    /// so payload admission keeps the same configured boundary as the solve.
    configuration: Option<vibe_resolver::configuration::ConfigurationSnapshot>,
}

#[allow(clippy::too_many_arguments)]
fn prepare_metadata_plan<S: InstallSource + ?Sized>(
    source: &S,
    project_root: &Path,
    manifest: &mut Manifest,
    workspace: &mut Workspace,
    mut request: InstallRequest,
    spec_format: SpecFormat,
    observer: &dyn PlanObserver,
    purpose: PlanningPurpose<'_>,
) -> Result<Option<PreparedMetadata>> {
    let selected = roots::selected_node(workspace, project_root)?;
    crate::configuration_admission::workspace_inputs_for_install(
        workspace,
        project_root,
        manifest,
    )?;
    request.roots = roots::unique_requests(&request.roots)?;
    let lockfile = match purpose {
        PlanningPurpose::Apply => load_or_empty_lockfile(&workspace.root, &request.generated_by)?,
        PlanningPurpose::Preview(snapshot, _) => snapshot.clone(),
    };
    match purpose {
        PlanningPurpose::Apply => {
            crate::configuration_admission::retained_before_planning(source, workspace, &lockfile)?
        }
        PlanningPurpose::Preview(_, storage) => {
            crate::configuration_admission::retained_for_preview(
                source, workspace, &lockfile, storage,
            )?
        }
    }
    let before_migration = manifest.clone();
    let mut workspace_before_migration = None;

    // PROP-003 §2.7 language chain (caller override > project [i18n]).
    let language_chain = build_language_chain(request.language.as_deref(), manifest);

    // Legacy seeding is permitted only for a genuinely undeclared workspace,
    // never for an empty selected node whose siblings own requirements.
    if request.roots.is_empty()
        && !roots::has_declarations(workspace)
        && !roots::raw_has_declarations(manifest)
        && !lockfile.meta.root_dependencies.is_empty()
    {
        let migrated = roots::migration_roots(&lockfile)?;
        observer.on(PlanEvent::MigratingRequires {
            entries: migrated.len(),
        });
        workspace_before_migration = Some(workspace.clone());
        manifest.requires.packages.clone_from(&migrated);
        migrate_selected_node(workspace, project_root, &migrated);
        // Seed the semantic world before resolution; physical publication is
        // permitted only at a successful, fully admitted plan boundary.
    }
    let result = (|| {
        if workspace_before_migration.is_some() {
            workspace.advance_authored_selected(project_root, &before_migration, manifest)?;
        }
        let requirements = roots::RootRequirements::collect(workspace, &selected, &request.roots)?;
        let inputs = requirements.configuration_inputs();
        let configured = inputs
            .values()
            .any(|consumers| consumers.values().any(Option::is_some));
        if configured
            && (request.features.all
                || request.features.no_defaults
                || !request.features.explicit.is_empty())
        {
            return Err(configured::mixed_flags());
        }
        let planning_source = configured::PlanningSource {
            source,
            configuration: configured
                .then(|| vibe_resolver::configuration::ConfigurationSnapshot::new(inputs)),
        };
        let source = &planning_source;
        let roots = requirements.authored();
        let root_count = requirements.package_count();
        let root_edges = requirements.source_edges();

        // ##EMPTY-REQUIRES-IS-A-NO-OP (PROP-011, the 2026-08-24 ruling): zero
        // dependencies is a fresh project, not an error — regenerate the boot
        // artifacts of the empty world and report the fresh shape.
        if roots.is_empty() && matches!(purpose, PlanningPurpose::Apply) {
            return Ok(None);
        }

        let progress = observer.progress();
        progress.checkpoint()?;
        let resolution_task = progress.task("Resolving dependencies");
        resolution_task.detail(format!(
            "{} root package{} from {} consumer declarations",
            root_count,
            if root_count == 1 { "" } else { "s" },
            roots.len()
        ));

        // Authored requirements remain unchanged for fallback and diagnostics.
        let mut solve_roots = if request.roots.is_empty() {
            roots.clone()
        } else {
            requirements.held(&lockfile, &request.roots)
        };

        // PROP-011 §2.2 — the freshness fast path. When no explicit pkgref
        // was given (the install-from-manifest shape) and `vibe.lock` is
        // already a correct resolution of every node's `[requires]`, the
        // depsolver — a registry walk over the network — is skipped
        // entirely: the resolution is the lock, and application is just a
        // whole-tree boot regeneration (cheap, self-healing — PROP-011
        // §2.4). This is also what makes `vibe install`
        // lockfile-respecting: a fresh lock is honoured verbatim, with no
        // version drift inside a constraint. An explicit
        // `vibe install <pkgref>` always runs the full pipeline.
        if request.roots.is_empty() && matches!(purpose, PlanningPurpose::Apply) {
            match freshness_prepared(
                project_root,
                manifest,
                workspace,
                &request,
                &lockfile,
                spec_format,
            )? {
                vibe_workspace::freshness::Freshness::Fresh => {
                    resolution_task.skip("vibe.lock is fresh");
                    return Ok(None);
                }
                vibe_workspace::freshness::Freshness::Stale(reason) => {
                    observer.on(PlanEvent::Reresolving {
                        reason: reason.clone(),
                    });
                    // PROP-011 §5.3 — minimum churn: hold the locked
                    // version of every root the change did not touch, so
                    // re-resolution moves only the changed dependency and
                    // its subtree.
                    solve_roots = vibe_workspace::freshness::hold_pins(&roots, &lockfile);
                }
            }
        }

        let root_id = visibility_root_id(manifest);
        let closure = metadata::resolve_metadata_closure(
            source,
            metadata::MetadataPlanningInput {
                workspace,
                root_manifest: manifest,
                roots: &roots,
                root_edges: &root_edges,
                held_roots: &solve_roots,
                lockfile: &lockfile,
                root_features: &request.features,
                language_chain: &language_chain,
                project_root,
                root_id: &root_id,
                observer,
                recommends: !request.no_recommends,
            },
        )
        .map_err(|error| {
            // Completion is explicit: a refused solve reports failure, while a
            // cancellation keeps the task's honest `Stopped` drop outcome.
            if !matches!(error, Error::Cancelled(_)) {
                resolution_task.fail("dependency resolution failed");
            }
            requirements.contextualize(error)
        })?;
        for selected in closure.selected.values() {
            progress.checkpoint()?;
            let package = selected.candidate.manifest().require_package()?;
            if package.kind.is_read_only() {
                return Err(Error::DocNotInstalled {
                    coordinate: format!("{}/{}", package.group, package.name),
                });
            }
            crate::configuration_admission::admitted_package(
                format!("{}/{}", package.group, package.name),
                selected.candidate.manifest(),
                selected.configuration.as_ref(),
            )?;
        }
        progress.checkpoint()?;
        requirements.check_metadata_kinds(&closure.selected)?;
        if !request.features.explicit.is_empty() {
            let accepted: BTreeSet<&str> = closure
                .graph
                .roots()
                .filter_map(|node| {
                    closure
                        .feature_expansions
                        .get(&(node.group.clone(), node.name.clone()))
                })
                .flat_map(|features| features.active_features.iter().map(String::as_str))
                .collect();
            let unmatched = request
                .features
                .explicit
                .iter()
                .filter(|feature| !accepted.contains(feature.as_str()))
                .cloned()
                .collect::<Vec<_>>();
            if !unmatched.is_empty() {
                observer.on(PlanEvent::FeaturesUnmatched {
                    features: unmatched,
                });
            }
        }
        let selected_count = closure.graph.packages.len();
        resolution_task.detail(format!(
            "selected {} package{}",
            selected_count,
            if selected_count == 1 { "" } else { "s" }
        ));
        resolution_task.finish();
        Ok(Some(PreparedMetadata {
            project_root: project_root.to_owned(),
            request,
            manifest: manifest.clone(),
            lockfile,
            language_chain,
            workspace: workspace.clone(),
            progress,
            closure,
            configuration: planning_source.configuration,
        }))
    })();
    if result.is_err()
        && let Some(original_workspace) = workspace_before_migration
    {
        *manifest = before_migration;
        *workspace = original_workspace;
    }
    result
}

mod acquire;
use acquire::acquire_prepared_metadata;
mod metadata;
mod preview;
pub use preview::{
    InstallPreview, PreviewSourceStorage, preview_has_requirements, preview_prepared,
};

/// The acquisition half of planning — fetch/defer helpers and the
/// conditional-dependency expansion, split along the seam when the
/// migration pushed the file past the 600-line budget.
mod fetch;

mod roots;
mod selection;
mod weak;
use selection::migrate_selected_node;
pub(crate) use selection::visibility_root_id;
pub use weak::{RecommendationOutcome, RecommendationRow, SuggestionRow, WeakDependencyReport};

mod configured;

mod fresh;
pub use fresh::freshness_prepared;

mod report;
