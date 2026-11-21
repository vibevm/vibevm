//! Queries and replay over an already retained record: selected data, selected
//! parameters, validation against the pinned manifest and the active view.
use super::{PackageConfiguration, PackageConfigurationError};
use crate::manifest::Manifest;
use crate::manifest::configuration::ConfigurationEffect;
use std::collections::{BTreeMap, BTreeSet};

impl PackageConfiguration {
    /// Derive selected public authored data from an admitted historical record.
    /// No parameter, path, environment or secret input is expanded.
    pub fn selected_data(
        &self,
        kind: crate::PackageKind,
    ) -> Result<BTreeMap<String, crate::manifest::ParameterValue>, PackageConfigurationError> {
        let evaluated = self.replay_parts(
            self.forwarded_inputs
                .first()
                .map(|input| input.target.qualified_name())
                .or_else(|| {
                    self.submitted_inputs
                        .values()
                        .next()
                        .map(|input| input.requirement.qualified_name())
                })
                .unwrap_or_else(|| "retained package".into()),
            kind,
        )?;
        if &evaluated != self {
            return Err(PackageConfigurationError::ReplayMismatch {
                package: "retained package".into(),
            });
        }
        let mut values = BTreeMap::new();
        for feature in &self.definition.features {
            if !self.features.contains(&feature.id) {
                continue;
            }
            for effect in &feature.effects {
                if let ConfigurationEffect::ConfigValue { name } = effect {
                    let value = self.definition.config_values.get(name).ok_or_else(|| {
                        PackageConfigurationError::Invalid {
                            package: "retained package".into(),
                            reason: "unknown config data key".into(),
                        }
                    })?;
                    values.insert(name.clone(), value.clone());
                }
            }
        }
        Ok(values)
    }

    /// Selected source members, not an admission of delivery or cache bytes.
    pub fn selected_subskill_paths(&self) -> BTreeSet<String> {
        self.definition
            .features
            .iter()
            .filter(|feature| self.features.contains(&feature.id))
            .flat_map(|feature| &feature.effects)
            .filter_map(|effect| match effect {
                ConfigurationEffect::Subskill { path } => Some(path.clone()),
                _ => None,
            })
            .collect()
    }

    /// Validated public bool projection, separate from selected effect data.
    pub fn selected_parameters(
        &self,
        kind: crate::PackageKind,
    ) -> Result<BTreeMap<String, crate::manifest::ParameterValue>, PackageConfigurationError> {
        self.selected_data(kind)?;
        let mut values = BTreeMap::new();
        for parameter in &self.definition.parameters {
            if !parameter.required && parameter.default.is_none() {
                // Admission/replay above already proved all present values agree.
                if let Some(value) = self
                    .consumers
                    .values()
                    .find_map(|input| input.parameters.get(&parameter.id))
                {
                    values.insert(parameter.id.clone(), value.clone());
                }
                continue;
            }
            let consumer = self
                .consumers
                .values()
                .next()
                .or_else(|| {
                    self.submitted_inputs
                        .values()
                        .next()
                        .map(|input| &input.original)
                })
                .ok_or_else(|| PackageConfigurationError::ReplayMismatch {
                    package: "retained package".into(),
                })?;
            let normalized = self
                .definition
                .normalize_input(consumer, "retained package consumer")
                .map_err(|_| PackageConfigurationError::ReplayMismatch {
                    package: "retained package".into(),
                })?;
            let value = normalized
                .parameters
                .get(&parameter.id)
                .or(parameter.default.as_ref())
                .ok_or_else(|| PackageConfigurationError::ReplayMismatch {
                    package: "retained package".into(),
                })?;
            values.insert(parameter.id.clone(), value.clone());
        }
        Ok(values)
    }

    /// Validate durable derived fields against the retained original request and
    /// the actual pinned manifest; never infer a request from effective features.
    #[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#RECONFIGURATION")]
    pub fn validate(&self, manifest: &Manifest) -> Result<(), PackageConfigurationError> {
        let metadata =
            manifest
                .require_package()
                .map_err(|error| PackageConfigurationError::Invalid {
                    package: "retained package".into(),
                    reason: error.to_string(),
                })?;
        let actual = if self.submitted_inputs.is_empty() {
            Self::evaluate_with_forwarded(manifest, &self.consumers, &self.forwarded_inputs)?
        } else {
            if manifest.configuration.as_ref() != Some(&self.definition)
                || manifest.requires != self.declarations
            {
                return Err(PackageConfigurationError::ReplayMismatch {
                    package: format!("{}/{}", metadata.group, metadata.name),
                });
            }
            self.replay_parts(
                format!("{}/{}", metadata.group, metadata.name),
                metadata.kind,
            )?
        };
        if &actual != self {
            let package = manifest
                .package
                .as_ref()
                .map(|p| format!("{}/{}", p.group, p.name))
                .unwrap_or_default();
            return Err(PackageConfigurationError::ReplayMismatch { package });
        }
        Ok(())
    }

    /// Internal resolver/visibility dependency view; source bytes and the record's
    /// full declarations remain intact. Only active declaration metadata survives.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-069#DORMANT-SOURCE-BOUNDARY"
    )]
    pub fn project(&self, manifest: &Manifest) -> Result<Manifest, PackageConfigurationError> {
        self.validate(manifest)?;
        let active: BTreeSet<_> = self.active_dependencies.iter().cloned().collect();
        let mut view = manifest.clone();
        view.requires
            .packages
            .retain(|p| active.contains(&p.qualified_name()));
        view.requires
            .git_packages
            .retain(|p| active.contains(&format!("{}/{}", p.group, p.name)));
        view.requires
            .path_packages
            .retain(|p| active.contains(&format!("{}/{}", p.group, p.name)));
        view.requires
            .var_packages
            .retain(|p| active.contains(&format!("{}/{}", p.group, p.name)));
        view.requires.links.retain(|key, _| active.contains(key));
        view.requires.accesses.retain(|key, _| active.contains(key));
        view.requires
            .friend_flags
            .retain(|key, _| active.contains(key));
        view.requires.excludes.retain(|key, _| active.contains(key));
        view.requires.optional.clear();
        view.requires.configurations.clear();
        view.configuration = None;
        Ok(view)
    }
}
