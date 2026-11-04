//! Validation of the feature tree, profiles, parameters and license of an
//! installer option (`PROP-065#feature-tree`, `#profiles`, `#parameters`,
//! `#license`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#feature-tree");

use std::collections::{BTreeMap, BTreeSet};

use crate::manifest::{FieldError, FieldPath, declarant_path, is_portable_token};

use super::step_validation::Names;
use super::*;

impl InstallerOptionDecl {
    /// Check the wizard declarations and return the ids steps may name.
    pub(super) fn validate_wizard(&self, at: &FieldPath) -> Result<Names<'_>, FieldError> {
        if self.configuration.is_some()
            && (!self.features.is_empty()
                || !self.profiles.is_empty()
                || !self.parameters.is_empty())
        {
            return Err(FieldError::new(
                at.table("configuration"),
                "common configuration conflicts with legacy features/profiles/parameters in this installer scope",
            ));
        }
        let features = if let Some(configuration) = &self.configuration {
            configuration.compile_choices_at(&at.table("configuration"))?;
            configuration
                .features
                .iter()
                .map(|f| f.id.as_str())
                .collect()
        } else {
            let features = self.validate_features(at)?;
            self.validate_profiles(&features, at)?;
            features
        };
        let mut params: BTreeSet<&str> = self
            .configuration
            .as_ref()
            .into_iter()
            .flat_map(|c| c.parameters.iter().map(|p| p.id.as_str()))
            .collect();
        for (index, parameter) in self.parameters.iter().enumerate() {
            let table = format!("[[installer_options.parameters]] `{}`", parameter.id);
            let row = at.table("parameters").index(index);
            if !is_portable_token(&parameter.id) {
                return Err(FieldError::new(
                    row.field("id"),
                    format!("{table} id must be a portable lowercase token"),
                ));
            }
            if !params.insert(parameter.id.as_str()) {
                return Err(FieldError::new(
                    row.field("id"),
                    format!("{table} is duplicated in installer option `{}`", self.id),
                ));
            }
            validate_parameter(parameter, &table, &row)?;
        }
        if let Some(license) = &self.license {
            let license_at = at.table("license");
            declarant_path(&license.file).map_err(|fault| {
                FieldError::new(
                    license_at.field("file"),
                    format!(
                        "[installer_options.license].file must be a package-relative file: {}",
                        fault.reason()
                    ),
                )
            })?;
            if license.name.trim().is_empty() {
                return Err(FieldError::new(
                    license_at.field("name"),
                    "[installer_options.license].name must be nonempty",
                ));
            }
        }
        Ok(Names { features, params })
    }

    fn validate_features(&self, at: &FieldPath) -> Result<BTreeSet<&str>, FieldError> {
        let option = &self.id;
        let row = |index: usize, field: &str| at.table("features").index(index).field(field);
        let mut ids = BTreeSet::new();
        for (index, feature) in self.features.iter().enumerate() {
            let table = format!("[[installer_options.features]] `{}`", feature.id);
            if !is_portable_token(&feature.id) {
                return Err(FieldError::new(
                    row(index, "id"),
                    format!("{table} id must be a portable lowercase token"),
                ));
            }
            if !ids.insert(feature.id.as_str()) {
                return Err(FieldError::new(
                    row(index, "id"),
                    format!("{table} is duplicated in installer option `{option}`"),
                ));
            }
            if feature.name.trim().is_empty() {
                return Err(FieldError::new(
                    row(index, "name"),
                    format!("{table} name must be nonempty"),
                ));
            }
        }
        let parents: BTreeMap<&str, &str> = self
            .features
            .iter()
            .filter_map(|feature| Some((feature.id.as_str(), feature.parent.as_deref()?)))
            .collect();
        let mut groups: BTreeMap<&str, Vec<(usize, &FeatureDecl)>> = BTreeMap::new();
        for (index, feature) in self.features.iter().enumerate() {
            let table = format!("[[installer_options.features]] `{}`", feature.id);
            if let Some(parent) = &feature.parent
                && !ids.contains(parent.as_str())
            {
                return Err(FieldError::new(
                    row(index, "parent"),
                    format!("{table} names an undeclared parent `{parent}`"),
                ));
            }
            // Walking up more parents than there are features is a cycle.
            let mut current = feature.id.as_str();
            for _ in 0..=self.features.len() {
                match parents.get(current) {
                    Some(parent) if *parent == feature.id => {
                        return Err(FieldError::new(
                            row(index, "parent"),
                            format!("{table} is its own ancestor; the tree has a cycle"),
                        ));
                    }
                    Some(parent) => current = parent,
                    None => break,
                }
            }
            for (position, required) in feature.requires.iter().enumerate() {
                if required == &feature.id || !ids.contains(required.as_str()) {
                    return Err(FieldError::new(
                        at.table("features")
                            .index(index)
                            .table("requires")
                            .index(position),
                        format!(
                            "{table} requires `{required}`, which is not another declared feature"
                        ),
                    ));
                }
            }
            if let Some(group) = &feature.exclusive_group {
                if !is_portable_token(group) {
                    return Err(FieldError::new(
                        row(index, "exclusive_group"),
                        format!(
                            "{table} exclusive_group `{group}` must be a portable lowercase token"
                        ),
                    ));
                }
                groups.entry(group).or_default().push((index, feature));
            }
        }
        for (group, members) in groups {
            let defaults = members.iter().filter(|(_, member)| member.default).count();
            if defaults != 1 {
                // No member is at fault alone; the first one that names the
                // group is where an editor shows it.
                let first = members.first().map_or(0, |(index, _)| *index);
                return Err(FieldError::new(
                    row(first, "exclusive_group"),
                    format!(
                        "exclusive_group `{group}` of installer option `{option}` has {defaults} default members; \
                         exactly one member must be `default = true`"
                    ),
                ));
            }
        }
        Ok(ids)
    }

    fn validate_profiles(
        &self,
        features: &BTreeSet<&str>,
        at: &FieldPath,
    ) -> Result<(), FieldError> {
        let mut ids = BTreeSet::new();
        for (index, profile) in self.profiles.iter().enumerate() {
            let table = format!("[[installer_options.profiles]] `{}`", profile.id);
            let row = at.table("profiles").index(index);
            if !is_portable_token(&profile.id) {
                return Err(FieldError::new(
                    row.field("id"),
                    format!("{table} id must be a portable lowercase token"),
                ));
            }
            if !ids.insert(profile.id.as_str()) {
                return Err(FieldError::new(
                    row.field("id"),
                    format!("{table} is duplicated in installer option `{}`", self.id),
                ));
            }
            if profile.name.trim().is_empty() {
                return Err(FieldError::new(
                    row.field("name"),
                    format!("{table} name must be nonempty"),
                ));
            }
            if let Some((position, unknown)) = profile
                .features
                .iter()
                .enumerate()
                .find(|(_, id)| !features.contains(id.as_str()))
            {
                return Err(FieldError::new(
                    row.table("features").index(position),
                    format!("{table} names an undeclared feature `{unknown}`"),
                ));
            }
        }
        // The second default is the one too many.
        if let Some((index, _)) = self
            .profiles
            .iter()
            .enumerate()
            .filter(|(_, profile)| profile.default)
            .nth(1)
        {
            return Err(FieldError::new(
                at.table("profiles").index(index).field("default"),
                format!(
                    "installer option `{}` declares more than one default profile",
                    self.id
                ),
            ));
        }
        Ok(())
    }
}

