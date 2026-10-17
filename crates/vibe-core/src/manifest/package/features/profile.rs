//! Explicit feature profile reader; the ordinary manifest decoder stays legacy.
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{FeaturesTable, constraints::*, effects::*};
use crate::manifest::{Manifest, package::ManifestWire};

pub mod constrained;

pub(crate) trait FeatureWire: Default + Clone {
    fn is_empty(&self) -> bool;
}
impl FeatureWire for FeaturesTable {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedFeatureManifest {
    pub manifest: Manifest,
    pub declarations: FeatureDeclarations,
    pub program: FeatureProgram,
    pub constraints: Constraints,
    pub selection: Selection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum TypedEffect {
    Local(Local),
    Activation(Activation),
    Forward(Forward),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Local {
    local: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Activation {
    dependency: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Forward {
    dependency: String,
    feature: String,
    strength: Strength,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Strength {
    Strong,
    Weak,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Group {
    cardinality: ZeroOrOne,
    members: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
enum ZeroOrOne {
    #[serde(rename = "zero-or-one")]
    Value,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct TypedFeatures {
    features: BTreeMap<String, Vec<TypedEffect>>,
    groups: BTreeMap<String, Group>,
}
impl FeatureWire for TypedFeatures {
    fn is_empty(&self) -> bool {
        self.features.is_empty() && self.groups.is_empty()
    }
}
impl<'de> Deserialize<'de> for TypedFeatures {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = BTreeMap::<String, toml::Value>::deserialize(d)?;
        let mut out = Self::default();
        for (key, value) in raw {
            if key == "exclusive" {
                out.groups = value.try_into().map_err(serde::de::Error::custom)?;
            } else {
                out.features
                    .insert(key, value.try_into().map_err(serde::de::Error::custom)?);
            }
        }
        Ok(out)
    }
}

impl TypedFeatures {
    /// A complete lossless semantic projection for the finite r1 grammar.
    /// It is checked beside the typed model, never an empty feature placeholder.
    fn legacy(&self) -> FeaturesTable {
        FeaturesTable {
            features: self
                .features
                .iter()
                .map(|(name, effects)| {
                    let values = effects
                        .iter()
                        .map(|effect| match effect {
                            TypedEffect::Local(v) => v.local.clone(),
                            TypedEffect::Activation(v) => format!("dep:{}", v.dependency),
                            TypedEffect::Forward(v) => format!(
                                "{}{}/{}",
                                v.dependency,
                                if matches!(v.strength, Strength::Weak) {
                                    "?"
                                } else {
                                    ""
                                },
                                v.feature
                            ),
                        })
                        .collect();
                    (name.clone(), values)
                })
                .collect(),
            exclusive: self
                .groups
                .iter()
                .map(|(k, v)| (k.clone(), v.members.clone()))
                .collect(),
        }
    }
}

pub(crate) fn compose(manifest: Manifest) -> Result<CheckedFeatureManifest, String> {
    let declarations = FeatureDeclarations::from_requires(&manifest.requires, &BTreeSet::new())
        .map_err(|e| e.to_string())?;
    let program = FeatureProgram::lower_legacy(&manifest.features, &declarations)
        .map_err(|e| e.to_string())?;
    let mut constraints = Constraints::default();
    for (name, members) in &program.exclusive {
        constraints.groups.insert(
            LocalFeatureRef::new(name).map_err(|e| e.to_string())?,
            ConstrainedGroup {
                cardinality: Cardinality::ZeroOrOne,
                members: members.iter().cloned().collect(),
                guards: vec![],
                defaults: None,
            },
        );
    }
    constraints
        .order(&program, &declarations)
        .map_err(|e| e.to_string())?;
    manifest.validate().map_err(|e| e.to_string())?;
    Ok(CheckedFeatureManifest {
        manifest,
        declarations,
        program,
        constraints,
        selection: Selection::Omitted,
    })
}

pub(crate) fn read_typed(text: &str) -> Result<CheckedFeatureManifest, String> {
    let wire: ManifestWire<TypedFeatures> = toml::from_str(text).map_err(|e| e.to_string())?;
    let legacy = wire.features.legacy();
    // Parsing typed fields is not sufficient: reject alias/legacy sigils in
    // typed operands before constructing their canonical semantic projection.
    validate_typed(&wire.features.features)?;
    compose(wire.into_manifest_with_features(legacy)?)
}

fn validate_typed(features: &BTreeMap<String, Vec<TypedEffect>>) -> Result<(), String> {
    for effects in features.values() {
        for effect in effects {
            match effect {
                TypedEffect::Local(v) => {
                    LocalFeatureRef::new(&v.local).map_err(|e| e.to_string())?;
                }
                TypedEffect::Activation(v) => {
                    DependencyDeclarationId::new(&v.dependency).map_err(|e| e.to_string())?;
                }
                TypedEffect::Forward(v) => {
                    DependencyDeclarationId::new(&v.dependency).map_err(|e| e.to_string())?;
                    LocalFeatureRef::new(&v.feature).map_err(|e| e.to_string())?;
                }
            }
        }
    }
    Ok(())
}
