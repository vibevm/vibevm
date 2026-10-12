//! Member-owner visibility over the retained captured manifest closure.
//! Core `analyze` remains the single access/friend/exclude/override engine.

use vibe_core::manifest::Manifest;
use vibe_core::visibility::{Analysis, EdgeDecl, NodeDecl, VisibilityGraph, analyze};

use crate::extension_world::ExtensionWorldEpoch;

use super::SourceQueryUnavailable;
use super::capture::{PreparedOwnerCapture, captured_manifest_closure};

pub(super) fn owner_visibility(
    epoch: &ExtensionWorldEpoch,
    node_root: &std::path::Path,
    node_manifest: &Manifest,
    captures: &[PreparedOwnerCapture],
) -> Result<Analysis, SourceQueryUnavailable> {
    let owner = epoch.node_owner_view(node_root, node_manifest)?;
    let root_id = owner.host.provider.identity.to_string();
    let retained = captured_manifest_closure(epoch, captures)?;
    let mut graph = VisibilityGraph::default();
    graph
        .nodes
        .insert(root_id.clone(), declaration(node_manifest)?);
    for source in &owner.installed {
        let id = source.provider.id.to_string();
        let key = (
            source.provider.id.group().as_str().to_owned(),
            source.provider.id.name().as_str().to_owned(),
        );
        let manifest = retained.get(&key).ok_or_else(|| {
            SourceQueryUnavailable::MissingPreparedCapture(format!("{id} manifest"))
        })?;
        graph.nodes.insert(id, declaration(manifest)?);
    }
    Ok(analyze(&graph, &root_id))
}

fn declaration(manifest: &Manifest) -> Result<NodeDecl, SourceQueryUnavailable> {
    let edges = manifest
        .requires
        .iter_pkgrefs()
        .filter_map(|(group, name)| {
            group.map(|group| EdgeDecl {
                to: format!("{group}/{name}"),
                access: manifest.requires.declared_access(group, name),
                friend: manifest.requires.declared_friend(group, name),
                exclude: manifest.requires.excludes_for(group, name).to_vec(),
            })
        })
        .collect();
    let visibility = manifest.visibility.clone().unwrap_or_default();
    let overrides = manifest
        .override_table
        .as_ref()
        .map(|table| {
            table.targets().map(|targets| {
                targets
                    .into_iter()
                    .map(|(target, entry)| (target, entry.clone()))
                    .collect()
            })
        })
        .transpose()
        .map_err(|error| SourceQueryUnavailable::Visibility(format!("owner override: {error}")))?
        .unwrap_or_default();
    Ok(NodeDecl {
        edges,
        friends: visibility.friends,
        unfriend: visibility.unfriend,
        allow_friends: visibility.allow_friends,
        overrides,
    })
}
