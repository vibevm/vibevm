//! `[features]` — the feature-definition table of a package manifest
//! (PROP-003 §2.4) and its hand-rolled serde forms.

specmark::scope!("spec://vibevm/modules/vibe-registry/PROP-002#git-source");

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// `[features]` table — feature definitions per PROP-003 §2.4.
///
/// Each feature maps to a list of activation strings; the strings can
/// be other feature names, dep-references (`dep:foo`, `foo?/feat`), or
/// subskill-references (`subskill:<path>`). The TOML form is a mix of
/// flat string-list keys plus a nested `exclusive` table; we deserialise
/// both via a manual visitor so the public API stays clean.
///
/// ```toml
/// [features]
/// default = ["wal-protocol"]
/// wal-protocol = []
/// rust-stack = ["subskill:stack/rust"]
/// python-stack = ["subskill:stack/python"]
///
/// [features.exclusive]
/// stacks = ["rust-stack", "python-stack"]
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeaturesTable {
    /// `feature-name` → list of activation strings.
    pub features: BTreeMap<String, Vec<String>>,
    /// `[features.exclusive]` — at-most-one named groups.
    pub exclusive: BTreeMap<String, Vec<String>>,
}

impl FeaturesTable {
    pub fn is_empty(&self) -> bool {
        self.features.is_empty() && self.exclusive.is_empty()
    }

    /// Convenience — list of features active by default
    /// (the `default` feature's activation list, if present).
    pub fn defaults(&self) -> &[String] {
        self.features
            .get("default")
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Look up a feature's activation list.
    pub fn get(&self, name: &str) -> Option<&[String]> {
        self.features.get(name).map(|v| v.as_slice())
    }
}

impl Serialize for FeaturesTable {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut total = self.features.len();
        if !self.exclusive.is_empty() {
            total += 1;
        }
        let mut m = s.serialize_map(Some(total))?;
        for (k, v) in &self.features {
            m.serialize_entry(k, v)?;
        }
        if !self.exclusive.is_empty() {
            m.serialize_entry("exclusive", &self.exclusive)?;
        }
        m.end()
    }
}

impl<'de> Deserialize<'de> for FeaturesTable {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        // Receive as a generic `BTreeMap<String, toml::Value>` then split
        // into features (string lists) and the special `exclusive` table.
        let raw: BTreeMap<String, toml::Value> = BTreeMap::deserialize(d)?;
        let mut features: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut exclusive: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (k, v) in raw {
            if k == "exclusive" {
                let table: BTreeMap<String, Vec<String>> =
                    v.try_into().map_err(serde::de::Error::custom)?;
                exclusive = table;
                continue;
            }
            let arr: Vec<String> = v.try_into().map_err(serde::de::Error::custom)?;
            features.insert(k, arr);
        }
        Ok(FeaturesTable {
            features,
            exclusive,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn features_table_roundtrips() {
        let raw = r#"
default = ["wal-protocol"]
wal-protocol = []
rust-stack = ["subskill:stack/rust"]

[exclusive]
stacks = ["rust-stack", "python-stack"]
"#;
        let ft: FeaturesTable = toml::from_str(raw).unwrap();
        assert_eq!(ft.defaults(), &["wal-protocol".to_string()]);
        assert_eq!(ft.get("rust-stack").unwrap().len(), 1);
        assert_eq!(ft.exclusive.get("stacks").unwrap().len(), 2);
        let rendered = toml::to_string_pretty(&ft).unwrap();
        let back: FeaturesTable = toml::from_str(&rendered).unwrap();
        assert_eq!(ft, back);
    }
}
