//! The planning phase — derive roots and one source-bound metadata closure,
//! including visibility, configuration and conditional activation. Preview
//! returns that selection; installation acquires its exact retained candidates
//! before
//! [`apply_with_spec_format_and_lifecycle_observed`](crate::apply_with_spec_format_and_lifecycle_observed)
//! publishes project state.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::path::{Path, PathBuf};

use vibe_core::PackageRef;
use vibe_core::manifest::{Lockfile, Manifest, SpecFormat};
use vibe_resolver::FeatureRequest;
use vibe_workspace::Workspace;
use vibe_workspace::install::ResolvedDep;

use crate::InstallSource;
use crate::error::{Error, Result};
use crate::events::PlanObserver;
use crate::fetched::Fetched;

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
    /// confirmation before
    /// [`apply_with_spec_format_and_lifecycle_observed`](crate::apply_with_spec_format_and_lifecycle_observed).
    Ready(Box<PlannedInstall>),
}

/// A confirmed-pending resolution — everything
/// [`apply_with_spec_format_and_lifecycle_observed`](crate::apply_with_spec_format_and_lifecycle_observed)
/// needs, carried by value so the apply phase cannot observe state
/// the plan did not produce.
pub type NodeRootProjection = std::collections::BTreeMap<String, Vec<PackageRef>>;
#[derive(Debug)]
pub struct PlannedInstall {
    pub(crate) node_roots: std::collections::BTreeMap<String, Vec<PackageRef>>,
    pub(crate) project_root: PathBuf,
    pub(crate) request: InstallRequest,
    pub(crate) submitted: Option<crate::submission::VerifiedSubmission>,
    pub(crate) path_checks: Vec<crate::configuration_admission::host_path::PathCheck>,
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
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail"
)]
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
    plan_prepared_with_selected_roots(
        source,
        project_root,
        manifest,
        workspace,
        request,
        spec_format,
        observer,
        &[],
    )
}

/// Continue ordinary planning with exact top-level candidates selected by this source owner.
#[allow(clippy::too_many_arguments)]
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail"
)]
pub fn plan_prepared_with_selected_roots<S: InstallSource + ?Sized>(
    source: &S,
    project_root: &Path,
    manifest: &mut Manifest,
    workspace: &mut Workspace,
    request: InstallRequest,
    spec_format: SpecFormat,
    observer: &dyn PlanObserver,
    selected_roots: &[vibe_registry::source_candidates::RetainedSourceCandidate],
) -> Result<Plan> {
    plan_impl(
        source,
        project_root,
        manifest,
        workspace,
        request,
        spec_format,
        observer,
        selected_roots,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail"
)]
pub fn plan_prepared_with_submission<S: InstallSource + ?Sized>(
    source: &S,
    root: &Path,
    manifest: &mut Manifest,
    workspace: &mut Workspace,
    request: InstallRequest,
    format: SpecFormat,
    observer: &dyn PlanObserver,
    submission: vibe_core::manifest::SubmittedInput,
) -> Result<Plan> {
    plan_impl(
        source,
        root,
        manifest,
        workspace,
        request,
        format,
        observer,
        &[],
        Some(submission),
    )
}
#[allow(clippy::too_many_arguments)]
fn plan_impl<S: InstallSource + ?Sized>(
    source: &S,
    project_root: &Path,
    manifest: &mut Manifest,
    workspace: &mut Workspace,
    request: InstallRequest,
    spec_format: SpecFormat,
    observer: &dyn PlanObserver,
    selected_roots: &[vibe_registry::source_candidates::RetainedSourceCandidate],
    submission: Option<vibe_core::manifest::SubmittedInput>,
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
        selected_roots,
        submission.as_ref(),
    )? {
        None => Ok(Plan::Fresh),
        Some(prepared) => {
            let mut plan = acquire_prepared_metadata(source, prepared)?;
            if let Some(input) = submission {
                plan.submitted = Some(crate::submission::VerifiedSubmission::mint(
                    &mut plan, input,
                )?);
            }
            crate::configuration_admission::planned(&plan)?;
            Ok(Plan::Ready(Box::new(plan)))
        }
    };
    if result.is_err() {
        *manifest = before;
        *workspace = previous_workspace;
    }
    result
}

mod acquire;
use acquire::acquire_prepared_metadata;
mod prepare;
use prepare::{PlanningPurpose, PreparedMetadata, prepare_metadata_plan};
mod metadata;
mod preview;
pub use preview::{
    InstallPreview, PreviewSourceStorage, preview_has_requirements, preview_prepared,
    preview_prepared_with_selected_roots,
};

/// The acquisition half of planning — fetch/defer helpers and the
/// conditional-dependency expansion, split along the seam when the
/// migration pushed the file past the 600-line budget.
mod fetch;

mod roots;
mod selection;
mod weak;
pub(crate) use selection::visibility_root_id;
pub use weak::{RecommendationOutcome, RecommendationRow, SuggestionRow, WeakDependencyReport};

mod configured;

mod fresh;
pub use fresh::{freshness_prepared, project_fresh_node_roots};

mod report;

/// Reuse the ordinary fixed-consumer collector for a governed whole-workspace update.
pub fn configured_workspace_inputs(
    workspace: &Workspace,
    selected_root: &Path,
) -> Result<vibe_resolver::configuration::PackageConfigurationInputs> {
    let selected = roots::selected_node(workspace, selected_root)?;
    Ok(roots::RootRequirements::collect(workspace, &selected, &[])?.configuration_inputs())
}
pub use configured::PlanningSource;
