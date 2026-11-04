//! Full local validation and scope-bound effect admission; never reads host state.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#AUTHORED-SCOPES");
use super::*;
use crate::PackageKind;
use crate::manifest::{FieldError, FieldPath, Requires, declarant_path, is_portable_token};
use std::collections::BTreeSet;

use crate::configuration::valid_identifier as literal;
fn named<'a>(
    id: &'a str,
    name: &str,
    seen: &mut BTreeSet<&'a str>,
    at: &FieldPath,
) -> Result<(), FieldError> {
    if !literal(id) {
        return Err(FieldError::new(
            at.field("id"),
            "identifier must be nonblank and contain no control characters",
        ));
    }
    if !seen.insert(id) {
        return Err(FieldError::new(
            at.field("id"),
            format!("duplicate identifier `{id}`"),
        ));
    }
    if name.trim().is_empty() {
        return Err(FieldError::new(at.field("name"), "name must be nonempty"));
    }
    Ok(())
}
impl ConfigurationDecl {
    pub(super) fn validate_local(&self, at: &FieldPath) -> Result<(), FieldError> {
        let mut features = BTreeSet::new();
        for (i, f) in self.features.iter().enumerate() {
            let row = at.table("features").index(i);
            named(&f.id, &f.name, &mut features, &row)?;
            for (j, effect) in f.effects.iter().enumerate() {
                let ea = row.table("effects").index(j);
                match effect {
                    ConfigurationEffect::ConfigValue { name }
                        if !self.config_values.contains_key(name) =>
                    {
                        return Err(FieldError::new(
                            ea.field("name"),
                            format!("unknown config value `{name}`"),
                        ));
                    }
                    ConfigurationEffect::Subskill { path } => {
                        declarant_path(std::path::Path::new(path))
                            .map_err(|e| FieldError::new(ea.field("path"), e.reason()))?;
                    }
                    ConfigurationEffect::Forward {
                        feature, target, ..
                    } => {
                        if !literal(feature) {
                            return Err(FieldError::new(
                                ea.field("feature"),
                                "target feature must be a literal nonblank identifier without controls",
                            ));
                        }
                        super::super::package::parse_pkgref_key(target)
                            .map_err(|e| FieldError::new(ea.field("target"), e.to_string()))?;
                    }
                    ConfigurationEffect::Dependency { target } => {
                        super::super::package::parse_pkgref_key(target)
                            .map_err(|e| FieldError::new(ea.field("target"), e.to_string()))?;
                    }
                    _ => {}
                }
            }
        }
        for key in self.config_values.keys() {
            if !literal(key) {
                return Err(FieldError::new(
                    at.table("config_values").field(key),
                    "config value identifier must be nonblank without controls",
                ));
            }
        }
        let mut groups = BTreeSet::new();
        let mut choices = BTreeMap::new();
        for (i, group) in self.groups.iter().enumerate() {
            let (id, name) = match group {
                ConfigurationGroupDecl::Choice {
                    id, name, members, ..
                } => {
                    choices.insert(id.as_str(), members);
                    (id, name)
                }
                ConfigurationGroupDecl::Closure { id, name, .. } => (id, name),
            };
            named(id, name, &mut groups, &at.table("groups").index(i))?;
        }
        let mut parameters = BTreeMap::new();
        for (i, p) in self.parameters.iter().enumerate() {
            let row = at.table("parameters").index(i);
            if !is_portable_token(&p.id) {
                return Err(FieldError::new(
                    row.field("id"),
                    "parameter id must be a portable lowercase token",
                ));
            }
            if parameters.insert(p.id.as_str(), p).is_some() {
                return Err(FieldError::new(
                    row.field("id"),
                    format!("duplicate parameter `{}`", p.id),
                ));
            }
            super::super::package::application::validate_parameter(
                p,
                "configuration parameter",
                &row,
            )?;
        }
        if self
            .profiles
            .iter()
            .filter(|profile| profile.default)
            .count()
            > 1
        {
            return Err(FieldError::new(
                at.table("profiles"),
                "at most one profile may be default",
            ));
        }
        let mut profiles = BTreeSet::new();
        for (i, p) in self.profiles.iter().enumerate() {
            let row = at.table("profiles").index(i);
            named(&p.id, &p.name, &mut profiles, &row)?;
            let mut seen = BTreeSet::new();
            for id in &p.features {
                if !features.contains(id.as_str()) || !seen.insert(id) {
                    return Err(FieldError::new(
                        row.field("features"),
                        format!("unknown or duplicate profile feature `{id}`"),
                    ));
                }
                if choices.values().any(|members| members.contains(id)) {
                    return Err(FieldError::new(
                        row.field("features"),
                        format!("choice member `{id}` must use a group selection"),
                    ));
                }
            }
            for (group, answer) in &p.selections {
                let ga = row.table("selections").field(group);
                let Some(members) = choices.get(group.as_str()) else {
                    return Err(FieldError::new(
                        ga,
                        format!("unknown choice group `{group}`"),
                    ));
                };
                let mut seen = BTreeSet::new();
                for member in answer.members.iter().flatten() {
                    if !members.contains(member) || !seen.insert(member) {
                        return Err(FieldError::new(
                            ga.clone(),
                            format!("unknown or duplicate profile member `{member}`"),
                        ));
                    }
                }
            }
            for (id, value) in &p.parameters {
                let pa = row.table("parameters").field(id);
                let Some(parameter) = parameters.get(id.as_str()) else {
                    return Err(FieldError::new(pa, format!("unknown parameter `{id}`")));
                };
                if parameter.secret {
                    return Err(FieldError::new(
                        pa,
                        "secret parameter cannot have a public profile preset",
                    ));
                }
                let mut with_preset = (*parameter).clone();
                with_preset.default = Some(value.clone());
                super::super::package::application::validate_parameter(
                    &with_preset,
                    "profile parameter preset",
                    &pa,
                )
                .map_err(|error| FieldError::new(pa, error.message))?;
            }
        }
        Ok(())
    }

