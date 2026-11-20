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
        for package in &self.packages {
            let key = format!("{}/{}", package.group, package.name);
            let record = self.configurations.get(&key);
            let selected = record
                .map(PackageConfiguration::selected_subskill_paths)
                .unwrap_or_default();
            if record.is_some() && selected.len() != package.subskills_active.len() {
                return Err(invalid(format!(
                    "configuration `{key}` selected subskill rows disagree"
                )));
            }
            let mut members = BTreeSet::new();
            for subskill in &package.subskills_active {
                if let Some(witness) = &subskill.source_witness {
                    if self.meta.schema_version < SUBSKILL_WITNESS_SCHEMA_VERSION {
                        return Err(invalid(
                            "configured subskill witness requires schema13".into(),
                        ));
                    }
                    subskill
                        .validate_source_witness(&package.content_hash)
                        .map_err(invalid)?;
                    if !selected.contains(&witness.member.path)
                        || !members.insert(witness.member.path.clone())
                    {
                        return Err(invalid(format!(
                            "configuration `{key}` has an extra or duplicate subskill witness"
                        )));
                    }
                } else if record.is_some() {
                    return Err(invalid(format!(
                        "configuration `{key}` subskill witness is missing"
                    )));
                }
            }
            if members != selected {
                return Err(invalid(format!(
                    "configuration `{key}` selected subskill witness is missing"
                )));
            }
        }
        for (key, record) in &self.configurations {
            if record.closure_fallback_basis.is_some()
                && self.meta.schema_version < CLOSURE_FALLBACK_BASIS_SCHEMA_VERSION
            {
                return Err(invalid(
                    "closure fallback reliance requires schema19".into(),
                ));
            }
            if record
                .automatic_basis
                .as_ref()
                .is_some_and(|basis| !basis.consumer_feature_defaults.is_empty())
                && self.meta.schema_version < FEATURE_DEFAULT_BASIS_SCHEMA_VERSION
            {
                return Err(invalid("feature default reliance requires schema18".into()));
            }
            if record
                .automatic_basis
                .as_ref()
                .is_some_and(|b| !b.consumer_dormant_choices.is_empty())
                && self.meta.schema_version < DORMANT_CHOICE_BASIS_SCHEMA_VERSION
            {
                return Err(invalid("dormant choice reliance requires schema17".into()));
            }
            if record
                .automatic_basis
                .as_ref()
                .is_some_and(|basis| !basis.consumer_choice_sets.is_empty())
                && self.meta.schema_version < MULTI_CHOICE_BASIS_SCHEMA_VERSION
            {
                return Err(invalid(
                    "multi-member default reliance requires schema15".into(),
                ));
            }
            if !record.forwarded_inputs.is_empty()
                && self.meta.schema_version < FORWARDED_INPUT_SCHEMA_VERSION
            {
                return Err(invalid(
                    "candidate-owned forwarding requires schema14".into(),
                ));
            }
            if record.automatic_basis.is_some()
                && self.meta.schema_version < AUTOMATIC_BASIS_SCHEMA_VERSION
            {
                return Err(invalid("automatic package basis requires schema12".into()));
            }
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
            if record.closure_fallback_basis.is_some()
                && (row.kind != PackageKind::Flow
                    || row.registry.is_none()
                    || row.materialization != Materialization::Copy
                    || row.source_kind != Some(SourceKind::Registry)
                    || !row.source_url.as_str().starts_with("file://")
                    || row.overridden
                    || row.via_override.is_some()
                    || row.via_redirect.is_some()
                    || row.template.is_some()
                    || !row.embedded_sources.is_empty()
                    || self.meta.root_dependencies.len() != 1
                    || !self.meta.root_dependencies.iter().any(|root| {
                        root.group.as_ref() == Some(&row.group)
                            && root.name == row.name
                            && root.version.is_exact_pin()
                            && root.version.matches(&row.version)
                    }))
            {
                return Err(invalid(
                    "closure fallback history requires one exact named local copy Flow root".into(),
                ));
            }
            if !record.submitted_inputs.is_empty() {
                if self.meta.schema_version < SUBMITTED_INPUT_SCHEMA_VERSION
                    || row.kind != PackageKind::Flow
                    || row.registry.is_none()
                    || row.materialization.is_in_place()
                    || row.via_override.is_some()
                    || row.via_redirect.is_some()
                    || row.template.is_some()
                    || !row.embedded_sources.is_empty()
                    || !matches!(
                        row.source_kind,
                        None | Some(SourceKind::Registry) | Some(SourceKind::Local)
                    )
                    || !row.source_url.as_str().starts_with("file://")
                {
                    return Err(invalid(
                        "submitted history requires schema16 and a named local Flow source".into(),
                    ));
                }
                for input in record.submitted_inputs.values() {
                    if row.registry.as_deref() != Some(input.expected_source.registry.as_str())
                        || row.content_hash != input.expected_source.content_hash
                        || !input.requirement.version.matches(&row.version)
                        || input.requirement.kind.is_some_and(|kind| kind != row.kind)
                        || !self.meta.root_dependencies.contains(&input.requirement)
                    {
                        return Err(invalid(
                            "submitted history does not match an actual exact locked root".into(),
                        ));
                    }
                }
            }
            let evaluated = record
                .replay_parts(key.clone(), row.kind)
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
        self.validate_forwarded_inputs()?;
        self.validate_forwarded_completeness()?;
        Ok(())
    }

    fn validate_forwarded_inputs(&self) -> Result<()> {
        let invalid = |reason: &str| crate::Error::InvalidLockfile {
            reason: reason.into(),
        };
        for (key, record) in &self.configurations {
            for input in &record.forwarded_inputs {
                let parent = self
                    .find(&input.source_group, input.source_name.as_str())
                    .ok_or_else(|| invalid("forwarded source package is missing"))?;
                let target = self
                    .packages
                    .iter()
                    .find(|row| format!("{}/{}", row.group, row.name) == *key)
                    .ok_or_else(|| invalid("forwarded target package is missing"))?;
                if parent.version != input.source_version
                    || parent.content_hash != input.source_content_hash
                    || input.target.qualified_name() != *key
                    || !input.target.version.is_exact_pin()
                    || !input.target.version.matches(&target.version)
                    || input.target.kind.is_some_and(|kind| kind != target.kind)
                {
                    return Err(invalid("forwarded source/target identity disagrees"));
                }
                let parent_key = format!("{}/{}", parent.group, parent.name);
                let parent_record = self
                    .configurations
                    .get(&parent_key)
                    .ok_or_else(|| invalid("forwarded source configuration is missing"))?;
                if parent_record.consumers.is_empty()
                    || !parent_record.forwarded_inputs.is_empty()
                    || !parent_record.features.contains(&input.source_feature)
                    || !parent_record.active_dependencies.contains(key)
                    || parent_record
                        .declarations
                        .packages
                        .get(input.declaration_index)
                        != Some(&input.target)
                    || !parent
                        .dependencies
                        .iter()
                        .any(|edge| edge.qualified_name() == *key)
                {
                    return Err(invalid(
                        "forwarded source declaration or selected feature disagrees",
                    ));
                }
                let active_effect=parent_record.definition.features.iter().filter(|feature|feature.id==input.source_feature)
                    .flat_map(|feature| &feature.effects).any(|effect|matches!(effect,
                        crate::manifest::ConfigurationEffect::Forward {target,feature,strength}
                        if crate::PackageRef::parse(target).is_ok_and(|reference|reference.qualified_name()==*key)
                            && feature==&input.target_feature && strength==&input.strength));
                if !active_effect {
                    return Err(invalid("forwarded request has no active source effect"));
                }
                if self.packages.iter().any(|row| {
                    row != parent
                        && row
                            .dependencies
                            .iter()
                            .any(|edge| edge.qualified_name() == *key)
                }) {
                    return Err(invalid(
                        "forwarded target has an unaccounted package consumer",
                    ));
                }
            }
        }
        Ok(())
    }

    fn validate_forwarded_completeness(&self) -> Result<()> {
        let invalid = |reason: &str| crate::Error::InvalidLockfile {
            reason: reason.into(),
        };
        for (parent_key, record) in &self.configurations {
            let parent = self
                .packages
                .iter()
                .find(|row| format!("{}/{}", row.group, row.name) == *parent_key)
                .ok_or_else(|| invalid("forwarded source row missing"))?;
            for source in record
                .definition
                .features
                .iter()
                .filter(|feature| record.features.contains(&feature.id))
            {
                for effect in &source.effects {
                    let crate::manifest::ConfigurationEffect::Forward {
                        target,
                        feature,
                        strength,
                    } = effect
                    else {
                        continue;
                    };
                    let target_key = crate::PackageRef::parse(target)
                        .map_err(|_| invalid("invalid forwarded target"))?
                        .qualified_name();
                    if *strength == crate::manifest::ForwardStrength::Weak
                        && !record.active_dependencies.contains(&target_key)
                    {
                        // The incoming-direction check rejects any forged
                        // contribution; completeness must not require a dormant
                        // target to exist merely to prove its absence.
                        continue;
                    }
                    let target_record = self
                        .configurations
                        .get(&target_key)
                        .ok_or_else(|| invalid("active forward target record missing"))?;
                    let matches = target_record
                        .forwarded_inputs
                        .iter()
                        .filter(|input| {
                            input.source_group == parent.group
                                && input.source_name == parent.name
                                && input.source_version == parent.version
                                && input.source_content_hash == parent.content_hash
                                && input.source_feature == source.id
                                && input.target.qualified_name() == target_key
                                && input.target_feature == *feature
                                && input.strength == *strength
                        })
                        .count();
                    if matches != 1 {
                        return Err(invalid(
                            "active forward requires exactly one retained incoming request",
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    pub(crate) fn for_write(&self) -> Result<Self> {
        // Stage A deliberately has no public writer for submitted history.
        if self
            .configurations
            .values()
            .any(|record| !record.submitted_inputs.is_empty())
        {
            return Err(crate::Error::InvalidLockfile {
                reason: "submitted history writer is disabled pending installation admission"
                    .into(),
            });
        }
        if !matches!(
            self.meta.schema_version,
            LEGACY_SCHEMA_VERSION
                | CONFIGURATION_SCHEMA_VERSION
                | REMAP_SOURCE_SCHEMA_VERSION
                | TEMPLATE_SCHEMA_VERSION
                | DIGESTLESS_TEMPLATE_SCHEMA_VERSION
                | AUTOMATIC_BASIS_SCHEMA_VERSION
                | SUBSKILL_WITNESS_SCHEMA_VERSION
                | FORWARDED_INPUT_SCHEMA_VERSION
                | MULTI_CHOICE_BASIS_SCHEMA_VERSION
                | SUBMITTED_INPUT_SCHEMA_VERSION
                | DORMANT_CHOICE_BASIS_SCHEMA_VERSION
                | FEATURE_DEFAULT_BASIS_SCHEMA_VERSION
                | CLOSURE_FALLBACK_BASIS_SCHEMA_VERSION
        ) {
            return Err(crate::Error::UnsupportedLockfile {
                found: self.meta.schema_version,
                expected: CURRENT_SCHEMA_VERSION,
            });
        }
        let mut value = self.clone();
        // The lowest epoch that carries what this lock actually holds, the
        // rule the 7 → 8 → 9 steps already followed: a project with no
        // template package and no channel requirement writes exactly the
        // bytes it wrote before PROP-075 (AV-FMT-002).
        value.meta.schema_version = if value
            .configurations
            .values()
            .any(|record| record.closure_fallback_basis.is_some())
        {
            CLOSURE_FALLBACK_BASIS_SCHEMA_VERSION
        } else if value.configurations.values().any(|record| {
            record
                .automatic_basis
                .as_ref()
                .is_some_and(|basis| !basis.consumer_feature_defaults.is_empty())
        }) {
            FEATURE_DEFAULT_BASIS_SCHEMA_VERSION
        } else if value.configurations.values().any(|record| {
            record
                .automatic_basis
                .as_ref()
                .is_some_and(|b| !b.consumer_dormant_choices.is_empty())
        }) {
            DORMANT_CHOICE_BASIS_SCHEMA_VERSION
        } else if value.configurations.values().any(|record| {
            record
                .automatic_basis
                .as_ref()
                .is_some_and(|basis| !basis.consumer_choice_sets.is_empty())
        }) {
            MULTI_CHOICE_BASIS_SCHEMA_VERSION
        } else if value
            .configurations
            .values()
            .any(|record| !record.forwarded_inputs.is_empty())
        {
            FORWARDED_INPUT_SCHEMA_VERSION
        } else if value.packages.iter().any(|package| {
            package
                .subskills_active
                .iter()
                .any(|subskill| subskill.source_witness.is_some())
        }) {
            SUBSKILL_WITNESS_SCHEMA_VERSION
        } else if value
            .configurations
            .values()
            .any(|record| record.automatic_basis.is_some())
        {
            AUTOMATIC_BASIS_SCHEMA_VERSION
        } else if value.holds_digestless_template_records() {
            DIGESTLESS_TEMPLATE_SCHEMA_VERSION
        } else if value.holds_template_records() {
            TEMPLATE_SCHEMA_VERSION
        } else if value.remaps.iter().any(|row| row.source.is_some()) {
            REMAP_SOURCE_SCHEMA_VERSION
        } else if value.configurations.is_empty() {
            LEGACY_SCHEMA_VERSION
        } else {
            CONFIGURATION_SCHEMA_VERSION
        };
        value.validate_remap_sources()?;
        value.validate_template_records()?;
        value.validate_configurations()?;
        Ok(value)
    }

    /// Whether any row carries a fact only schema 10 can express.
    pub(crate) fn holds_template_records(&self) -> bool {
        self.packages
            .iter()
            .any(|row| row.template.is_some() || row.via_channel.is_some())
    }

    /// Whether any row carries a template record with **no** digest
    /// document — the one fact schema 10 cannot express, because its
    /// `digest_source_sha256` is a required field (PROP-075
    /// `#TEMPLATE-NO-DIGEST`, AV-FMT-033).
    pub(crate) fn holds_digestless_template_records(&self) -> bool {
        self.packages.iter().any(|row| {
            row.template
                .as_ref()
                .is_some_and(|template| template.digest_source_sha256.is_none())
        })
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
