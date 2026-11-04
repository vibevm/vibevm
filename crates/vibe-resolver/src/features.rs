//! Feature expansion engine — PROP-003 §2.4.
//!
//! Given a `[features]` table from a package manifest plus a set of
//! roots-requested feature names, expand transitively into the full
//! activation set: feature → feature, `dep:foo`, `foo?/feat`,
//! `subskill:<path>`. Validate that no exclusive group is violated.
//!
//! Local implications use the shared configuration kernel. Package policy and
//! non-local value projection remain here. This does not bind foreign dependency
//! declarations or establish their still-open solver/install/boot consumers.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#LEGACY-PACKAGE-COMPATIBILITY");

use std::collections::{BTreeMap, BTreeSet};

use specmark::spec;
use thiserror::Error;
use vibe_core::manifest::FeaturesTable;

mod compiled;

#[cfg(test)]
#[path = "features/characterization.rs"]
mod characterization;
#[cfg(test)]
#[path = "features/compatibility.rs"]
mod compatibility;
#[cfg(test)]
#[path = "features/legacy.rs"]
mod legacy;
#[cfg(test)]
#[path = "features/regression.rs"]
mod regression;

/// One activation entry, parsed from a feature's activation list.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#features")]
pub enum FeatureValue {
    /// `feat-A` — enabling A enables this feature.
    Feature(String),
    /// `dep:foo` — activates the optional dep `foo`.
    Dep { dep_name: String },
    /// `foo/feat` (strong) or `foo?/feat` (weak).
    DepFeature {
        dep_name: String,
        dep_feature: String,
        weak: bool,
    },
    /// `subskill:<path>` — activates a subskill.
    Subskill { path: String },
}

impl FeatureValue {
    pub fn parse(raw: &str) -> Result<Self, FeatureError> {
        if let Some(path) = raw.strip_prefix("subskill:") {
            if path.is_empty() {
                return Err(FeatureError::Malformed(raw.to_string()));
            }
            return Ok(FeatureValue::Subskill {
                path: path.to_string(),
            });
        }
        if let Some(name) = raw.strip_prefix("dep:") {
            if name.is_empty() {
                return Err(FeatureError::Malformed(raw.to_string()));
            }
            return Ok(FeatureValue::Dep {
                dep_name: name.to_string(),
            });
        }
        if let Some((dep, feat)) = raw.split_once('/') {
            let (dep_name, weak) = match dep.strip_suffix('?') {
                Some(stripped) => (stripped.to_string(), true),
                None => (dep.to_string(), false),
            };
            if dep_name.is_empty() || feat.is_empty() {
                return Err(FeatureError::Malformed(raw.to_string()));
            }
            return Ok(FeatureValue::DepFeature {
                dep_name,
                dep_feature: feat.to_string(),
                weak,
            });
        }
        if raw.is_empty() {
            return Err(FeatureError::Malformed(raw.to_string()));
        }
        Ok(FeatureValue::Feature(raw.to_string()))
    }
}

/// Resolved feature expansion: every feature, dep, dep-feature, and
/// subskill-path that the requested feature set transitively pulls in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#features")]
pub struct FeatureExpansion {
    /// Names of features active in the package itself.
    pub active_features: BTreeSet<String>,
    /// `dep:foo` activations — names of optional deps that should be
    /// added to the dep graph.
    pub active_deps: BTreeSet<String>,
    /// `dep/feat` activations — `dep_name → set of feature names to
    /// enable on that dep`.
    pub dep_features: BTreeMap<String, BTreeSet<String>>,
    /// Weak `dep?/feat` activations — `dep_name → set of feature names
    /// to enable IF the dep is already in the graph`.
    pub weak_dep_features: BTreeMap<String, BTreeSet<String>>,
    /// Subskill paths activated through features (manual channel).
    pub active_subskills: BTreeSet<String>,
}

impl FeatureExpansion {
    /// Union another expansion's values. This operation alone does not perform
    /// cross-package solving or establish downstream effect consumption.
    pub fn merge(&mut self, other: &FeatureExpansion) {
        self.active_features
            .extend(other.active_features.iter().cloned());
        self.active_deps.extend(other.active_deps.iter().cloned());
        for (k, v) in &other.dep_features {
            self.dep_features
                .entry(k.clone())
                .or_default()
                .extend(v.iter().cloned());
        }
        for (k, v) in &other.weak_dep_features {
            self.weak_dep_features
                .entry(k.clone())
                .or_default()
                .extend(v.iter().cloned());
        }
        self.active_subskills
            .extend(other.active_subskills.iter().cloned());
    }
}

/// Configuration controlling which features to start from.
#[derive(Debug, Clone, Default)]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#features")]
pub struct FeatureRequest {
    /// Features explicitly requested on the CLI (`vibe install --features`).
    pub explicit: Vec<String>,
    /// `--no-default-features` — skip the `default` feature's activation.
    pub no_defaults: bool,
    /// `--all-features` — activate every named feature in the table.
    pub all: bool,
}