    pub(crate) fn validate_scope(
        &self,
        requires: &Requires,
        kind: PackageKind,
        installer: bool,
        at: &FieldPath,
    ) -> Result<(), FieldError> {
        self.compile_choices_at(at)?;
        for (i, f) in self.features.iter().enumerate() {
            for (j, effect) in f.effects.iter().enumerate() {
                let ea = at.table("features").index(i).table("effects").index(j);
                if (installer || matches!(kind, PackageKind::Doc | PackageKind::Remap))
                    && !matches!(effect, ConfigurationEffect::ConfigValue { .. })
                {
                    return Err(FieldError::new(
                        ea,
                        "this configuration scope supports data effects only; dependency, forwarding and subskill effects are unsupported",
                    ));
                }
                let target = match effect {
                    ConfigurationEffect::Dependency { target }
                    | ConfigurationEffect::Forward { target, .. } => target,
                    _ => continue,
                };
                let (asserted, group, name) = super::super::package::parse_pkgref_key(target)
                    .map_err(|e| FieldError::new(ea.field("target"), e.to_string()))?;
                let entries: Vec<_> = requires
                    .packages
                    .iter()
                    .map(|p| (p.kind, p.group.as_ref(), p.name.as_str()))
                    .chain(
                        requires
                            .git_packages
                            .iter()
                            .map(|p| (p.kind, Some(&p.group), p.name.as_str())),
                    )
                    .chain(
                        requires
                            .path_packages
                            .iter()
                            .map(|p| (p.kind, Some(&p.group), p.name.as_str())),
                    )
                    .chain(
                        requires
                            .var_packages
                            .iter()
                            .map(|p| (p.kind, Some(&p.group), p.name.as_str())),
                    )
                    .filter(|(_, g, n)| *g == Some(&group) && *n == name)
                    .collect();
                if entries.len() != 1 {
                    return Err(FieldError::new(
                        ea.field("target"),
                        format!(
                            "dependency target `{target}` must identify exactly one declared requires edge (found {})",
                            entries.len()
                        ),
                    ));
                }
                if asserted.zip(entries[0].0).is_some_and(|(a, b)| a != b) {
                    return Err(FieldError::new(
                        ea.field("target"),
                        format!("dependency kind for `{target}` disagrees with its declaration"),
                    ));
                }
            }
        }
        Ok(())
    }
}
