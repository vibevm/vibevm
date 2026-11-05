//! Original incoming inputs are part of freshness even when versions and
//! computed features are unchanged. No source or default consent is inferred.
use super::*;
use std::collections::BTreeMap;

/// Compare the actual workspace declarations with retained package inputs.
/// Other freshness checks (versions/slots/source posture) remain separate.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#RECONFIGURATION")]
pub fn configuration_freshness(workspace: &Workspace, lock: &Lockfile) -> Freshness {
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
            return Freshness::Stale(format!(
                "`{key}` configuration no longer has its recorded workspace consumers"
            ));
        };
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