/// Expand a `[features]` table from the requested starting set.
///
/// Positive cycles terminate in the shared finite closure, never as an error.
/// Every local definition and activation syntax is validated, including private
/// and dormant branches. Unknown local references flag as `UnknownFeature`.
/// Exclusive-group violations flag as `ExclusiveViolation`. The caller
/// is expected to surface diagnostics actionably (e.g. through
/// `vibe install` step lines).
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#features")]
pub fn expand_features(
    table: &FeaturesTable,
    request: &FeatureRequest,
) -> Result<FeatureExpansion, FeatureError> {
    let seeds = seeds(table, request)?;
    let compiled = compiled::CompiledFeatures::new(table).map_err(|failure| failure.first)?;
    compiled.expand(seeds)
}

/// Package policy: all public names, or the named default plus explicit union.
/// The all-features mode retains precedence over the unused explicit list.
fn seeds(table: &FeaturesTable, request: &FeatureRequest) -> Result<Vec<String>, FeatureError> {
    let mut seed = BTreeSet::new();
    if request.all {
        seed.extend(
            table
                .features
                .keys()
                .filter(|id| !id.starts_with('_'))
                .cloned(),
        );
    } else {
        if !request.no_defaults && table.features.contains_key("default") {
            seed.insert("default".into());
        }
        for id in &request.explicit {
            if id.starts_with('_') {
                return Err(FeatureError::PrivateFeature(id.clone()));
            }
            if !table.features.contains_key(id) {
                return Err(FeatureError::UnknownFeature(id.clone()));
            }
            seed.insert(id.clone());
        }
    }
    Ok(seed.into_iter().collect())
}

/// Validate every local declaration/activation, then the actual default request.
/// Selecting all exclusive alternatives is a request conflict, not a definition error.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-069#LEGACY-PACKAGE-COMPATIBILITY")]
pub fn validate_features_table(table: &FeaturesTable) -> Vec<String> {
    let compiled = match compiled::CompiledFeatures::new(table) {
        Ok(compiled) => compiled,
        Err(failures) => {
            return failures
                .into_errors()
                .map(|error| format!("features table: {error}"))
                .collect();
        }
    };
    match seeds(table, &FeatureRequest::default()).and_then(|seeds| compiled.expand(seeds)) {
        Ok(_) => Vec::new(),
        Err(error) => vec![format!("default features: {error}")],
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#features")]
pub enum FeatureError {
    #[error(
        "unknown feature: {0} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#features; \
         fix: declare the feature in the [features] table or fix the name)"
    )]
    UnknownFeature(String),

    #[error(
        "private feature `{0}` cannot be activated by name \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#features; \
         fix: activate a public feature that pulls it in, or drop the `_` prefix)"
    )]
    PrivateFeature(String),

    #[error(
        "malformed feature activation `{0}` \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#features; \
         fix: use `feat`, `dep:name`, `name/feat`, `name?/feat`, or `subskill:path`)"
    )]
    Malformed(String),

    #[error(
        "exclusive group `{group}` violated — multiple features active: {active:?} \
         (violates spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#features; \
         fix: activate at most one feature from the group)"
    )]
    ExclusiveViolation { group: String, active: Vec<String> },
}

#[cfg(test)]
mod tests {
    use specmark::verifies;

    use super::*;

