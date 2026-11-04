//! Test-only original package expansion/validation at effb460e4.
//! Historical defects are characterized separately, not compatibility promises.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#LEGACY-PACKAGE-COMPATIBILITY");

use super::{FeatureError, FeatureExpansion, FeatureRequest, FeatureValue};
use specmark::spec;
use std::collections::BTreeSet;
use vibe_core::manifest::FeaturesTable;

pub(super) fn expand_features(
    table: &FeaturesTable,
    request: &FeatureRequest,
) -> Result<FeatureExpansion, FeatureError> {
    // Build the seed set. The `default` feature, when present, is
    // itself a real feature — its activation list expands during BFS
    // through the normal feature-feature edge.
    let mut seed: BTreeSet<String> = BTreeSet::new();
    if request.all {
        for k in table.features.keys() {
            if !k.starts_with('_') {
                seed.insert(k.clone());
            }
        }
    } else {
        if !request.no_defaults && table.features.contains_key("default") {
            seed.insert("default".to_string());
        }
        for f in &request.explicit {
            if f.starts_with('_') {
                return Err(FeatureError::PrivateFeature(f.clone()));
            }
            if !table.features.contains_key(f) {
                return Err(FeatureError::UnknownFeature(f.clone()));
            }
            seed.insert(f.clone());
        }
    }

    let mut out = FeatureExpansion::default();
    // BFS / DFS expansion. Track feature names we've already processed
    // so cycles terminate.
    let mut work: Vec<String> = seed.into_iter().collect();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    while let Some(f) = work.pop() {
        if !seen.insert(f.clone()) {
            continue;
        }
        if !table.features.contains_key(&f) {
            return Err(FeatureError::UnknownFeature(f));
        }
        out.active_features.insert(f.clone());
        let activations = table.get(&f).unwrap_or(&[]);
        for raw in activations {
            let val = FeatureValue::parse(raw).map_err(|e| match e {
                FeatureError::Malformed(_) => {
                    FeatureError::Malformed(format!("feature `{f}` activation `{raw}`",))
                }
                other => other,
            })?;
            match val {
                FeatureValue::Feature(name) => {
                    if !table.features.contains_key(&name) {
                        return Err(FeatureError::UnknownFeature(format!(
                            "{name} (referenced by feature `{f}`)"
                        )));
                    }
                    work.push(name);
                }
                FeatureValue::Dep { dep_name } => {
                    out.active_deps.insert(dep_name);
                }
                FeatureValue::DepFeature {
                    dep_name,
                    dep_feature,
                    weak,
                } => {
                    let target = if weak {
                        &mut out.weak_dep_features
                    } else {
                        &mut out.dep_features
                    };
                    target
                        .entry(dep_name.clone())
                        .or_default()
                        .insert(dep_feature);
                    if !weak {
                        out.active_deps.insert(dep_name);
                    }
                }
                FeatureValue::Subskill { path } => {
                    out.active_subskills.insert(path);
                }
            }
        }
    }

    // Exclusive-group check.
    for (group, members) in &table.exclusive {
        let active_in_group: Vec<&String> = members
            .iter()
            .filter(|m| out.active_features.contains(m.as_str()))
            .collect();
        if active_in_group.len() > 1 {
            return Err(FeatureError::ExclusiveViolation {
                group: group.clone(),
                active: active_in_group.iter().map(|s| (*s).clone()).collect(),
            });
        }
    }

    Ok(out)
}

/// Static structural validation of a `[features]` table — runs at
/// `vibe check` time. Returns a list of diagnostics
/// (empty = valid).
#[spec(informs = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#features")]
pub(super) fn validate_features_table(table: &FeaturesTable) -> Vec<String> {
    let mut findings = Vec::new();
    // Cycle detection by attempting an `--all-features` expansion.
    let probe = FeatureRequest {
        explicit: Vec::new(),
        no_defaults: true,
        all: true,
    };
    if let Err(e) = expand_features(table, &probe) {
        // Only feature-graph problems should fan out here; surface
        // them as findings rather than aborting the check pass.
        findings.push(format!("features table: {e}"));
    }
    // Default activation must respect exclusive groups.
    let defaults_probe = FeatureRequest {
        explicit: Vec::new(),
        no_defaults: false,
        all: false,
    };
    if let Err(e) = expand_features(table, &defaults_probe)
        && !matches!(e, FeatureError::UnknownFeature(_))
    {
        findings.push(format!("default features: {e}"));
    }
    // Subskill references look syntactically valid.
    for (name, activations) in &table.features {
        for raw in activations {
            if let Err(e) = FeatureValue::parse(raw) {
                findings.push(format!("feature `{name}` activation `{raw}`: {e}"));
            }
        }
    }
    findings
}
