//! Ordinary authored configuration. Definitions retain presentation and effects;
//! local feature/choice evaluation remains in the shared configuration kernel.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#AUTHORED-DEFINITIONS");

use super::{ParameterDecl, ParameterValue};
use crate::configuration::{Cardinality, MergeIntent};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod intent;
mod lower;
mod package;
pub use package::{PackageConfiguration, PackageConfigurationError};
mod semantic;
mod validate;
mod vocabulary;
pub use intent::{ConfigurationIntentDecl, NormalizedConfigurationInput};

/// A package or installer-owned common definition, admitted by the ordinary reader.
/// ```
/// use vibe_core::manifest::ConfigurationDecl;
/// let definition: ConfigurationDecl = toml::from_str("").unwrap();
/// assert!(definition.compile_choices().is_ok());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationDecl {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<ConfigurationFeatureDecl>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<ConfigurationGroupDecl>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<ParameterDecl>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub profiles: Vec<ConfigurationProfileDecl>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub config_values: BTreeMap<String, ParameterValue>,
}

/// A literal local feature and its presentation, graph relations and typed effects.
/// ```
/// use vibe_core::manifest::ConfigurationFeatureDecl;
/// let f: ConfigurationFeatureDecl = toml::from_str("id='lint level'\nname='Lint'").unwrap();
/// assert_eq!(f.id, "lint level");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationFeatureDecl {
    pub id: String,
    pub name: String,
    /// Source interface visibility; the pure feature kernel keeps opaque IDs.
    #[serde(
        default,
        skip_serializing_if = "ConfigurationFeatureVisibility::is_public"
    )]
    pub visibility: ConfigurationFeatureVisibility,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub guards: Vec<ConfigurationGuard>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<ConfigurationEffect>,
}

/// Common source visibility, independent of ID spelling and source-read access.
/// ```
/// use vibe_core::manifest::ConfigurationFeatureVisibility;
/// assert_eq!(ConfigurationFeatureVisibility::default(), ConfigurationFeatureVisibility::Public);
/// ```
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#FEATURE-VISIBILITY")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigurationFeatureVisibility {
    #[default]
    Public,
    Internal,
}
impl ConfigurationFeatureVisibility {
    fn is_public(&self) -> bool {
        *self == Self::Public
    }
}

/// Positive choice guards; the two identifier namespaces are distinct.
/// ```
/// use vibe_core::manifest::ConfigurationGuard;
/// let g: ConfigurationGuard = toml::from_str("member='wal'").unwrap();
/// assert_eq!(g, ConfigurationGuard::Member("wal".into()));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigurationGuard {
    Group(String),
    Member(String),
}

/// Closed group meanings: consented choice defaults differ from closure fallback.
/// ```
/// use vibe_core::manifest::ConfigurationGroupDecl;
/// let g: ConfigurationGroupDecl = toml::from_str("mode='choice'\nid='g'\nname='Group'\ncardinality='exactly-one'\nmembers=['a']").unwrap();
/// assert!(matches!(g, ConfigurationGroupDecl::Choice { .. }));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase", deny_unknown_fields)]
pub enum ConfigurationGroupDecl {
    Choice {
        id: String,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(with = "vocabulary::cardinality")]
        cardinality: Cardinality,
        members: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        defaults: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        guards: Vec<ConfigurationGuard>,
    },
    Closure {
        id: String,
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(with = "vocabulary::cardinality")]
        cardinality: Cardinality,
        members: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fallback: Option<Vec<String>>,
    },
}

/// Typed effect records retain data, not executable closures or command text.
/// ```
/// use vibe_core::manifest::ConfigurationEffect;
/// let e: ConfigurationEffect = toml::from_str("kind='config-value'\nname='verbose'").unwrap();
/// assert!(matches!(e, ConfigurationEffect::ConfigValue { .. }));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ConfigurationEffect {
    Dependency {
        target: String,
    },
    Forward {
        target: String,
        feature: String,
        strength: ForwardStrength,
    },
    Subskill {
        path: String,
    },
    ConfigValue {
        name: String,
    },
}

/// Weak forwarding never activates its own dependency declaration.
/// ```
/// use vibe_core::manifest::ForwardStrength;
/// assert_ne!(ForwardStrength::Strong, ForwardStrength::Weak);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ForwardStrength {
    Strong,
    Weak,
}

/// A profile supplies inputs; it does not complete missing answers or apply effects.
/// ```
/// use vibe_core::manifest::ConfigurationProfileDecl;
/// let p: ConfigurationProfileDecl = toml::from_str("id='typical'\nname='Typical'").unwrap();
/// assert!(p.selections.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationProfileDecl {
    pub id: String,
    pub name: String,
    /// Declared UI preselection, never automatic consent or execution.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub default: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub selections: BTreeMap<String, ConfigurationSelection>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, ParameterValue>,
}

/// Missing members means omitted; an empty vector is an explicit empty answer.
/// ```
/// use vibe_core::manifest::ConfigurationSelection;
/// let s: ConfigurationSelection = toml::from_str("intent='exact'\nmembers=[]").unwrap();
/// assert_eq!(s.members, Some(vec![]));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationSelection {
    #[serde(with = "vocabulary::intent")]
    pub intent: MergeIntent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub members: Option<Vec<String>>,
}

#[cfg(test)]
mod tests;