/// Type-specific fields and a default that the type accepts; `at` is the
/// parameter's row.
pub(crate) fn validate_parameter(
    parameter: &ParameterDecl,
    table: &str,
    at: &FieldPath,
) -> Result<(), FieldError> {
    let refuse = |field: &str, message: String| Err(FieldError::new(at.field(field), message));
    if parameter.name.trim().is_empty() {
        return refuse("name", format!("{table} name must be nonempty"));
    }
    let kind = parameter.kind;
    if (parameter.min.is_some() || parameter.max.is_some()) && kind != ParameterType::Integer {
        let field = if parameter.min.is_some() {
            "min"
        } else {
            "max"
        };
        return refuse(
            field,
            format!("{table} min / max apply only to type = \"integer\""),
        );
    }
    if let (Some(min), Some(max)) = (parameter.min, parameter.max)
        && min > max
    {
        return refuse(
            "min",
            format!("{table} min {min} is greater than max {max}"),
        );
    }
    if parameter.must_exist && kind != ParameterType::Path {
        return refuse(
            "must_exist",
            format!("{table} must_exist applies only to type = \"path\""),
        );
    }
    if kind == ParameterType::Choice {
        let mut values = BTreeSet::new();
        if parameter.choices.is_empty() {
            return refuse(
                "choices",
                format!("{table} of type \"choice\" needs a nonempty choices array"),
            );
        }
        for (index, choice) in parameter.choices.iter().enumerate() {
            let choice_at = at.table("choices").index(index);
            if choice.value.is_empty() || choice.name.trim().is_empty() {
                let field = if choice.value.is_empty() {
                    "value"
                } else {
                    "name"
                };
                return Err(FieldError::new(
                    choice_at.field(field),
                    format!("{table} choices need a nonempty value and name"),
                ));
            }
            if !values.insert(choice.value.as_str()) {
                return Err(FieldError::new(
                    choice_at.field("value"),
                    format!("{table} choice `{}` is duplicated", choice.value),
                ));
            }
        }
    } else if !parameter.choices.is_empty() {
        return refuse(
            "choices",
            format!("{table} choices apply only to type = \"choice\""),
        );
    }
    if let Some(default) = &parameter.default {
        let matches = matches!(
            (kind, default),
            (ParameterType::Bool, ParameterValue::Bool(_))
                | (ParameterType::Integer, ParameterValue::Integer(_))
                | (
                    ParameterType::String | ParameterType::Choice | ParameterType::Path,
                    ParameterValue::Text(_)
                )
        );
        if !matches {
            return refuse(
                "default",
                format!(
                    "{table} default `{default}` is not a {} value",
                    kind.as_str()
                ),
            );
        }
        parameter
            .canonical(&default.to_string())
            .map_err(|reason| {
                FieldError::new(
                    at.field("default"),
                    format!("{table} default is invalid: {reason}"),
                )
            })?;
    }
    Ok(())
}
