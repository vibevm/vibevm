//! Declaring-node path inputs projected into one workspace acquisition world.
specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#workspace-section");
use crate::error::{Error, Result};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use vibe_core::manifest::Manifest;

/// Resolve every authored path at its declaring node, with a portable locator
/// relative to the workspace root. Conflicting source owners refuse before use.
pub fn workspace_path_sources(
    selected: &Path,
    manifest: &Manifest,
) -> Result<Vec<vibe_registry::ResolvedPathDep>> {
    let workspace = vibe_workspace::Workspace::discover_with_selected_manifest(selected, manifest)?;
    let mut paths = BTreeMap::<String, vibe_registry::ResolvedPathDep>::new();
    let fail = |reason: String| {
        Error::Core(vibe_core::Error::BadDependencyDecl {
            input: "workspace path sources".into(),
            reason,
        })
    };
    for (node, declaration) in workspace.iter_nodes() {
        let declarant = workspace.root.join(node);
        for dep in &declaration.requires.path_packages {
            let package_dir = declarant.join(&dep.path).canonicalize().map_err(|e| {
                fail(format!(
                    "cannot resolve {node} path {}/{}: {e}",
                    dep.group, dep.name
                ))
            })?;
            let common = workspace
                .root
                .ancestors()
                .find(|parent| package_dir.starts_with(parent))
                .ok_or_else(|| fail("path source crosses filesystem roots".into()))?;
            let mut relative = PathBuf::new();
            for _ in workspace
                .root
                .strip_prefix(common)
                .map_err(|e| fail(e.to_string()))?
                .components()
            {
                relative.push("..");
            }
            relative.push(
                package_dir
                    .strip_prefix(common)
                    .map_err(|e| fail(e.to_string()))?,
            );
            let source_manifest = Manifest::read(package_dir.join(Manifest::FILENAME))?;
            let package = source_manifest.require_package()?;
            if package.group != dep.group
                || package.name != dep.name
                || dep.kind.is_some_and(|kind| kind != package.kind)
                || dep
                    .version
                    .as_ref()
                    .is_some_and(|v| !v.matches(&package.version))
            {
                return Err(fail(format!(
                    "{node} path declaration does not match its package identity/kind/version"
                )));
            }
            let key = format!("{}/{}", dep.group, dep.name);
            if let Some(old) = paths.get(&key) {
                if old.package_dir != package_dir {
                    return Err(fail(format!(
                        "conflicting paths for {key}; reconcile declaring nodes"
                    )));
                }
                continue;
            }
            paths.insert(
                key,
                vibe_registry::ResolvedPathDep {
                    kind: dep.kind,
                    group: dep.group.clone(),
                    name: dep.name.clone(),
                    version: dep.version.clone(),
                    package_dir,
                    workspace_rel: vibe_core::machine_json_path(&relative),
                },
            );
        }
    }
    Ok(paths.into_values().collect())
}
