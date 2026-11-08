//! Rich package lock admission and lossless legacy serialization.
use super::*;
use crate::manifest::PackageConfiguration;
use std::collections::BTreeSet;

impl Lockfile {
    /// Strict structural/semantic admission of present configuration records.
    /// Whether a record is missing for a source-defined interface additionally
    /// requires the actual pinned manifest, checked by the durable-world owner.
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#PERSISTENCE")]
    pub fn validate_configurations(&self) -> Result<()> {
        let invalid = |reason| crate::Error::InvalidLockfile { reason };
        if self.meta.schema_version == LEGACY_SCHEMA_VERSION && !self.configurations.is_empty() {
            return Err(invalid(
                "schema7 cannot carry rich package configuration".into(),
            ));
        }
        for (key, record) in &self.configurations {
            let rows: Vec<_> = self
                .packages
                .iter()
                .filter(|p| format!("{}/{}", p.group, p.name) == *key)
                .collect();
            if rows.len() != 1 {
                return Err(invalid(format!(
                    "configuration `{key}` must name exactly one locked package"
                )));
            }
            let row = rows[0];
            let evaluated = PackageConfiguration::evaluate_parts(
                key.clone(),
                row.kind,
                &record.definition,
                &record.declarations,
                &record.consumers,
            )
            .map_err(|error| invalid(error.to_string()))?;
            if &evaluated != record {
                return Err(invalid(format!(
                    "configuration `{key}` derived state disagrees with original inputs"
                )));
            }
            let features: BTreeSet<_> = row.features.iter().collect();
            let selected: BTreeSet<_> = record.features.iter().collect();
            let edges: BTreeSet<_> = row
                .dependencies
                .iter()
                .map(|p| p.qualified_name())
                .collect();
            let active: BTreeSet<_> = record.active_dependencies.iter().cloned().collect();
            if features != selected || edges != active || row.dependencies.len() != edges.len() {
                return Err(invalid(format!(
                    "configuration `{key}` disagrees with locked features/dependency graph"
                )));
            }
            for edge in &row.dependencies {
                let Some(group) = &edge.group else {
                    return Err(invalid(format!(
                        "configuration `{key}` has unqualified selected dependency"
                    )));
                };
                let Some(target) = self.find(group, edge.name.as_str()) else {
                    return Err(invalid(format!(
                        "configuration `{key}` dependency `{edge}` has no locked target"
                    )));
                };
                if edge.version != target.as_package_ref()?.version
                    || edge.kind.is_some_and(|kind| kind != target.kind)
                {
                    return Err(invalid(format!(
                        "configuration `{key}` dependency `{edge}` must exactly pin its locked target"
                    )));
                }
                let declaration=record.declarations.packages.iter().find(|p|p.group.as_ref()==Some(group) && p.name==edge.name)
                    .ok_or_else(|| invalid(format!("configuration `{key}` selected edge `{edge}` has no registry declaration")))?;
                if !declaration.version.matches(&target.version)
                    || declaration.kind.is_some_and(|kind| kind != target.kind)
                {
                    return Err(invalid(format!(
                        "configuration `{key}` selected target `{edge}` violates its original version/kind declaration"
                    )));
                }
                for feature in record
                    .definition
                    .features
                    .iter()
                    .filter(|feature| selected.contains(&feature.id))
                {
                    for effect in &feature.effects {
                        let reference = match effect {
                            crate::manifest::ConfigurationEffect::Dependency { target }
                            | crate::manifest::ConfigurationEffect::Forward { target, .. } => {
                                target
                            }
                            _ => continue,
                        };
                        let (asserted, group, name) =
                            crate::manifest::package::parse_pkgref_key(reference)
                                .map_err(|error| invalid(error.to_string()))?;
                        if group == target.group
                            && name == target.name.as_str()
                            && asserted.is_some_and(|kind| kind != target.kind)
                        {
                            return Err(invalid(format!(
                                "configuration `{key}` selected effect kind `{reference}` from feature `{}` disagrees with target kind `{}`",
                                feature.id, target.kind
                            )));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub(crate) fn for_write(&self) -> Result<Self> {
        if !matches!(
            self.meta.schema_version,
            LEGACY_SCHEMA_VERSION | CONFIGURATION_SCHEMA_VERSION | CURRENT_SCHEMA_VERSION
        ) {
            return Err(crate::Error::UnsupportedLockfile {
                found: self.meta.schema_version,
                expected: CURRENT_SCHEMA_VERSION,
            });
        }
        let mut value = self.clone();
        value.meta.schema_version = if value.remaps.iter().any(|row| row.source.is_some()) {
            CURRENT_SCHEMA_VERSION
        } else if value.configurations.is_empty() {
            LEGACY_SCHEMA_VERSION
        } else {
            CONFIGURATION_SCHEMA_VERSION
        };
        value.validate_remap_sources()?;
        value.validate_configurations()?;
        Ok(value)
    }
}

impl Lockfile {
    /// Re-admit a named locked package against its actual pinned source. A
    /// configured source with a missing record is corruption, never legacy input.
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#RECONFIGURATION")]
    pub fn configuration_for(
        &self,
        package: &LockedPackage,
        manifest: &super::super::Manifest,
    ) -> Result<Option<PackageConfiguration>> {
        let key = format!("{}/{}", package.group, package.name);
        if manifest.package.as_ref().is_none_or(|actual| {
            actual.group != package.group
                || actual.name != package.name
                || actual.version != package.version
                || actual.kind != package.kind
                || actual.materialization != package.materialization
        }) {
            return Err(crate::Error::InvalidLockfile {
                reason: format!(
                    "retained `{key}` manifest identity/materialization disagrees with its locked row"
                ),
            });
        }
        match self.configurations.get(&key) {
            Some(record) => {
                record
                    .validate(manifest)
                    .map_err(|error| crate::Error::InvalidLockfile {
                        reason: error.to_string(),
                    })?;
                Ok(Some(record.clone()))
            }
            None if manifest.configuration.is_some()
                || manifest.requires.optional.values().any(|v| *v)
                || !manifest.requires.configurations.is_empty() =>
            {
                Err(crate::Error::InvalidLockfile {
                    reason: format!(
                        "package `{key}` requires retained configuration but has no lock record"
                    ),
                })
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests;
