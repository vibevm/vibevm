//! Explicit authored layer grants and relations; prose never grants authority.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-074#OVERRIDE-ADDITIVE");
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// ```
/// let decl: vibe_core::manifest::LayersSection = toml::from_str("schema=1")?;
/// assert!(decl.targets.is_empty());
/// # Ok::<(), toml::de::Error>(())
/// ```
pub struct LayersSection {
    pub schema: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<LayerTarget>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relations: Vec<LayerRelation>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// ```
/// let target: vibe_core::manifest::LayerTarget = toml::from_str("id='width'\nsource='spec://org.demo/p/common/STYLE#width'")?;
/// assert_eq!(target.policy, vibe_core::manifest::LayerPolicy::Protected);
/// # Ok::<(), toml::de::Error>(())
/// ```
pub struct LayerTarget {
    pub id: String,
    pub source: String,
    #[serde(default)]
    pub policy: LayerPolicy,
    #[serde(default)]
    pub category: LayerCategory,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
/// ```
/// assert_eq!(vibe_core::manifest::LayerPolicy::default(), vibe_core::manifest::LayerPolicy::Protected);
/// ```
pub enum LayerPolicy {
    Overridable,
    #[default]
    Protected,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
/// ```
/// assert_eq!(vibe_core::manifest::LayerCategory::default(), vibe_core::manifest::LayerCategory::Preference);
/// ```
pub enum LayerCategory {
    #[default]
    Preference,
    Safety,
    Attribution,
    CommitDiscipline,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// ```
/// let relation: vibe_core::manifest::LayerRelation = toml::from_str("id='width'\nkind='supplement'\ntarget_layer='project'\ntarget='width'\nsource='spec://org.demo/p/common/STYLE#width'")?;
/// assert!(relation.target_owner.is_none());
/// # Ok::<(), toml::de::Error>(())
/// ```
pub struct LayerRelation {
    pub id: String,
    pub kind: LayerRelationKind,
    pub target_layer: LayerName,
    pub target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_owner: Option<String>,
    pub source: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
/// ```
/// assert_ne!(vibe_core::manifest::LayerRelationKind::Replace, vibe_core::manifest::LayerRelationKind::Supplement);
/// ```
pub enum LayerRelationKind {
    Supplement,
    Replace,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
/// ```
/// assert!(vibe_core::manifest::LayerName::Project < vibe_core::manifest::LayerName::User);
/// ```
pub enum LayerName {
    Project,
    User,
    Override,
}
impl LayersSection {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != 1 {
            return Err("[layers] requires schema = 1".into());
        }
        fn unique<'a>(ids: impl Iterator<Item = &'a str>) -> Result<(), String> {
            let mut seen = BTreeSet::new();
            for id in ids {
                if id.is_empty()
                    || !id
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
                    || !seen.insert(id)
                {
                    return Err(format!("[layers] invalid or duplicate id `{id}`"));
                }
            }
            Ok(())
        }
        unique(self.targets.iter().map(|row| row.id.as_str()))?;
        unique(self.relations.iter().map(|row| row.id.as_str()))?;
        for row in &self.targets {
            if row.policy == LayerPolicy::Overridable && row.category != LayerCategory::Preference {
                return Err(format!(
                    "[layers] target `{}`: safety, attribution and commit-discipline are always protected",
                    row.id
                ));
            }
        }
        for source in self
            .targets
            .iter()
            .map(|r| &r.source)
            .chain(self.relations.iter().map(|r| &r.source))
        {
            if !source.starts_with("spec://")
                || !source
                    .split_once('#')
                    .is_some_and(|(_, anchor)| !anchor.is_empty())
            {
                return Err(format!(
                    "[layers] source `{source}` requires an explicit spec:// document and anchor"
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
