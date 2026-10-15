//! Normalized feature algebra. No authored wire grammar is published here.
//! Governing contract: PROP-003#M05B1-DECLARATIONS and #M05B1-LOWERING.
use super::FeaturesTable;
use crate::PackageRef;
use crate::manifest::{
    AccessLevel, GitPackageDep, LinkType, PathPackageDep, Requires, VarRegistryDep,
};
use std::collections::{BTreeMap, BTreeSet};

fn token(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-+.".contains(&b))
        && s != "."
        && s != ".."
}

/// A reference in exactly one package's feature namespace.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalFeatureRef(String);
impl LocalFeatureRef {
    pub fn new(s: impl Into<String>) -> Result<Self, EffectError> {
        let s = s.into();
        if !token(&s) {
            return Err(EffectError::Malformed(s));
        }
        Ok(Self(s))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Stable declaration identity, never an array index or a consumer alias.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DependencyDeclarationId(String);
impl DependencyDeclarationId {
    pub fn new(s: impl Into<String>) -> Result<Self, EffectError> {
        let s = s.into();
        let valid = if let Some((group, name)) = s.split_once('/') {
            crate::Group::parse(group).is_ok() && token(name) && !name.contains('/')
        } else {
            token(&s)
        };
        if !valid {
            return Err(EffectError::Malformed(s));
        }
        Ok(Self(s))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencySource {
    Registry(PackageRef),
    Git(GitPackageDep),
    Path(PathPackageDep),
    Workspace(VarRegistryDep),
}

/// All source, version and visibility data occurs here, never on effects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyDeclaration {
    pub source: DependencySource,
    pub optional: bool,
    pub link: Option<LinkType>,
    pub access: Option<AccessLevel>,
    pub friend: Option<bool>,
    pub exclude: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConfigValue {
    Bool(bool),
    Integer(i64),
    String(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FeatureStrength {
    Strong,
    Weak,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FeatureEffect {
    Local(LocalFeatureRef),
    ActivateDependency(DependencyDeclarationId),
    TargetFeature {
        dependency: DependencyDeclarationId,
        feature: LocalFeatureRef,
        strength: FeatureStrength,
    },
    ExportedSubskill(String),
    ConfigValue(LocalFeatureRef),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EffectError {
    #[error("malformed feature reference `{0}`")]
    Malformed(String),
    #[error("unknown feature `{0}`")]
    UnknownFeature(String),
    #[error("undeclared dependency `{0}`")]
    UnknownDependency(String),
    #[error("undeclared exported subskill `{0}`")]
    UnknownSubskill(String),
    #[error("undeclared config value `{0}`")]
    UnknownConfig(String),
    #[error("duplicate or ambiguous declaration `{0}`")]
    Duplicate(String),
    #[error("dangling dependency metadata `{0}`")]
    DanglingMetadata(String),
}

/// Package-local declaration context. Aliases never enter normalized effects.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeatureDeclarations {
    pub dependencies: BTreeMap<DependencyDeclarationId, DependencyDeclaration>,
    pub aliases: BTreeMap<LocalFeatureRef, DependencyDeclarationId>,
    pub exported_subskills: BTreeSet<String>,
    pub config_values: BTreeMap<LocalFeatureRef, ConfigValue>,
}
impl FeatureDeclarations {
    /// Normalize legacy sources once. Optionality is explicit caller input;
    /// the old requires wire format has no optional field.
    pub fn from_requires(
        requires: &Requires,
        optional: &BTreeSet<DependencyDeclarationId>,
    ) -> Result<Self, EffectError> {
        let mut out = Self::default();
        let sources = requires
            .packages
            .iter()
            .cloned()
            .map(DependencySource::Registry)
            .chain(
                requires
                    .git_packages
                    .iter()
                    .cloned()
                    .map(DependencySource::Git),
            )
            .chain(
                requires
                    .path_packages
                    .iter()
                    .cloned()
                    .map(DependencySource::Path),
            )
            .chain(
                requires
                    .var_packages
                    .iter()
                    .cloned()
                    .map(DependencySource::Workspace),
            );
        for source in sources {
            let key = match &source {
                DependencySource::Registry(p) => {
                    if p.group.is_none() {
                        return Err(EffectError::Malformed(p.to_string()));
                    }
                    p.qualified_name()
                }
                DependencySource::Git(p) => format!("{}/{}", p.group, p.name),
                DependencySource::Path(p) => format!("{}/{}", p.group, p.name),
                DependencySource::Workspace(p) => format!("{}/{}", p.group, p.name),
            };
            let id = DependencyDeclarationId::new(&key)?;
            let declaration = DependencyDeclaration {
                source,
                optional: optional.contains(&id),
                link: requires.links.get(&key).copied(),
                access: requires.accesses.get(&key).copied(),
                friend: requires.friend_flags.get(&key).copied(),
                exclude: requires.excludes.get(&key).cloned(),
            };
            if out.dependencies.insert(id, declaration).is_some() {
                return Err(EffectError::Duplicate(key));
            }
        }
        for key in requires
            .links
            .keys()
            .chain(requires.accesses.keys())
            .chain(requires.friend_flags.keys())
            .chain(requires.excludes.keys())
        {
            if !out
                .dependencies
                .contains_key(&DependencyDeclarationId::new(key)?)
            {
                return Err(EffectError::DanglingMetadata(key.clone()));
            }
        }
        for id in optional {
            if !out.dependencies.contains_key(id) {
                return Err(EffectError::UnknownDependency(id.0.clone()));
            }
        }
        Ok(out)
    }
    pub fn validate(&self) -> Result<(), EffectError> {
        for (alias, id) in &self.aliases {
            if !self.dependencies.contains_key(id) {
                return Err(EffectError::UnknownDependency(id.0.clone()));
            }
            if self
                .dependencies
                .contains_key(&DependencyDeclarationId::new(alias.as_str())?)
            {
                return Err(EffectError::Duplicate(alias.0.clone()));
            }
        }
        for path in &self.exported_subskills {
            validate_subskill(path)?;
        }
        Ok(())
    }
    fn resolve(&self, raw: &str) -> Result<DependencyDeclarationId, EffectError> {
        let id = DependencyDeclarationId::new(raw)?;
        let id = LocalFeatureRef::new(raw)
            .ok()
            .and_then(|a| self.aliases.get(&a))
            .cloned()
            .unwrap_or(id);
        if !self.dependencies.contains_key(&id) {
            return Err(EffectError::UnknownDependency(raw.into()));
        }
        Ok(id)
    }
}

fn validate_subskill(s: &str) -> Result<(), EffectError> {
    if !s.split('/').all(token) {
        return Err(EffectError::Malformed(s.into()));
    }
    Ok(())
}

impl FeatureEffect {
    /// Parse only legacy syntax. Declaration resolution happens in lowering.
    pub fn parse_legacy(raw: &str) -> Result<Self, EffectError> {
        if let Some(path) = raw.strip_prefix("subskill:") {
            validate_subskill(path)?;
            return Ok(Self::ExportedSubskill(path.into()));
        }
        if let Some(dep) = raw.strip_prefix("dep:") {
            return Ok(Self::ActivateDependency(DependencyDeclarationId::new(dep)?));
        }
        if let Some((dep, feature)) = raw.rsplit_once('/') {
            let (dep, strength) = dep
                .strip_suffix('?')
                .map_or((dep, FeatureStrength::Strong), |d| {
                    (d, FeatureStrength::Weak)
                });
            return Ok(Self::TargetFeature {
                dependency: DependencyDeclarationId::new(dep)?,
                feature: LocalFeatureRef::new(feature)?,
                strength,
            });
        }
        Ok(Self::Local(LocalFeatureRef::new(raw)?))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeatureProgram {
    pub features: BTreeMap<LocalFeatureRef, BTreeSet<FeatureEffect>>,
    pub exclusive: BTreeMap<String, Vec<LocalFeatureRef>>,
}
impl FeatureProgram {
    /// Compatibility lowering for callers with only a legacy feature table.
    /// This cannot prove dependency/export existence; use lower_legacy for that.
    pub fn lower_table(table: &FeaturesTable) -> Result<Self, EffectError> {
        let mut out = Self::default();
        for (name, values) in &table.features {
            out.features.insert(
                LocalFeatureRef::new(name)?,
                values
                    .iter()
                    .map(|v| FeatureEffect::parse_legacy(v))
                    .collect::<Result<_, _>>()?,
            );
        }
        for (group, members) in &table.exclusive {
            out.exclusive.insert(
                group.clone(),
                members
                    .iter()
                    .map(LocalFeatureRef::new)
                    .collect::<Result<_, _>>()?,
            );
        }
        out.validate_locals()?;
        Ok(out)
    }
    pub fn lower_legacy(
        table: &FeaturesTable,
        declarations: &FeatureDeclarations,
    ) -> Result<Self, EffectError> {
        Self::lower_legacy_in_context(table, declarations, Some(&declarations.exported_subskills))
    }

    /// Legacy manifests supply dependency declarations but no export catalogue.
    /// Keep subskill selectors for downstream validation without claiming their
    /// existence or synthesizing declarations from uses. Configs stay strict.
    pub fn lower_legacy_dependencies(
        table: &FeaturesTable,
        declarations: &FeatureDeclarations,
    ) -> Result<Self, EffectError> {
        Self::lower_legacy_in_context(table, declarations, None)
    }

    fn lower_legacy_in_context(
        table: &FeaturesTable,
        declarations: &FeatureDeclarations,
        exported_subskills: Option<&BTreeSet<String>>,
    ) -> Result<Self, EffectError> {
        declarations.validate()?;
        let mut out = Self::lower_table(table)?;
        for effects in out.features.values_mut() {
            *effects = effects
                .iter()
                .map(|effect| {
                    Ok(match effect {
                        FeatureEffect::ActivateDependency(id) => {
                            FeatureEffect::ActivateDependency(declarations.resolve(id.as_str())?)
                        }
                        FeatureEffect::TargetFeature {
                            dependency,
                            feature,
                            strength,
                        } => FeatureEffect::TargetFeature {
                            dependency: declarations.resolve(dependency.as_str())?,
                            feature: feature.clone(),
                            strength: *strength,
                        },
                        other => other.clone(),
                    })
                })
                .collect::<Result<_, EffectError>>()?;
        }
        out.validate_in_context(declarations, exported_subskills)?;
        Ok(out)
    }
    fn validate_locals(&self) -> Result<(), EffectError> {
        for local in self
            .features
            .values()
            .flatten()
            .filter_map(|e| {
                if let FeatureEffect::Local(f) = e {
                    Some(f)
                } else {
                    None
                }
            })
            .chain(self.exclusive.values().flatten())
        {
            if !self.features.contains_key(local) {
                return Err(EffectError::UnknownFeature(local.0.clone()));
            }
        }
        Ok(())
    }
    pub fn validate(&self, declarations: &FeatureDeclarations) -> Result<(), EffectError> {
        self.validate_in_context(declarations, Some(&declarations.exported_subskills))
    }

    fn validate_in_context(
        &self,
        declarations: &FeatureDeclarations,
        exported_subskills: Option<&BTreeSet<String>>,
    ) -> Result<(), EffectError> {
        declarations.validate()?;
        self.validate_locals()?;
        for effect in self.features.values().flatten() {
            match effect {
                FeatureEffect::ActivateDependency(id)
                | FeatureEffect::TargetFeature { dependency: id, .. }
                    if !declarations.dependencies.contains_key(id) =>
                {
                    return Err(EffectError::UnknownDependency(id.0.clone()));
                }
                FeatureEffect::ExportedSubskill(path) => {
                    validate_subskill(path)?;
                    if exported_subskills.is_some_and(|exports| !exports.contains(path)) {
                        return Err(EffectError::UnknownSubskill(path.clone()));
                    }
                }
                FeatureEffect::ConfigValue(key)
                    if !declarations.config_values.contains_key(key) =>
                {
                    return Err(EffectError::UnknownConfig(key.0.clone()));
                }
                _ => {}
            }
        }
        Ok(())
    }
}
