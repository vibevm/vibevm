//! Ordinary final manifest/lock world projected into the canonical neutral wire.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-074#AI-NATIVE-AUTHORITY-BRIDGE");
use crate::{Workspace, WorkspaceError};
use specmap_core::generated::specmap_authority::{Dependency, Input, SpecmapAuthority};
use specmark::spec;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    path::Path,
};
use vibe_core::manifest::{Lockfile, Manifest};
const FILE: &str = ".vibe/specmap-authority.json";
fn fail(root: &Path, error: impl std::fmt::Display) -> WorkspaceError {
    WorkspaceError::Io {
        path: root.join(FILE),
        reason: format!(
            "ordinary boot/install state is already published; specmap authority publication failed: {error}; rerun vibe install --path {}",
            root.display()
        ),
    }
}
fn fingerprint(root: &Path, path: &str) -> Result<Input, WorkspaceError> {
    let safe = vibe_safefs::Project::open(root).map_err(|e| fail(root, e))?;
    let snapshot = safe
        .read_file_snapshot_bounded(path, 4 * 1024 * 1024)
        .map_err(|e| fail(root, e))?;
    Ok(Input {
        root: vibe_core::machine_json_path(root),
        path: path.into(),
        sha256: snapshot.map(|s| Box::new(s.sha256)),
    })
}
/// Verify after lifecycle callbacks; never claim fresh authority over changed inputs.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-074#AI-NATIVE-AUTHORITY-BRIDGE")]
pub fn verify_current(node: &Path) -> Result<(), WorkspaceError> {
    specmap_core::authority::context(node)
        .map_err(|error| fail(node, error))?
        .ok_or_else(|| fail(node, "installed authority context absent"))?;
    Ok(())
}
/// Publish after ordinary durable boot and final manifest/lock publication.
/// Empty/no-op installs also refresh this derived artifact, never init or reads.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-074#AI-NATIVE-AUTHORITY-BRIDGE")]
pub fn publish(
    workspace: &Workspace,
    node: &Path,
    node_roots: &BTreeMap<String, Vec<vibe_core::PackageRef>>,
) -> Result<(), WorkspaceError> {
    let selected = workspace
        .node_rel_of(node)
        .ok_or_else(|| fail(node, "selected node is outside workspace"))?;
    let manifest = workspace
        .iter_nodes()
        .find(|(rel, _)| *rel == selected.as_str())
        .map(|(_, m)| m)
        .ok_or_else(|| fail(node, "selected manifest absent"))?;
    let resolution = crate::install::read_durable_resolution(&workspace.root)?;
    let index: BTreeMap<_, _> = resolution
        .iter()
        .map(|r| ((r.group.to_string(), r.name.clone()), r))
        .collect();
    let roots = node_roots.get(selected.as_str()).ok_or_else(|| {
        fail(
            node,
            "exact planner root projection absent for selected node",
        )
    })?;
    let mut queue: VecDeque<_> = roots
        .iter()
        .filter_map(|p| {
            p.group
                .as_ref()
                .map(|g| (g.to_string(), p.name.to_string()))
        })
        .collect();
    let mut seen = BTreeSet::new();
    let mut dependencies = Vec::new();
    let mut inputs = vec![
        fingerprint(&workspace.root, Manifest::FILENAME)?,
        fingerprint(&workspace.root, Lockfile::FILENAME)?,
        fingerprint(node, "specmap.toml")?,
    ];
    for (rel, _) in workspace.iter_nodes() {
        if rel != "." {
            inputs.push(fingerprint(&workspace.root.join(rel), Manifest::FILENAME)?);
        }
    }
    while let Some(key) = queue.pop_front() {
        if !seen.insert(key.clone()) {
            continue;
        }
        let row = index.get(&key).ok_or_else(|| {
            fail(
                node,
                format!("selected dependency {}/{} missing", key.0, key.1),
            )
        })?;
        inputs.push(fingerprint(&row.content_dir, Manifest::FILENAME)?);
        inputs.push(fingerprint(&row.content_dir, "specmap.toml")?);
        let package_safe =
            vibe_safefs::Project::open(&row.content_dir).map_err(|e| fail(node, e))?;
        let package_config = package_safe
            .read_file_snapshot_bounded("specmap.toml", 4 * 1024 * 1024)
            .map_err(|e| fail(node, e))?
            .map(|s| -> Result<_, WorkspaceError> {
                specmap_core::config::Config::parse_authoring(
                    std::str::from_utf8(&s.bytes).map_err(|e| fail(node, e))?,
                )
                .map_err(|e| fail(node, e))
            })
            .transpose()?;
        let spec_roots = package_config.map(|cfg| cfg.spec_roots).unwrap_or_else(|| {
            vec![vibe_core::machine_json_path(
                &vibe_core::layout::current_specs_root(),
            )]
        });
        let mut admitted = Vec::new();
        for rel in spec_roots {
            vibe_core::manifest::declarant_path(std::path::Path::new(&rel))
                .map_err(|e| fail(node, e.reason()))?;
            admitted.push(vibe_core::machine_json_path(&row.content_dir.join(rel)));
        }
        dependencies.push(Dependency {
            coordinate: format!("{}/{}", row.group, row.name),
            version: row.version.to_string(),
            root: vibe_core::machine_json_path(&row.content_dir),
            specRoots: admitted,
        });
        queue.extend(row.requires.iter().map(|(g, n)| (g.to_string(), n.clone())));
    }
    let safe_node = vibe_safefs::Project::open(node).map_err(|e| fail(node, e))?;
    let config = safe_node
        .read_file_snapshot_bounded("specmap.toml", 4 * 1024 * 1024)
        .map_err(|e| fail(node, e))?
        .map(|s| -> Result<_, WorkspaceError> {
            specmap_core::config::Config::parse_authoring(
                std::str::from_utf8(&s.bytes).map_err(|e| fail(node, e))?,
            )
            .map_err(|e| fail(node, e))
        })
        .transpose()?;
    let value = SpecmapAuthority {
        schema: 1,
        nodeRoots: node_roots
            .iter()
            .map(|(node, roots)| {
                (
                    node.clone(),
                    roots.iter().map(ToString::to_string).collect(),
                )
            })
            .collect(),
        selfNamespace: config
            .map(|c| c.namespace)
            .unwrap_or_else(|| specmap_core::config::Config::default().namespace),
        workspaceRoot: vibe_core::machine_json_path(&workspace.root),
        selectedNode: selected.to_string(),
        selfCoordinate: manifest
            .consumer_node()
            .map(|n| n.coordinate())
            .unwrap_or_default(),
        dependencies,
        inputs,
    };
    // Generated map fields use HashMap. Normalize through the ordered JSON
    // object representation so an unchanged multi-node projection is byte-stable.
    let mut value = serde_json::to_value(value).map_err(|e| fail(node, e))?;
    value.sort_all_objects();
    let bytes = serde_json::to_vec_pretty(&value).map_err(|e| fail(node, e))?;
    let safe = vibe_safefs::Project::open(node).map_err(|e| fail(node, e))?;
    if safe
        .read_file_snapshot_bounded(FILE, 4 * 1024 * 1024)
        .map_err(|e| fail(node, e))?
        .is_some_and(|s| s.bytes == bytes)
    {
        return Ok(());
    }
    // This derived metadata uses ordinary inherited permissions on non-Unix.
    // Exact Unix mode requests remain strict in safefs.
    let mode = if cfg!(unix) { Some(0o600) } else { None };
    safe.write_atomic_with_mode(FILE, &bytes, mode)
        .map_err(|e| fail(node, e))?;
    Ok(())
}
