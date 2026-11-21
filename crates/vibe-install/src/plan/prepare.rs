//! Metadata preparation — the one planning walk shared by installation and
//! preview: root collection, retained-configuration admission, the legacy
//! requires migration, the freshness fast path and the metadata closure.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use vibe_core::manifest::{Lockfile, Manifest, SpecFormat};
use vibe_workspace::Workspace;

use super::selection::migrate_selected_node;
use super::{
    InstallRequest, PreviewSourceStorage, configured, freshness_prepared, metadata, roots,
    visibility_root_id,
};
use crate::InstallSource;
use crate::error::{Error, Result};
use crate::events::{PlanEvent, PlanObserver};
use crate::fetched::{build_language_chain, load_or_empty_lockfile};

#[derive(Clone, Copy)]
pub(super) enum PlanningPurpose<'a> {
    Apply,
    Preview(&'a Lockfile, PreviewSourceStorage<'a>),
}

pub(super) struct PreparedMetadata {
    pub(super) project_root: PathBuf,
    pub(super) request: InstallRequest,
    pub(super) manifest: Manifest,
    pub(super) lockfile: Lockfile,
    pub(super) language_chain: Vec<String>,
    pub(super) workspace: Workspace,
    pub(super) progress: vibe_core::progress::Progress,
    pub(super) closure: metadata::MetadataClosure,
    /// The operation's configured-candidate snapshot. Acquisition re-binds it
    /// so payload admission keeps the same configured boundary as the solve.
    pub(super) configuration: Option<vibe_resolver::configuration::ConfigurationSnapshot>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn prepare_metadata_plan<S: InstallSource + ?Sized>(
    source: &S,
    project_root: &Path,
    manifest: &mut Manifest,
    workspace: &mut Workspace,
    mut request: InstallRequest,
    spec_format: SpecFormat,
    observer: &dyn PlanObserver,
    purpose: PlanningPurpose<'_>,
    selected_roots: &[vibe_registry::source_candidates::RetainedSourceCandidate],
    submission: Option<&vibe_core::manifest::SubmittedInput>,
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
        let mut inputs = requirements.configuration_inputs();
        if let Some(input) = submission {
            crate::submission::check_root(project_root, manifest, input)?;
            let owners = inputs
                .get_mut(&input.requirement.qualified_name())
                .ok_or_else(|| vibe_core::Error::InvalidLockfile {
                    reason: "submitted root missing from planning".into(),
                })?;
            if owners.len() != 1 || owners.get("./vibe.toml") != Some(&None) {
                return Err(vibe_core::Error::InvalidLockfile {
                    reason: "submitted input conflicts with authored root".into(),
                }
                .into());
            }
            owners.insert("./vibe.toml".into(), Some(input.original.clone()));
        }
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
            configuration: configured.then(|| {
                let snapshot = vibe_resolver::configuration::ConfigurationSnapshot::with_history(
                    inputs,
                    lockfile.configurations.clone(),
                );
                let roots = requirements.authored();
                if request.roots.is_empty()
                    && roots.len() == 1
                    && roots == lockfile.meta.root_dependencies
                    && roots[0].version.is_exact_pin()
                {
                    snapshot.with_guarded_locked_rows(&lockfile.packages)
                } else {
                    snapshot
                }
            }),
        };
        let source = &planning_source;
        let roots = requirements.authored();
        let root_count = requirements.package_count();
        let root_edges = requirements.source_edges();
        let root_owners = requirements.node_owners();

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
                selected_roots,
                workspace,
                root_manifest: manifest,
                roots: &roots,
                root_edges: &root_edges,
                root_owners: &root_owners,
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
            crate::configuration_admission::install_candidate(
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
