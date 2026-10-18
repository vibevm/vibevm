//! Explicit source capture and binding for the shared debugger route.
//! Resolution uses the ordinary admitted InstallSource transaction. Neither
//! this capture nor subsequent evaluation installs or fetches a package.
use super::*;
use std::collections::BTreeMap;
use vibe_orchestrator::machine_operation::boot_debugger::{
    BootDebuggerRoute,
    hypothetical::{FrozenWorld, HostCapture, PreparedScenario, SelectedPackage},
};
use vibe_resolver::resolvo_engine::request::NormalizedRequest;

impl InstallResolver {
    /// Capture available local inputs, preserving the incumbent selected
    /// catalog and route. Remote content stays a named missing fetch stage.
    /// This is explicit preparation, never an inspect/preview side effect.
    pub fn capture_boot_world(
        &self,
        request: NormalizedRequest,
        host: HostCapture,
    ) -> Result<FrozenWorld, String> {
        if request.roots != host.manifest.requires.packages {
            return Err("normalized roots differ from captured host requires".into());
        }
        let transaction = self
            .bind_request(request.clone())
            .map_err(|e| e.to_string())?;
        let graph =
            vibe_install::resolve_boot_comparison(&transaction, &request.roots, &host.manifest)
                .map_err(|e| e.to_string())?;
        let catalog = transaction
            .catalog_inspection()
            .ok_or("missing transaction catalog")?;
        let route = transaction
            .route_resolution()
            .ok_or("missing selected route")?;
        let admitted = transaction
            .normalized_request()
            .ok_or("missing admitted normalized request")?;
        let mut packages = vec![];
        let mut missing = vec![];
        for (coordinate, effects) in &route.effects {
            if !effects.active_subskills.is_empty() || !effects.config_values.is_empty() {
                missing.push(format!(
                    "effective subskill/configuration source preparation: {coordinate}"
                ));
            }
        }
        for node in graph.iter() {
            let key = format!("{}/{}@{}", node.group, node.name, node.version);
            let manifest = route
                .manifests
                .get(&key)
                .ok_or_else(|| format!("missing selected manifest: {key}"))?
                .clone();
            if !manifest.conditional_deps.is_empty() {
                missing.push(format!("conditional resolution projection: {key}"));
            }
            let identity = &catalog
                .candidates
                .iter()
                .find(|(k, _)| k == &key)
                .ok_or_else(|| format!("missing candidate identity: {key}"))?
                .1;
            let roots: Vec<_> = match self {
                Self::Local(local, _) => vec![local.root()],
                Self::Embedded { locals, .. } => locals.iter().map(LocalRegistry::root).collect(),
                Self::Multi(..) => vec![],
            };
            let mut files = BTreeMap::new();
            let mut captured = false;
            for root in roots {
                let directory = root
                    .join(node.group.as_str())
                    .join(node.name.as_str())
                    .join(format!("v{}", node.version));
                if !directory.is_dir() {
                    continue;
                }
                let before = vibe_registry::CandidateMetadata::read_local_registry(&directory)
                    .map_err(|e| e.to_string())?;
                if &before.identity != identity {
                    if before.identity.source == identity.source {
                        return Err(format!("stale catalog/source capture: {key}"));
                    }
                    continue;
                }
                // Effective configuration can change boot/subskill loading;
                // only the exact source declaration may supply this capture.
                let mut source_declaration = before.manifest.clone();
                // A selected dependency set is resolver metadata, not source
                // derivation. All other authored source/owner fields must match.
                source_declaration.requires.packages = manifest.requires.packages.clone();
                if source_declaration != manifest {
                    missing.push(format!("configured source/owner preparation: {key}"));
                    captured = true;
                    break;
                }
                let project = vibe_safefs::Project::open(&directory).map_err(|e| e.to_string())?;
                if manifest
                    .package
                    .as_ref()
                    .is_some_and(|p| p.format.is_normal())
                {
                    capture_spec_tree(
                        &project,
                        &directory,
                        &vibe_core::layout::current_specs_root(),
                        &mut files,
                    )?;
                }
                if let Some(snippet) = &manifest.boot_snippet {
                    for path in std::iter::once(&snippet.source)
                        .chain(snippet.fragments.iter().map(|f| &f.source))
                    {
                        let mut paths = vec![path.clone()];
                        if path.extension().is_some_and(|e| e == "md" || e == "xml") {
                            paths.push(path.with_extension(
                                if path.extension().is_some_and(|e| e == "md") {
                                    "xml"
                                } else {
                                    "md"
                                },
                            ));
                        }
                        for path in paths {
                            if let Some(snapshot) = project
                                .read_file_snapshot_bounded(
                                    &path.to_string_lossy().replace('\\', "/"),
                                    64 * 1024 * 1024,
                                )
                                .map_err(|e| e.to_string())?
                            {
                                files.insert(
                                    path.to_string_lossy().replace('\\', "/"),
                                    snapshot.bytes,
                                );
                            }
                        }
                    }
                }
                let after = vibe_registry::CandidateMetadata::read_local_registry(&directory)
                    .map_err(|e| e.to_string())?;
                if &after.identity != identity {
                    return Err(format!("stale catalog/source capture: {key}"));
                }
                captured = true;
                break;
            }
            if !captured {
                missing.push(format!("fetch/source capture: {key} ({})", identity.source));
            }
            packages.push(SelectedPackage {
                manifest,
                requires: node
                    .dependencies
                    .iter()
                    .filter_map(|p| p.group.clone().map(|g| (g, p.name.to_string())))
                    .collect(),
                files,
                source_identity: format!("{identity:?}"),
            });
        }
        FrozenWorld::from_resolution(
            host,
            packages,
            catalog.digest.clone(),
            format!("{admitted:?}; {route:?}; {:?}", self.cell_selection()),
            missing,
        )
    }

