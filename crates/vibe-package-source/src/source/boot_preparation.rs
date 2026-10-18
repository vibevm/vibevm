//! Production producer for the bounded exact local-registry fetch profile.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#PREVIEW-PREPARATION");
use super::*;
use std::{collections::BTreeMap, sync::Arc};
use vibe_orchestrator::machine_operation::boot_debugger::{
    BootDebuggerRequest, BootDebuggerRoute,
    preparation::{
        PreparationPlan, PreparationProvider, PreparationService, PreparationStage, StageEvidence,
        StageKind, StageOutcome,
    },
};
struct LocalBootFetch {
    registry: LocalRegistry,
    selected: Vec<(PackageRef, vibe_registry::CandidateMetadata)>,
}
impl PreparationProvider for LocalBootFetch {
    fn prepare(&self, stage: &PreparationStage, cache: &Path) -> StageOutcome {
        if stage.kind != StageKind::Fetch {
            return StageOutcome::Unavailable {
                reason: format!("no scoped {:?} producer installed", stage.kind),
            };
        }
        let result = (|| -> Result<StageEvidence, String> {
            let (pkg, metadata) = self
                .selected
                .iter()
                .find(|(p, m)| input(p, m) == stage.input)
                .ok_or("unbound selected fetch input")?;
            let resolved = self.registry.resolve(pkg).map_err(|e| e.to_string())?;
            let fetched = self
                .registry
                .fetch_with_catalog(&resolved, &cache.join("sources"), Some(metadata))
                .map_err(|e| e.to_string())?;
            // Verify both the cache and original witness across bounded boot capture.
            verify_capture(&fetched.cache_dir, metadata)?;
            let project =
                vibe_safefs::Project::open(&fetched.cache_dir).map_err(|e| e.to_string())?;
            let mut files = BTreeMap::new();
            let manifest = project
                .read_file_snapshot_bounded("vibe.toml", 1024 * 1024)
                .map_err(|e| e.to_string())?
                .ok_or("missing fetched manifest")?;
            files.insert("vibe.toml".into(), manifest.bytes);
            super::preview::capture_spec_tree(
                &project,
                &fetched.cache_dir,
                &vibe_core::layout::current_specs_root(),
                &mut files,
            )?;
            if let Some(boot) = &metadata.manifest.boot_snippet {
                for path in
                    std::iter::once(&boot.source).chain(boot.fragments.iter().map(|f| &f.source))
                {
                    let path = path.to_string_lossy().replace('\\', "/");
                    let bytes = project
                        .read_file_snapshot_bounded(&path, 16 * 1024 * 1024)
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| format!("missing selected boot source: {path}"))?;
                    files.insert(path, bytes.bytes);
                }
            }
            verify_capture(&fetched.cache_dir, metadata)?;
            if fetched.content_hash != metadata.identity.content {
                return Err("fetched content witness mismatch".into());
            }
            Ok(StageEvidence { files })
        })();
        match result {
            Ok(evidence) => StageOutcome::Captured { evidence },
            Err(reason) => StageOutcome::Failed { reason },
        }
    }
}
fn input(pkg: &PackageRef, m: &vibe_registry::CandidateMetadata) -> String {
    format!(
        "{pkg}; source={}; content={}; definition={}",
        m.identity.source, m.identity.content, m.identity.definition
    )
}
impl InstallResolver {
    /// Host configuration, separate from transport: select exact local candidates,
    /// present their plan, and optionally issue preview-only authority. Remote,
    /// hydration and native providers are intentionally not implied by this profile.
    pub fn bind_boot_preparation(
        &self,
        route: &BootDebuggerRoute,
        capture: &str,
        cache: &Path,
        principal: String,
        ttl_ms: Option<u64>,
    ) -> Result<(Arc<PreparationService>, BootDebuggerRequest), String> {
        let Self::Local(local, _) = self else {
            return Err("unavailable: scoped remote fetch producer".into());
        };
        let host = route.hypothetical_host(capture)?;
        let request = vibe_resolver::resolvo_engine::request::NormalizedRequest::legacy(
            &host.manifest.requires.packages,
            &Default::default(),
        );
        let transaction = self
            .bind_request(request.clone())
            .map_err(|e| e.to_string())?;
        let graph = transaction
            .solve(&request.roots)
            .map_err(|e| e.to_string())?;
        let catalog = transaction
            .catalog_inspection()
            .ok_or("missing selected catalog")?;
        let mut selected = vec![];
        let mut stages = vec![];
        let mut unavailable =
            missing_producers(&host.manifest, &format!("host capture: {capture}"));
        for node in graph.iter() {
            let key = format!("{}/{}@{}", node.group, node.name, node.version);
            let pkg: PackageRef = format!("{}/{}@={}", node.group, node.name, node.version)
                .parse()
                .map_err(|e: vibe_core::Error| e.to_string())?;
            let resolved = local.resolve(&pkg).map_err(|e| e.to_string())?;
            let metadata =
                vibe_registry::CandidateMetadata::read_local_registry(&resolved.source_dir)
                    .map_err(|e| e.to_string())?;
            let expected = catalog
                .candidates
                .iter()
                .find(|(k, _)| k == &key)
                .ok_or("missing candidate")?;
            if metadata.identity != expected.1 {
                return Err("stale selected preparation source".into());
            }
            stages.push(PreparationStage {
                kind: StageKind::Fetch,
                input: input(&pkg, &metadata),
                effects: vec!["write preview cache/sources; retain captured boot evidence".into()],
            });
            unavailable.extend(missing_producers(
                &metadata.manifest,
                &input(&pkg, &metadata),
            ));
            selected.push((pkg, metadata));
        }
        stages.extend(unavailable);
        let service = Arc::new(PreparationService::open(
            route.root(),
            PreparationPlan {
                context: route.context_id().into(),
                capture: capture.into(),
                cache: cache.to_string_lossy().into(),
                stages,
            },
            principal,
        )?);
        if let Some(ttl) = ttl_ms {
            let registry = LocalRegistry::new(local.root()).map_err(|e| e.to_string())?;
            service.admit(Arc::new(LocalBootFetch { registry, selected }), ttl)?;
        }
        let request = route.register_preparation(service.clone())?;
        Ok((service, request))
    }
}

fn verify_capture(
    directory: &Path,
    expected: &vibe_registry::CandidateMetadata,
) -> Result<(), String> {
    let observed =
        vibe_registry::CandidateMetadata::read_local(directory, expected.identity.source.clone())
            .map_err(|e| e.to_string())?;
    if observed.identity != expected.identity {
        return Err("captured content differs from selected catalog".into());
    }
    Ok(())
}

// This only names unsupported producer boundaries. It does not lower skills,
// resolve an owner runtime, or replace the incumbent hydration/native owners.
fn missing_producers(
    manifest: &vibe_core::manifest::Manifest,
    witness: &str,
) -> Vec<PreparationStage> {
    let mut missing = vec![];
    if manifest
        .skills
        .iter()
        .any(|s| s.source.is_some() || !s.resources.is_empty())
    {
        missing.push(PreparationStage {
            kind: StageKind::Hydration,
            input: witness.into(),
            effects: vec![],
        });
    }
    if manifest
        .extensions
        .iter()
        .any(|e| e.handler.kind() == "native")
    {
        missing.push(PreparationStage {
            kind: StageKind::Native,
            input: witness.into(),
            effects: vec![],
        });
    }
    missing
}
