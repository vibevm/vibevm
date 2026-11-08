//! Payload acquisition from the metadata phase's exact retained candidates.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE");
use crate::{
    InstallSource,
    error::Result,
    fetched::{Fetched, NodeInstallMeta},
};
use std::path::Path;
use vibe_core::manifest::Lockfile;
use vibe_registry::{CachedPackage, RegistryError, ResolvedPackage};
use vibe_resolver::ResolvedNode;
use vibe_workspace::vibedeps;

/// Acquire the already selected metadata candidate. No name/version source
/// resolution is performed after the metadata closure has been accepted.
#[allow(clippy::too_many_arguments)]
pub(super) fn fetch_selected_node<S: InstallSource + ?Sized>(
    source: &S,
    node: &ResolvedNode,
    metadata: &vibe_resolver::CandidateMetadata,
    features: &vibe_resolver::FeatureExpansion,
    lockfile: &Lockfile,
    store_root: &Path,
    workspace_root: &Path,
    offline: bool,
    progress: &vibe_core::progress::Progress,
) -> Result<Fetched> {
    let selected = &metadata.candidate;
    if selected.group() != &node.group
        || selected.name() != node.name
        || selected.version() != &node.version
    {
        return Err(RegistryError::MalformedMeta {
            path: workspace_root.to_owned(),
            reason: "candidate metadata identity differs from the selected graph".into(),
        }
        .into());
    }
    let expected = lockfile
        .find(&node.group, &node.name)
        .filter(|record| record.version == node.version)
        .map(|record| record.content_hash.as_str());
    let slot = vibedeps::in_place_slot_abs_path(workspace_root, &node.group, &node.name);
    let (cached, in_place) = match source
        .acquire_candidate_for_install(selected, store_root, expected, &slot, progress)?
    {
        vibe_registry::PackageAcquisition::Cached(cached) => (*cached, None),
        vibe_registry::PackageAcquisition::InPlace(prepared) => {
            let cached = cached_prepared(node, &slot, &prepared)?;
            (cached, Some(prepared))
        }
    };
    if &cached.manifest != selected.manifest() {
        return Err(super::configured::manifest_drift(&cached.cache_dir).into());
    }
    super::configured::admit_configured_acquisition(&cached, metadata)?;
    crate::configuration_admission::admitted_package(
        format!("{}/{}", node.group, node.name),
        &cached.manifest,
        metadata.configuration.as_ref(),
    )?;
    let embedded_sources =
        vibe_registry::lock_embedded_sources_with(&cached.manifest.embedded_sources, offline)?;
    Ok(Fetched {
        cached,
        feature_expansion: features.clone(),
        meta: NodeInstallMeta {
            dependencies: node.dependencies.clone(),
            is_root: node.is_root,
        },
        embedded_sources,
        in_place,
        configuration: metadata.configuration.clone(),
    })
}

fn cached_prepared(
    node: &ResolvedNode,
    slot: &Path,
    prepared: &vibe_registry::PreparedInPlace,
) -> Result<CachedPackage> {
    if prepared.slot() != slot {
        return Err(RegistryError::MalformedMeta {
            path: slot.to_owned(),
            reason: "prepared source belongs to another destination".into(),
        }
        .into());
    }
    let manifest = prepared.manifest().clone();
    let source_kind = prepared.recorded_source_kind();
    let evidence = prepared.source_evidence();
    let cached = CachedPackage {
        locked_selection: prepared.locked_selection().cloned(),
        route_publication: prepared.publication_scope(),
        recorded_source_kind: prepared.recorded_source_kind(),
        store_reused: evidence.store_reused,
        acquisition: evidence.acquisition,
        resolved: ResolvedPackage {
            group: node.group.clone(),
            name: node.name.clone(),
            version: node.version.clone(),
            source_dir: slot.to_path_buf(),
        },
        cache_dir: slot.to_path_buf(),
        manifest,
        // Prospective identity is retained with its preparation; this path is
        // not consumed as materialized content until the bound apply succeeds.
        content_hash: prepared.content_hash().to_owned(),
        source_uri: prepared.source_uri().to_owned(),
        fetched_from: evidence.fetched_from,
        registry_name: evidence.registry,
        source_ref: Some(prepared.source_ref().to_owned()),
        resolved_commit: Some(prepared.resolved_commit().to_owned()),
        overridden: source_kind == Some(vibe_core::manifest::SourceKind::Override),
        is_git_source: source_kind == Some(vibe_core::manifest::SourceKind::Git),
        is_path_source: false,
        is_embedded: false,
        is_local: false,
        via_redirect: evidence.via_redirect,
    };
    Ok(cached)
}