    fn make_table(toml_src: &str) -> FeaturesTable {
        toml::from_str(toml_src).unwrap()
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#features")]
    fn parses_feature_value_variants() {
        assert_eq!(
            FeatureValue::parse("foo").unwrap(),
            FeatureValue::Feature("foo".into())
        );
        assert_eq!(
            FeatureValue::parse("dep:bar").unwrap(),
            FeatureValue::Dep {
                dep_name: "bar".into()
            }
        );
        assert_eq!(
            FeatureValue::parse("baz/qux").unwrap(),
            FeatureValue::DepFeature {
                dep_name: "baz".into(),
                dep_feature: "qux".into(),
                weak: false,
            }
        );
        assert_eq!(
            FeatureValue::parse("baz?/qux").unwrap(),
            FeatureValue::DepFeature {
                dep_name: "baz".into(),
                dep_feature: "qux".into(),
                weak: true,
            }
        );
        assert_eq!(
            FeatureValue::parse("subskill:stack/rust").unwrap(),
            FeatureValue::Subskill {
                path: "stack/rust".into()
            }
        );
    }

    #[test]
    fn parse_rejects_empty() {
        assert!(FeatureValue::parse("").is_err());
        assert!(FeatureValue::parse("dep:").is_err());
        assert!(FeatureValue::parse("subskill:").is_err());
        assert!(FeatureValue::parse("foo/").is_err());
        assert!(FeatureValue::parse("/foo").is_err());
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#features")]
    fn defaults_activate_when_present() {
        let t = make_table(
            r#"
default = ["wal-protocol"]
wal-protocol = []
optional-x = []
"#,
        );
        let req = FeatureRequest::default();
        let exp = expand_features(&t, &req).unwrap();
        assert!(exp.active_features.contains("default"));
        assert!(exp.active_features.contains("wal-protocol"));
        assert!(!exp.active_features.contains("optional-x"));
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#features")]
    fn no_defaults_skips_default() {
        let t = make_table(
            r#"
default = ["wal-protocol"]
wal-protocol = []
"#,
        );
        let req = FeatureRequest {
            explicit: Vec::new(),
            no_defaults: true,
            all: false,
        };
        let exp = expand_features(&t, &req).unwrap();
        assert!(!exp.active_features.contains("default"));
        assert!(!exp.active_features.contains("wal-protocol"));
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#features")]
    fn all_features_skips_private() {
        let t = make_table(
            r#"
public-a = []
_internal-helper = []
"#,
        );
        let req = FeatureRequest {
            explicit: Vec::new(),
            no_defaults: true,
            all: true,
        };
        let exp = expand_features(&t, &req).unwrap();
        assert!(exp.active_features.contains("public-a"));
        assert!(!exp.active_features.contains("_internal-helper"));
    }

    #[test]
    fn explicit_unknown_feature_rejected() {
        let t = make_table(r#"x = []"#);
        let req = FeatureRequest {
            explicit: vec!["nope".into()],
            no_defaults: true,
            all: false,
        };
        let err = expand_features(&t, &req).unwrap_err();
        assert!(matches!(err, FeatureError::UnknownFeature(_)));
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#features")]
    fn private_feature_explicit_rejected() {
        let t = make_table(r#"_internal = []"#);
        let req = FeatureRequest {
            explicit: vec!["_internal".into()],
            no_defaults: true,
            all: false,
        };
        let err = expand_features(&t, &req).unwrap_err();
        assert!(matches!(err, FeatureError::PrivateFeature(_)));
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#features")]
    fn transitive_expansion() {
        let t = make_table(
            r#"
default = ["a"]
a = ["b", "c"]
b = ["dep:bar"]
c = ["d/feat", "subskill:stack/rust"]
d = []
"#,
        );
        let exp = expand_features(&t, &FeatureRequest::default()).unwrap();
        assert!(exp.active_features.contains("a"));
        assert!(exp.active_features.contains("b"));
        assert!(exp.active_features.contains("c"));
        assert!(exp.active_deps.contains("bar"));
        assert!(exp.active_deps.contains("d"));
        assert!(exp.dep_features.get("d").unwrap().contains("feat"));
        assert!(exp.active_subskills.contains("stack/rust"));
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#features")]
    fn weak_dep_feature_does_not_pull_dep() {
        let t = make_table(
            r#"
default = ["foo"]
foo = ["other?/some-feat"]
"#,
        );
        let exp = expand_features(&t, &FeatureRequest::default()).unwrap();
        assert!(!exp.active_deps.contains("other"));
        assert!(
            exp.weak_dep_features
                .get("other")
                .unwrap()
                .contains("some-feat")
        );
    }

    #[test]
    fn cycles_terminate() {
        let t = make_table(
            r#"
a = ["b"]
b = ["a"]
"#,
        );
        let req = FeatureRequest {
            explicit: vec!["a".into()],
            no_defaults: true,
            all: false,
        };
        let exp = expand_features(&t, &req).unwrap();
        assert!(exp.active_features.contains("a"));
        assert!(exp.active_features.contains("b"));
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#features")]
    fn exclusive_violation_detected() {
        let t = make_table(
            r#"
default = ["rust-stack", "python-stack"]
rust-stack = []
python-stack = []

[exclusive]
stacks = ["rust-stack", "python-stack"]
"#,
        );
        let err = expand_features(&t, &FeatureRequest::default()).unwrap_err();
        match err {
            FeatureError::ExclusiveViolation { group, active } => {
                assert_eq!(group, "stacks");
                assert_eq!(active.len(), 2);
            }
            other => panic!("wrong error: {other:?}"),
        }
    }

    #[test]
    fn exclusive_one_member_ok() {
        let t = make_table(
            r#"
default = ["rust-stack"]
rust-stack = []
python-stack = []

[exclusive]
stacks = ["rust-stack", "python-stack"]
"#,
        );
        let exp = expand_features(&t, &FeatureRequest::default()).unwrap();
        assert!(exp.active_features.contains("rust-stack"));
    }

    #[test]
    fn validate_finds_unknown_referenced_feature() {
        let t = make_table(r#"a = ["b"]"#);
        let findings = validate_features_table(&t);
        assert!(findings.iter().any(|f| f.contains("unknown feature")));
    }

    #[test]
    fn validate_passes_clean_table() {
        let t = make_table(
            r#"
default = ["a"]
a = []
b = []
"#,
        );
        assert!(validate_features_table(&t).is_empty());
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#features")]
    fn merge_extends_all_subsets() {
        let mut a = FeatureExpansion::default();
        a.active_features.insert("foo".into());
        a.active_deps.insert("bar".into());
        let mut b = FeatureExpansion::default();
        b.active_features.insert("baz".into());
        b.dep_features
            .entry("dep1".into())
            .or_default()
            .insert("feat-x".into());
        a.merge(&b);
        assert!(a.active_features.contains("foo"));
        assert!(a.active_features.contains("baz"));
        assert!(a.active_deps.contains("bar"));
        assert!(a.dep_features.get("dep1").unwrap().contains("feat-x"));
    }
}
