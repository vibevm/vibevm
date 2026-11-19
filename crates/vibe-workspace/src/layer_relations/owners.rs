//! Layer declarations retain the exact owner and selected dependency version.
use super::*;
use std::{collections::VecDeque, path::PathBuf};
use vibe_core::manifest::Manifest;

pub(super) struct Owner {
    pub root: PathBuf,
    pub manifest: Manifest,
    pub coordinate: String,
    pub version: String,
    pub is_root: bool,
    pub dependencies: Vec<(String, String)>,
}
pub(super) fn coordinate(manifest: &Manifest) -> String {
    manifest
        .consumer_node()
        .map(|node| match node.group {
            Some(group) => format!("{group}/{}", node.name),
            None => format!("__host__/{}", node.name),
        })
        .unwrap_or_else(|| "__workspace__".into())
}
pub(super) fn manifest(root: &Path) -> Result<Manifest, String> {
    let safe = vibe_safefs::Project::open(root).map_err(|e| e.to_string())?;
    let bytes = safe
        .read_file_snapshot_bounded(Manifest::FILENAME, 1024 * 1024)
        .map_err(|e| e.to_string())?
        .ok_or("layer manifest absent")?;
    Manifest::parse_str(std::str::from_utf8(&bytes.bytes).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
/// Ordinary durable world checks slot/lock identity; only the selected node's
/// direct roots and actual locked edges enroll owners, never sibling closure.
pub(super) fn collect(root: &Path) -> Result<(PathBuf, Vec<Owner>), String> {
    let node = manifest(root)?;
    let workspace = crate::Workspace::discover(root).map_err(|e| e.to_string())?;
    // A declaration-free leaf grants nothing. Legacy boot/tree observation must
    // not require installation merely to discover that there are no relations.
    if node.layers.is_none() && node.requires.is_empty() {
        return Ok((
            workspace.root,
            vec![Owner {
                root: root.into(),
                coordinate: coordinate(&node),
                version: String::new(),
                manifest: node,
                is_root: true,
                dependencies: Vec::new(),
            }],
        ));
    }
    let mut owners = vec![Owner {
        root: root.into(),
        coordinate: coordinate(&node),
        version: node
            .project
            .as_ref()
            .map(|p| p.version.clone())
            .or_else(|| node.package.as_ref().map(|p| p.version.to_string()))
            .unwrap_or_default(),
        dependencies: {
            let context = specmap_core::authority::context(root)
                .map_err(|e| e.to_string())?
                .ok_or("Vibe node requires ordinary installed authority")?;
            let selected = workspace.node_rel_of(root).ok_or("selected node absent")?;
            context
                .nodeRoots
                .get(selected.as_str())
                .ok_or("exact planner root projection absent")?
                .iter()
                .map(|raw| {
                    let reference = vibe_core::PackageRef::parse(raw).map_err(|e| e.to_string())?;
                    Ok((
                        reference
                            .group
                            .ok_or("unqualified projected root")?
                            .to_string(),
                        reference.name.to_string(),
                    ))
                })
                .collect::<Result<Vec<_>, String>>()?
        },
        manifest: node,
        is_root: true,
    }];
    let resolution =
        crate::install::read_durable_resolution(&workspace.root).map_err(|e| e.to_string())?;
    let index: BTreeMap<_, _> = resolution
        .iter()
        .map(|row| ((row.group.to_string(), row.name.clone()), row))
        .collect();
    let mut queue: VecDeque<_> = owners[0].dependencies.clone().into();
    let mut seen = BTreeSet::new();
    while let Some(key) = queue.pop_front() {
        if !seen.insert(key.clone()) {
            continue;
        }
        let row = index.get(&key).ok_or_else(|| {
            format!(
                "declared dependency {}/{} absent from durable world; install requirements",
                key.0, key.1
            )
        })?;
        owners.push(Owner {
            root: row.content_dir.clone(),
            manifest: row.manifest.clone(),
            coordinate: format!("{}/{}", row.group, row.name),
            version: row.version.to_string(),
            is_root: false,
            dependencies: row
                .requires
                .iter()
                .map(|(g, n)| (g.to_string(), n.clone()))
                .collect(),
        });
        queue.extend(row.requires.iter().map(|(g, n)| (g.to_string(), n.clone())));
    }
    Ok((workspace.root, owners))
}
pub(super) fn selected(
    owner: &Owner,
    owners: &[Owner],
) -> Result<BTreeMap<(String, String), vibe_spec::SelectedPackage>, String> {
    let index: BTreeMap<_, _> = owners.iter().map(|o| (o.coordinate.clone(), o)).collect();
    let mut queue: VecDeque<_> = owner
        .dependencies
        .iter()
        .map(|(g, n)| format!("{g}/{n}"))
        .collect();
    let mut selected = BTreeMap::new();
    while let Some(key) = queue.pop_front() {
        let row = index.get(&key).ok_or_else(|| {
            format!("source dependency `{key}` is outside selected declaration owner")
        })?;
        let package = row.manifest.require_package().map_err(|e| e.to_string())?;
        let coordinate = (package.group.to_string(), package.name.to_string());
        if selected.contains_key(&coordinate) {
            continue;
        }
        selected.insert(
            coordinate,
            vibe_spec::SelectedPackage::new(&row.version, &row.root),
        );
        queue.extend(row.dependencies.iter().map(|(g, n)| format!("{g}/{n}")));
    }
    Ok(selected)
}
