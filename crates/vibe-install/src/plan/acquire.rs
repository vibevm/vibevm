//! The payload-acquisition half of `plan`: fetch every selected node from its
//! retained metadata binding and assemble the [`PlannedInstall`]. Split from
//! `plan.rs` along the metadata/acquisition seam for the file-length budget.
specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use vibe_core::ContentHash;
use vibe_core::visibility::ProvenanceRule;
use vibe_registry::store;
use vibe_workspace::freshness::is_in_workspace_file_source;
use vibe_workspace::install::ResolvedDep;

use super::{PlannedInstall, PreparedMetadata, configured, fetch};
use crate::InstallSource;
use crate::error::Result;

pub(super) fn acquire_prepared_metadata<S: InstallSource + ?Sized>(
    source: &S,
    prepared: PreparedMetadata,
) -> Result<PlannedInstall> {
    let PreparedMetadata {
        project_root,
        request,
        manifest,
        lockfile,
        language_chain,
        workspace,
        progress,
        closure,
        configuration,
    } = prepared;
    let planning_source = configured::PlanningSource {
        source,
        configuration,
    };
    let source = &planning_source;
    let store_root = store::store_root()?;
    let fetch_task = progress.task("Fetching package content");
    let mut fetched = Vec::with_capacity(closure.graph.packages.len());
    for node in closure.graph.iter() {
        progress.checkpoint()?;
        fetch_task.detail(format!("{}/{}@{}", node.group, node.name, node.version));
        let selected = closure
            .selected
            .get(&(node.group.clone(), node.name.clone()))
            .ok_or_else(|| vibe_registry::RegistryError::MalformedMeta {
                path: project_root.clone(),
                reason: "selected graph lost its retained metadata binding".into(),
            })?;
        let features = closure
            .feature_expansions
            .get(&(node.group.clone(), node.name.clone()))
            .ok_or_else(|| vibe_registry::RegistryError::MalformedMeta {
                path: project_root.clone(),
                reason: "selected graph lost its feature expansion".into(),
            })?;
        fetched.push(fetch::fetch_selected_node(
            source,
            node,
            selected,
            features,
            &lockfile,
            &store_root,
            &workspace.root,
            request.offline,
            &fetch_task.progress(),
        )?);
        fetch_task.set_progress(
            fetched.len() as u64,
            Some(closure.graph.packages.len() as u64),
            "packages",
        );
    }
    fetch_task.finish();
    let resolution: Vec<ResolvedDep> = fetched
        .iter()
        .map(|f| {
            let coordinate = format!("{}/{}", f.cached.resolved.group, f.cached.resolved.name);
            let provenance = closure.visibility_analysis.effective.get(&coordinate);
            ResolvedDep {
                kind: f.cached.package_meta().kind,
                group: f.cached.resolved.group.clone(),
                name: f.cached.resolved.name.clone(),
                version: f.cached.resolved.version.clone(),
                content_dir: f.cached.cache_dir.clone(),
                source_hash: Some(ContentHash::from_validated(f.cached.content_hash.clone())),
                embedded_sources: f.embedded_sources.clone(),
                manifest: f.cached.manifest.clone(),
                // A `[requires.packages]` dependency pkgref is
                // group-qualified (PROP-008 §2.6).
                requires: f
                    .meta
                    .dependencies
                    .iter()
                    .filter_map(|p| p.group.clone().map(|g| (g, p.name.to_string())))
                    .collect(),
                // A chain that crossed a recommendation says so, and that is
                // how the lock records a weak member (PROP-077
                // `#PROJECT-RECOMMENDS`): no new lock field, one new value of
                // the provenance member the lock already carries.
                admitted_by: provenance.map(|witness| match (witness.weak, witness.rule) {
                    (true, _) => "weak-chain".to_string(),
                    (false, ProvenanceRule::RootEdge) => "root-edge".to_string(),
                    (false, ProvenanceRule::PublicChain) => "public-chain".to_string(),
                    (false, ProvenanceRule::FriendsChain) => "friends-chain".to_string(),
                }),
                via_override: provenance.and_then(|witness| witness.via_override.clone()),
                // Mutable iff an in-workspace `file://` self-hosting source (§2.6).
                source_mutable: is_in_workspace_file_source(&f.cached.source_uri, &workspace.root),
                in_place_changed: None,
                configuration: f.configuration.clone(),
            }
        })
        .collect();

    let source_candidates = closure
        .selected
        .iter()
        .map(|(key, metadata)| (key.clone(), metadata.observations.clone()))
        .collect();
    let path_records = fetched
        .iter()
        .filter_map(|f| {
            f.configuration.as_ref().map(|record| {
                (
                    format!("{}/{}", f.cached.resolved.group, f.cached.resolved.name),
                    record,
                )
            })
        })
        .collect::<Vec<_>>();
    for f in &fetched {
        if f.configuration
            .as_ref()
            .is_some_and(|r| !r.host_path_targets().is_empty())
            && (!f.cached.source_uri.starts_with("file://")
                || f.cached.registry_name.is_none()
                || f.cached.source_kind() != Some(vibe_core::manifest::SourceKind::Registry)
                || f.cached.package_meta().kind != vibe_core::PackageKind::Flow
                || f.cached.package_meta().materialization
                    != vibe_core::manifest::Materialization::Copy
                || !manifest.requires.packages.iter().any(|r| {
                    r.qualified_name()
                        == format!("{}/{}", f.cached.resolved.group, f.cached.resolved.name)
                        && r.version.matches(&f.cached.resolved.version)
                })
                || f.cached.is_git_source
                || f.cached.is_path_source
                || f.cached.overridden
                || f.cached.via_redirect.is_some()
                || f.cached.template.is_some()
                || f.in_place.is_some())
        {
            return Err(crate::configuration_admission::host_path::unsupported());
        }
    }
    let path_checks = crate::configuration_admission::host_path::capture(
        &workspace,
        &project_root,
        &manifest,
        path_records
            .iter()
            .map(|(key, record)| (key.as_str(), *record)),
    )?;
    let planned = PlannedInstall {
        submitted: None,
        path_checks,
        node_roots: closure.node_roots,
        weak: closure.weak,
        project_root,
        request,
        manifest,
        lockfile,
        language_chain,
        fetched,
        resolution,
        workspace,
        progress,
        source_candidates,
    };
    crate::configuration_admission::planned(&planned)?;
    Ok(planned)
}
