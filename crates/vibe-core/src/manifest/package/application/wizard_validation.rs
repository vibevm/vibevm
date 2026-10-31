//! Validation of the feature tree, profiles, parameters and license of an
//! installer option (`PROP-065#feature-tree`, `#profiles`, `#parameters`,
//! `#license`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#feature-tree");

use std::collections::{BTreeMap, BTreeSet};

use crate::manifest::{declarant_path, is_portable_token};

use super::step_validation::Names;
use super::*;

impl InstallerOptionDecl {
    /// Check the wizard declarations and return the ids steps may name.
    pub(super) fn validate_wizard(&self) -> Result<Names<'_>, String> {
        let features = self.validate_features()?;
        self.validate_profiles(&features)?;
        let mut params = BTreeSet::new();
        for parameter in &self.parameters {
            let table = format!("[[installer_options.parameters]] `{}`", parameter.id);
            if !is_portable_token(&parameter.id) {
                return Err(format!("{table} id must be a portable lowercase token"));
            }
            if !params.insert(parameter.id.as_str()) {
                return Err(format!(
                    "{table} is duplicated in installer option `{}`",
                    self.id
                ));
            }
            validate_parameter(parameter, &table)?;
        }
        if let Some(license) = &self.license {
            declarant_path(&license.file).map_err(|fault| {
                format!(
                    "[installer_options.license].file must be a package-relative file: {}",
                    fault.reason()
                )
            })?;
            if license.name.trim().is_empty() {
                return Err("[installer_options.license].name must be nonempty".into());
            }
        }
        Ok(Names { features, params })
    }

    fn validate_features(&self) -> Result<BTreeSet<&str>, String> {
        let option = &self.id;
        let mut ids = BTreeSet::new();
        for feature in &self.features {
            let table = format!("[[installer_options.features]] `{}`", feature.id);
            if !is_portable_token(&feature.id) {
                return Err(format!("{table} id must be a portable lowercase token"));
            }
            if !ids.insert(feature.id.as_str()) {
                return Err(format!(
                    "{table} is duplicated in installer option `{option}`"
                ));
            }
            if feature.name.trim().is_empty() {
                return Err(format!("{table} name must be nonempty"));
            }
        }
        let parents: BTreeMap<&str, &str> = self
            .features
            .iter()
            .filter_map(|feature| Some((feature.id.as_str(), feature.parent.as_deref()?)))
            .collect();
        let mut groups: BTreeMap<&str, Vec<&FeatureDecl>> = BTreeMap::new();
        for feature in &self.features {
            let table = format!("[[installer_options.features]] `{}`", feature.id);
            if let Some(parent) = &feature.parent
                && !ids.contains(parent.as_str())
            {
                return Err(format!("{table} names an undeclared parent `{parent}`"));
            }
            // Walking up more parents than there are features is a cycle.
            let mut current = feature.id.as_str();
            for _ in 0..=self.features.len() {
                match parents.get(current) {
                    Some(parent) if *parent == feature.id => {
                        return Err(format!("{table} is its own ancestor; the tree has a cycle"));
                    }
                    Some(parent) => current = parent,
                    None => break,
                }
            }
            for required in &feature.requires {
                if required == &feature.id || !ids.contains(required.as_str()) {
                    return Err(format!(
                        "{table} requires `{required}`, which is not another declared feature"
                    ));
                }
            }
            if let Some(group) = &feature.exclusive_group {
                if !is_portable_token(group) {
                    return Err(format!(
                        "{table} exclusive_group `{group}` must be a portable lowercase token"
                    ));
                }
                groups.entry(group).or_default().push(feature);
            }
        }
        for (group, members) in groups {
            let defaults = members.iter().filter(|member| member.default).count();
            if defaults != 1 {
                return Err(format!(
                    "exclusive_group `{group}` of installer option `{option}` has {defaults} default members; \
                     exactly one member must be `default = true`"
                ));
            }
        }
        Ok(ids)
    }

    fn validate_profiles(&self, features: &BTreeSet<&str>) -> Result<(), String> {
        let mut ids = BTreeSet::new();
        for profile in &self.profiles {
            let table = format!("[[installer_options.profiles]] `{}`", profile.id);
            if !is_portable_token(&profile.id) {
                return Err(format!("{table} id must be a portable lowercase token"));
            }
            if !ids.insert(profile.id.as_str()) {
                return Err(format!(
                    "{table} is duplicated in installer option `{}`",
                    self.id
                ));
            }
            if profile.name.trim().is_empty() {
                return Err(format!("{table} name must be nonempty"));
            }
            if let Some(unknown) = profile
                .features
                .iter()
                .find(|id| !features.contains(id.as_str()))
            {
                return Err(format!("{table} names an undeclared feature `{unknown}`"));
            }
        }
        if self
            .profiles
            .iter()
            .filter(|profile| profile.default)
            .count()
            > 1
        {
            return Err(format!(
                "installer option `{}` declares more than one default profile",
                self.id
            ));
        }
        Ok(())
    }
}

/// Type-specific fields and a default that the type accepts.
fn validate_parameter(parameter: &ParameterDecl, table: &str) -> Result<(), String> {
    if parameter.name.trim().is_empty() {
        return Err(format!("{table} name must be nonempty"));
    }
    let kind = parameter.kind;
    if (parameter.min.is_some() || parameter.max.is_some()) && kind != ParameterType::Integer {
        return Err(format!(
            "{table} min / max apply only to type = \"integer\""
        ));
    }
    if let (Some(min), Some(max)) = (parameter.min, parameter.max)
        && min > max
    {
        return Err(format!("{table} min {min} is greater than max {max}"));
    }
    if parameter.must_exist && kind != ParameterType::Path {
        return Err(format!(
            "{table} must_exist applies only to type = \"path\""
        ));
    }
    if kind == ParameterType::Choice {
        let mut values = BTreeSet::new();
        if parameter.choices.is_empty() {
            return Err(format!(
                "{table} of type \"choice\" needs a nonempty choices array"
            ));
        }
        for choice in &parameter.choices {
            if choice.value.is_empty() || choice.name.trim().is_empty() {
                return Err(format!("{table} choices need a nonempty value and name"));
            }
            if !values.insert(choice.value.as_str()) {
                return Err(format!("{table} choice `{}` is duplicated", choice.value));
            }
        }
    } else if !parameter.choices.is_empty() {
        return Err(format!("{table} choices apply only to type = \"choice\""));
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
            return Err(format!(
                "{table} default `{default}` is not a {} value",
                kind.as_str()
            ));
        }
        parameter
            .canonical(&default.to_string())
            .map_err(|reason| format!("{table} default is invalid: {reason}"))?;
    }
    Ok(())
}
