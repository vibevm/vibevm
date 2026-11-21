//! Dependency activation from the resolved feature set: mandatory declarations
//! first, then selected dependency effects, then forwards over that fixed base.
use super::{PackageConfigurationError, Scope};
use crate::configuration::FeatureResolution;
use crate::manifest::configuration::{ConfigurationEffect, ForwardStrength};
use std::collections::{BTreeMap, BTreeSet};

/// Active dependency keys and the selected features that caused each of them.
pub(super) struct Activation {
    pub(super) active: BTreeSet<String>,
    /// Empty causes mean an ordinary mandatory declaration.
    pub(super) causes: BTreeMap<String, Vec<String>>,
}

impl Activation {
    fn record(&mut self, key: String, feature: &str) {
        self.active.insert(key.clone());
        let entries = self.causes.entry(key).or_default();
        if !entries.iter().any(|entry| entry == feature) {
            entries.push(feature.to_owned());
        }
    }
}

pub(super) fn activate(
    scope: &Scope<'_>,
    features: &FeatureResolution,
) -> Result<Activation, PackageConfigurationError> {
    let Scope {
        definition,
        requires,
        forwarded,
        ..
    } = *scope;
    let active = requires
        .iter_pkgrefs()
        .filter_map(|(group, name)| group.map(|g| format!("{g}/{name}")))
        .filter(|key| !requires.optional.get(key).copied().unwrap_or(false))
        .collect::<BTreeSet<_>>();
    let causes = active
        .iter()
        .map(|key| (key.clone(), Vec::new()))
        .collect::<BTreeMap<_, _>>();
    let mut activation = Activation { active, causes };
    // Independent activation is complete before any forward is considered.
    // Weak requests cannot activate themselves, even indirectly through
    // declaration order or another package's workspace-root presence.
    for feature in &definition.features {
        if !features.selected.contains(&feature.id) {
            continue;
        }
        for effect in &feature.effects {
            if let ConfigurationEffect::Dependency { target } = effect {
                let (_, group, name) = crate::manifest::package::parse_pkgref_key(target)
                    .map_err(|e| scope.invalid(e.to_string()))?;
                activation.record(format!("{group}/{name}"), &feature.id);
            }
        }
    }
    let independently_active = activation.active.clone();
    for feature in &definition.features {
        if !features.selected.contains(&feature.id) {
            continue;
        }
        for effect in &feature.effects {
            match effect {
                ConfigurationEffect::Dependency { .. } => {}
                ConfigurationEffect::Forward {
                    target, strength, ..
                } => {
                    let declaration = requires
                        .packages
                        .iter()
                        .find(|edge| edge.qualified_name() == *target);
                    if !forwarded.is_empty()
                        || declaration.is_none_or(|edge| !edge.version.is_exact_pin())
                    {
                        return Err(scope.unsupported(
                            "selected cross-package forwarding requires candidate-conditioned incoming consumers; only exact compulsory strong edges are connected",
                        ));
                    }
                    let key = declaration
                        .ok_or_else(|| {
                            scope.invalid("forwarded dependency declaration is missing".into())
                        })?
                        .qualified_name();
                    if *strength == ForwardStrength::Weak && !independently_active.contains(&key) {
                        continue;
                    }
                    activation.record(key, &feature.id);
                }
                ConfigurationEffect::Subskill { path } => {
                    if !path.starts_with("subskills/") || path.rsplit_once('/').is_none() {
                        return Err(scope.unsupported(
                            "selected subskill loading is not connected for bare files; declaration-backed subskills/ member required",
                        ));
                    }
                    // This is selection only. Source admission supplies the
                    // independent declaration/member witness before apply.
                }
                ConfigurationEffect::ConfigValue { name } => {
                    if !definition.config_values.contains_key(name) {
                        return Err(scope.invalid("selected config value is not declared".into()));
                    }
                }
            }
        }
    }
    let registry: BTreeSet<_> = requires
        .packages
        .iter()
        .map(|p| p.qualified_name())
        .collect();
    if activation.active.iter().any(|key| !registry.contains(key)) {
        return Err(scope.unsupported("an active declaration uses a non-registry source"));
    }
    if requires
        .configurations
        .keys()
        .any(|key| activation.active.contains(key))
    {
        return Err(scope.unsupported(
            "active transitive configuration requires candidate-conditioned incoming consumers",
        ));
    }
    Ok(activation)
}