    /// The standard source composition binds both prepared worlds into the
    /// existing host route. The returned strict request selects immutable
    /// evidence; dispatch performs builtin compilation only in memory.
    pub fn bind_boot_preview(
        &self,
        route: &BootDebuggerRoute,
        baseline: &str,
        candidate: &str,
        baseline_request: NormalizedRequest,
        candidate_request: NormalizedRequest,
        target: String,
    ) -> Result<vibe_orchestrator::machine_operation::boot_debugger::BootDebuggerRequest, String>
    {
        let baseline =
            self.capture_boot_world(baseline_request, route.hypothetical_host(baseline)?)?;
        let candidate =
            self.capture_boot_world(candidate_request, route.hypothetical_host(candidate)?)?;
        route.retain_hypothetical(PreparedScenario::new(baseline, candidate, target)?)
    }
}

pub(super) fn capture_spec_tree(
    project: &vibe_safefs::Project,
    directory: &Path,
    relative: &Path,
    files: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(), String> {
    let path = directory.join(relative);
    match std::fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.to_string()),
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err(format!(
                "source capture refuses symlink: {}",
                relative.display()
            ));
        }
        Ok(meta) if !meta.is_dir() => {
            return Err(format!(
                "source inventory is not a directory: {}",
                relative.display()
            ));
        }
        _ => {}
    }
    for entry in std::fs::read_dir(path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        let relative = relative.join(entry.file_name());
        if kind.is_symlink() {
            return Err(format!(
                "source capture refuses symlink: {}",
                relative.display()
            ));
        }
        if kind.is_dir() {
            capture_spec_tree(project, directory, &relative, files)?;
        } else if kind.is_file()
            && relative
                .extension()
                .is_some_and(|e| e == "md" || e == "xml")
        {
            let path = relative.to_string_lossy().replace('\\', "/");
            let snapshot = project
                .read_file_snapshot_bounded(&path, 64 * 1024 * 1024)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("source changed during capture: {path}"))?;
            files.insert(path, snapshot.bytes);
        }
    }
    Ok(())
}
