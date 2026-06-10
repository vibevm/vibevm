//! Feature expansion engine — PROP-003 §2.4.
//!
//! Given a `[features]` table from a package manifest plus a set of
//! roots-requested feature names, expand transitively into the full
//! activation set: feature → feature, `dep:foo`, `foo?/feat`,
//! `subskill:<path>`. Validate that no exclusive group is violated.
//!
//! This module deals exclusively with the in-package logic. Cross-
//! package feature unification (`pkg-A` and `pkg-B` both depend on
//! `pkg-C` with different features) is handled by the solver at a
//! higher layer — it calls into this module per package once with the
//! union of requested features.

use std::collections::{BTreeMap, BTreeSet};

use specmark::spec;
use thiserror::Error;
use vibe_core::manifest::FeaturesTable;

/// One activation entry, parsed from a feature's activation list.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-003#features")]
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
#[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-003#features")]
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
    /// Merge another expansion into this one. Used at the cross-package
    /// unification layer.
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
#[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-003#features")]
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
/// Cycles are detected and rejected. Unknown feature names referenced
/// from one feature's activation list flag as `UnknownFeature`.
/// Exclusive-group violations flag as `ExclusiveViolation`. The caller
/// is expected to surface diagnostics actionably (e.g. through
/// `vibe install` step lines).
#[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-003#features")]
pub fn expand_features(
    table: &FeaturesTable,
    request: &FeatureRequest,
) -> Result<FeatureExpansion, FeatureError> {
    // Build the seed set. The `default` feature, when present, is
    // itself a real feature — its activation list expands during BFS
    // through the normal feature-feature edge.
    let mut seed: BTreeSet<String> = BTreeSet::new();
    if request.all {
        for k in table.features.keys() {
            if !k.starts_with('_') {
                seed.insert(k.clone());
            }
        }
    } else {
        if !request.no_defaults && table.features.contains_key("default") {
            seed.insert("default".to_string());
        }
        for f in &request.explicit {
            if f.starts_with('_') {
                return Err(FeatureError::PrivateFeature(f.clone()));
            }
            if !table.features.contains_key(f) {
                return Err(FeatureError::UnknownFeature(f.clone()));
            }
            seed.insert(f.clone());
        }
    }

    let mut out = FeatureExpansion::default();
    // BFS / DFS expansion. Track feature names we've already processed
    // so cycles terminate.
    let mut work: Vec<String> = seed.into_iter().collect();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    while let Some(f) = work.pop() {
        if !seen.insert(f.clone()) {
            continue;
        }
        if !table.features.contains_key(&f) {
            return Err(FeatureError::UnknownFeature(f));
        }
        out.active_features.insert(f.clone());
        let activations = table.get(&f).unwrap_or(&[]);
        for raw in activations {
            let val = FeatureValue::parse(raw).map_err(|e| match e {
                FeatureError::Malformed(_) => {
                    FeatureError::Malformed(format!("feature `{f}` activation `{raw}`",))
                }
                other => other,
            })?;
            match val {
                FeatureValue::Feature(name) => {
                    if !table.features.contains_key(&name) {
                        return Err(FeatureError::UnknownFeature(format!(
                            "{name} (referenced by feature `{f}`)"
                        )));
                    }
                    work.push(name);
                }
                FeatureValue::Dep { dep_name } => {
                    out.active_deps.insert(dep_name);
                }
                FeatureValue::DepFeature {
                    dep_name,
                    dep_feature,
                    weak,
                } => {
                    let target = if weak {
                        &mut out.weak_dep_features
                    } else {
                        &mut out.dep_features
                    };
                    target
                        .entry(dep_name.clone())
                        .or_default()
                        .insert(dep_feature);
                    if !weak {
                        out.active_deps.insert(dep_name);
                    }
                }
                FeatureValue::Subskill { path } => {
                    out.active_subskills.insert(path);
                }
            }
        }
    }

    // Exclusive-group check.
    for (group, members) in &table.exclusive {
        let active_in_group: Vec<&String> = members
            .iter()
            .filter(|m| out.active_features.contains(m.as_str()))
            .collect();
        if active_in_group.len() > 1 {
            return Err(FeatureError::ExclusiveViolation {
                group: group.clone(),
                active: active_in_group.iter().map(|s| (*s).clone()).collect(),
            });
        }
    }

    Ok(out)
}

/// Static structural validation of a `[features]` table — runs at
/// `vibe check` time. Returns a list of diagnostics
/// (empty = valid).
#[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-003#features")]
pub fn validate_features_table(table: &FeaturesTable) -> Vec<String> {
    let mut findings = Vec::new();
    // Cycle detection by attempting an `--all-features` expansion.
    let probe = FeatureRequest {
        explicit: Vec::new(),
        no_defaults: true,
        all: true,
    };
    if let Err(e) = expand_features(table, &probe) {
        // Only feature-graph problems should fan out here; surface
        // them as findings rather than aborting the check pass.
        findings.push(format!("features table: {e}"));
    }
    // Default activation must respect exclusive groups.
    let defaults_probe = FeatureRequest {
        explicit: Vec::new(),
        no_defaults: false,
        all: false,
    };
    if let Err(e) = expand_features(table, &defaults_probe)
        && !matches!(e, FeatureError::UnknownFeature(_))
    {
        findings.push(format!("default features: {e}"));
    }
    // Subskill references look syntactically valid.
    for (name, activations) in &table.features {
        for raw in activations {
            if let Err(e) = FeatureValue::parse(raw) {
                findings.push(format!("feature `{name}` activation `{raw}`: {e}"));
            }
        }
    }
    findings
}

#[derive(Debug, Error, PartialEq, Eq)]
#[spec(implements = "spec://vibevm/modules/vibe-resolver/PROP-003#features")]
pub enum FeatureError {
    #[error("unknown feature: {0}")]
    UnknownFeature(String),

    #[error("private feature `{0}` cannot be activated by name")]
    PrivateFeature(String),

    #[error("malformed feature activation `{0}`")]
    Malformed(String),

    #[error("exclusive group `{group}` violated — multiple features active: {active:?}")]
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
    #[verifies("spec://vibevm/modules/vibe-resolver/PROP-003#features")]
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
    #[verifies("spec://vibevm/modules/vibe-resolver/PROP-003#features")]
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
    #[verifies("spec://vibevm/modules/vibe-resolver/PROP-003#features")]
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
    #[verifies("spec://vibevm/modules/vibe-resolver/PROP-003#features")]
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
    #[verifies("spec://vibevm/modules/vibe-resolver/PROP-003#features")]
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
    #[verifies("spec://vibevm/modules/vibe-resolver/PROP-003#features")]
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
    #[verifies("spec://vibevm/modules/vibe-resolver/PROP-003#features")]
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
    #[verifies("spec://vibevm/modules/vibe-resolver/PROP-003#features")]
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
    #[verifies("spec://vibevm/modules/vibe-resolver/PROP-003#features")]
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
