//! Original incoming inputs are part of freshness even when versions and
//! computed features are unchanged. No source or default consent is inferred.
use super::*;
use std::collections::BTreeMap;

/// Compare the actual workspace declarations with retained package inputs.
/// Other freshness checks (versions/slots/source posture) remain separate.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#RECONFIGURATION")]
pub fn configuration_freshness(workspace: &Workspace, lock: &Lockfile) -> Freshness {
    if lock.validate_configurations().is_err() {
        return Freshness::Stale("retained configuration provenance is invalid".into());
    }
    let mut current: BTreeMap<
        String,
        BTreeMap<String, Option<vibe_core::manifest::ConfigurationIntentDecl>>,
    > = BTreeMap::new();
    for (rel, manifest) in workspace.iter_nodes() {
        for dependency in &manifest.requires.packages {
            let key = dependency.qualified_name();
            current.entry(key.clone()).or_default().insert(
                format!("{rel}/vibe.toml"),
                manifest.requires.configurations.get(&key).cloned(),
            );
        }
    }
    for (key, consumers) in &current {
        if consumers.values().any(Option::is_some) && !lock.configurations.contains_key(key) {
            return Freshness::Stale(format!(
                "`{key}` original configuration is not recorded in vibe.lock"
            ));
        }
    }
    for (key, record) in &lock.configurations {
        let Some(consumers) = current.get(key) else {
            if record.consumers.is_empty()
                && !record.forwarded_inputs.is_empty()
                && lock.meta.schema_version >= vibe_core::manifest::FORWARDED_INPUT_SCHEMA_VERSION
            {
                continue;
            }
            return Freshness::Stale(format!(
                "`{key}` configuration no longer has its recorded workspace consumers"
            ));
        };
        if !record.submitted_inputs.is_empty() {
            if let Err(reason) = submitted_root_matches(workspace, lock, key, record, consumers) {
                return Freshness::Stale(reason);
            }
            continue;
        }
        let expected = record
            .consumers
            .iter()
            .map(|(name, input)| (name.clone(), Some(input.clone())))
            .collect::<BTreeMap<_, _>>();
        if consumers != &expected {
            return Freshness::Stale(format!("`{key}` original consumer configuration changed"));
        }
    }
    Freshness::Fresh
}

fn submitted_root_matches(
    workspace: &Workspace,
    lock: &Lockfile,
    key: &str,
    record: &vibe_core::manifest::PackageConfiguration,
    consumers: &BTreeMap<String, Option<vibe_core::manifest::ConfigurationIntentDecl>>,
) -> Result<(), String> {
    let bad = || "submitted configuration root/declaration/project/source changed".to_owned();
    if !workspace.members.is_empty()
        || workspace.root_manifest.workspace.is_some()
        || record.submitted_inputs.len() != 1
        || consumers.len() != 1
        || consumers.get("./vibe.toml") != Some(&None)
    {
        return Err(bad());
    }
    let input = record.submitted_inputs.get("./vibe.toml").ok_or_else(bad)?;
    let manifest = &workspace.root_manifest;
    if manifest.requires.packages.len() != 1
        || manifest.requires.packages[0] != input.requirement
        || manifest
            .requires
            .optional
            .get(key)
            .copied()
            .unwrap_or(false)
        || !manifest.requires.configurations.is_empty()
        || !manifest.requires.git_packages.is_empty()
        || !manifest.requires.path_packages.is_empty()
        || !manifest.overrides.is_empty()
        || !manifest.source_policy.is_empty()
        || !manifest.mirrors.is_empty()
    {
        return Err(bad());
    }
    let canonical = workspace.root.canonicalize().map_err(|_| bad())?;
    if canonical != workspace.root {
        return Err(bad());
    }
    let project = vibe_safefs::Project::open(&workspace.root).map_err(|_| bad())?;
    let snapshot = project
        .read_file_snapshot_bounded("vibe.toml", 64 * 1024)
        .map_err(|_| bad())?
        .ok_or_else(bad)?;
    if vibe_core::manifest::submitted_project_witness(&canonical, &snapshot.bytes)?
        != input.project_witness
    {
        return Err(bad());
    }
    let observed = vibe_core::manifest::Manifest::parse_str(
        std::str::from_utf8(&snapshot.bytes).map_err(|_| bad())?,
    )
    .map_err(|_| bad())?;
    if observed != *manifest {
        return Err(bad());
    }
    let row = lock
        .packages
        .iter()
        .find(|row| format!("{}/{}", row.group, row.name) == key)
        .ok_or_else(bad)?;
    if manifest.registries.len() != 1
        || row.registry.as_deref() != Some(manifest.registries[0].name.as_str())
        || !manifest.registries[0].url.starts_with("file://")
    {
        return Err(bad());
    }
    Ok(())
}
