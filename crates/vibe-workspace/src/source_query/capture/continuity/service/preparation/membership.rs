//! Only the selected ancestor membership chain, never sibling manifests.
use super::*;
use crate::{Workspace, WorkspaceMember};
use vibe_core::RelPath;

pub(super) struct SelectedMembership {
    pub(super) workspace: Workspace,
    pub(super) node_rel: String,
    pub(super) node_root: PathBuf,
    pub(super) snapshots: BTreeMap<String, StableFileSnapshot>,
}

pub(super) fn selected(root: &Path) -> Result<SelectedMembership, QueryError> {
    let node_root = root
        .canonicalize()
        .map_err(|_| unavailable("selected node root"))?;
    let node_project = Project::open(&node_root).map_err(|_| unavailable("held selected node"))?;
    let node_snapshot = snapshot(&node_project, "vibe.toml", MANIFEST_CAP)?;
    let node_manifest = parse_manifest(&node_snapshot)?;
    let mut chain = vec![(node_root.clone(), node_manifest.clone(), node_snapshot)];
    let mut cursor = node_root.parent();
    while let Some(directory) = cursor {
        if chain.len() > 64 {
            return Err(unavailable("membership depth bound"));
        }
        // A candidate unrelated ancestor is not this node's authority. Only
        // its declaration of the exact retained child extends the chain.
        let project = match Project::open(directory) {
            Ok(project) => project,
            Err(_) => {
                cursor = directory.parent();
                continue;
            }
        };
        if let Ok(Some(observed)) = project.read_file_snapshot_bounded("vibe.toml", MANIFEST_CAP) {
            if let Ok(manifest) = parse_manifest(&observed) {
                let child = &chain.last().unwrap().0;
                let relative = child
                    .strip_prefix(directory)
                    .map_err(|_| unavailable("membership containment"))?
                    .to_string_lossy()
                    .replace('\\', "/");
                if manifest.workspace.as_ref().is_some_and(|ws| {
                    ws.members.iter().any(|pattern| {
                        glob::Pattern::new(pattern).is_ok_and(|pattern| {
                            pattern.matches_with(
                                &relative,
                                glob::MatchOptions {
                                    case_sensitive: true,
                                    require_literal_separator: true,
                                    require_literal_leading_dot: true,
                                },
                            )
                        })
                    })
                }) {
                    chain.push((directory.to_path_buf(), manifest, observed));
                }
            }
        }
        cursor = directory.parent();
    }
    chain.reverse();
    let workspace_root = chain[0].0.clone();
    let root_manifest = chain[0].1.clone();
    let mut members = Vec::new();
    let mut snapshots = BTreeMap::new();
    snapshots.insert(".".into(), chain[0].2.clone());
    let mut parent = None;
    for (depth, (directory, manifest, observed)) in chain.iter().skip(1).enumerate() {
        let rel = directory
            .strip_prefix(&workspace_root)
            .map_err(|_| unavailable("member containment"))?
            .to_string_lossy()
            .replace('\\', "/");
        snapshots.insert(rel.clone(), observed.clone());
        members.push(WorkspaceMember {
            rel_path: RelPath::new(rel.clone()),
            manifest: manifest.clone(),
            depth,
            parent: parent.clone(),
        });
        parent = Some(RelPath::new(rel));
    }
    let node_rel = if node_root == workspace_root {
        ".".into()
    } else {
        node_root
            .strip_prefix(&workspace_root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/")
    };
    Ok(SelectedMembership {
        workspace: Workspace {
            root: workspace_root,
            root_manifest,
            members,
        },
        node_rel,
        node_root,
        snapshots,
    })
}

pub(super) fn parse_manifest(snapshot: &StableFileSnapshot) -> Result<Manifest, QueryError> {
    let text = std::str::from_utf8(&snapshot.bytes).map_err(|_| unavailable("manifest UTF-8"))?;
    Manifest::parse_str(text).map_err(|_| unavailable("manifest grammar"))
}

pub(super) fn effective_node(selected: &SelectedMembership) -> Result<Manifest, QueryError> {
    // Reuse the incumbent version finalizer on ONLY retained chain metadata.
    // The raw manifests stay the B2 witnesses; no authored bytes are rewritten.
    let mut workspace = selected.workspace.clone();
    crate::expand::finalize_versions(&mut workspace.root_manifest, &mut workspace.members)
        .map_err(|_| unavailable("inherited selected versions"))?;
    if selected.node_rel == "." {
        Ok(workspace.root_manifest)
    } else {
        workspace
            .member_by_rel_path(&selected.node_rel)
            .map(|member| member.manifest.clone())
            .ok_or_else(|| unavailable("selected member identity"))
    }
}
