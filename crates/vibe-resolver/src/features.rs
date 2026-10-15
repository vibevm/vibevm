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
use vibe_core::manifest::feature_effects::{
    ConfigValue, DependencyDeclarationId, EffectError, FeatureDeclarations, FeatureEffect,
    FeatureProgram, FeatureStrength, LocalFeatureRef,
};

/// Compatibility view of a legacy activation string. Parsing delegates to
/// the core algebra; this view is never evaluated by the expansion engine.
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
        Ok(
            match FeatureEffect::parse_legacy(raw).map_err(FeatureError::from)? {
                FeatureEffect::Local(f) => Self::Feature(f.as_str().into()),
                FeatureEffect::ActivateDependency(d) => Self::Dep {
                    dep_name: d.as_str().into(),
                },
                FeatureEffect::TargetFeature {
                    dependency,
                    feature,
                    strength,
                } => Self::DepFeature {
                    dep_name: dependency.as_str().into(),
                    dep_feature: feature.as_str().into(),
                    weak: strength == FeatureStrength::Weak,
                },
                FeatureEffect::ExportedSubskill(path) => Self::Subskill { path },
                FeatureEffect::ConfigValue(_) => {
                    unreachable!("legacy grammar has no config syntax")
                }
            },
        )
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
    /// Values selected from package-local declarations.
    pub config_values: BTreeMap<String, ConfigValue>,
}

impl FeatureExpansion {
    /// Merge requests for the same owning package, atomically refusing config
    /// conflicts. Callers must keep expansions from distinct owners separate.
    pub fn merge(&mut self, other: &FeatureExpansion) -> Result<(), FeatureError> {
        for (key, value) in &other.config_values {
            if self
                .config_values
                .get(key)
                .is_some_and(|existing| existing != value)
            {
                return Err(FeatureError::ConfigConflict(key.clone()));
            }
        }
        self.config_values.extend(other.config_values.clone());
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
        self.apply_weak_requests();
        Ok(())
    }

    fn apply_weak_requests(&mut self) {
        for (dep, features) in &self.weak_dep_features {
            if self.active_deps.contains(dep) {
                self.dep_features
                    .entry(dep.clone())
                    .or_default()
                    .extend(features.iter().cloned());
            }
        }
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
/// Cycles **terminate silently**: an already-expanded feature is
/// skipped via the active-feature set, never reported as an error — a cycle
/// in a features table is redundant, not wrong (verified by the
/// `cycles_terminate` test; AUD-0014 corrected this line, which used
/// to claim cycles were "rejected"). Unknown feature names referenced
/// from one feature's activation list flag as `UnknownFeature`.
/// Exclusive-group violations flag as `ExclusiveViolation`. The caller
/// is expected to surface diagnostics actionably (e.g. through
/// `vibe install` step lines).
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#features")]
pub fn expand_features(
    table: &FeaturesTable,
    request: &FeatureRequest,
) -> Result<FeatureExpansion, FeatureError> {
    let program = FeatureProgram::lower_table(table)?;
    expand_program(
        &program,
        &FeatureDeclarations::default(),
        request,
        &BTreeSet::new(),
    )
}

/// Expand validated typed effects with package-local declarations and an
/// explicit set of already-active dependencies. Unknown IDs refuse.
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#M05B1-EFFECTS")]
pub fn expand_feature_effects(
    program: &FeatureProgram,
    declarations: &FeatureDeclarations,
    request: &FeatureRequest,
    already_active: &BTreeSet<DependencyDeclarationId>,
) -> Result<FeatureExpansion, FeatureError> {
    program.validate(declarations)?;
    for id in already_active {
        if !declarations.dependencies.contains_key(id) {
            return Err(EffectError::UnknownDependency(id.as_str().into()).into());
        }
    }
    expand_program(program, declarations, request, already_active)
}

/// Declaration-aware legacy entry: identical lowering, validation and engine.
pub fn expand_declared_features(
    table: &FeaturesTable,
    declarations: &FeatureDeclarations,
    request: &FeatureRequest,
    already_active: &BTreeSet<DependencyDeclarationId>,
) -> Result<FeatureExpansion, FeatureError> {
    let program = FeatureProgram::lower_legacy(table, declarations)?;
    expand_feature_effects(&program, declarations, request, already_active)
}

/// Expand a full legacy manifest using its declarations. Requires has no
/// optional wire field, so every declared dependency is a required seed.
/// No ambient installed packages are treated as this package's dependencies.
#[spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#M05B1-MANIFEST-CALLERS"
)]
pub fn expand_manifest_features(
    manifest: &vibe_core::manifest::Manifest,
    request: &FeatureRequest,
) -> Result<FeatureExpansion, FeatureError> {
    let declarations = FeatureDeclarations::from_requires(&manifest.requires, &BTreeSet::new())?;
    // Manifest has no authoritative subskill export catalogue. This lowering
    // validates known dependency context but leaves selectors to downstream
    // export validation, using the very same program and evaluator.
    let program = FeatureProgram::lower_legacy_dependencies(&manifest.features, &declarations)?;
    expand_program(&program, &declarations, request, &BTreeSet::new())
}

fn expand_program(
    program: &FeatureProgram,
    declarations: &FeatureDeclarations,
    request: &FeatureRequest,
    already_active: &BTreeSet<DependencyDeclarationId>,
) -> Result<FeatureExpansion, FeatureError> {
    let mut work = Vec::new();
    if request.all {
        work.extend(
            program
                .features
                .keys()
                .filter(|f| !f.as_str().starts_with('_'))
                .cloned(),
        );
    } else {
        let default = LocalFeatureRef::new("default")?;
        if !request.no_defaults && program.features.contains_key(&default) {
            work.push(default);
        }
        for f in &request.explicit {
            if f.starts_with('_') {
                return Err(FeatureError::PrivateFeature(f.clone()));
            }
            let local = LocalFeatureRef::new(f)?;
            if !program.features.contains_key(&local) {
                return Err(FeatureError::UnknownFeature(f.clone()));
            }
            work.push(local);
        }
    }
    let mut out = FeatureExpansion::default();
    out.active_deps
        .extend(already_active.iter().map(|id| id.as_str().to_string()));
    out.active_deps.extend(
        declarations
            .dependencies
            .iter()
            .filter(|(_, d)| !d.optional)
            .map(|(id, _)| id.as_str().to_string()),
    );
    while let Some(f) = work.pop() {
        if !out.active_features.insert(f.as_str().into()) {
            continue;
        }
        let effects = program
            .features
            .get(&f)
            .ok_or_else(|| FeatureError::UnknownFeature(f.as_str().into()))?;
        for effect in effects {
            match effect {
                FeatureEffect::Local(local) => work.push(local.clone()),
                FeatureEffect::ActivateDependency(id) => {
                    out.active_deps.insert(id.as_str().into());
                }
                FeatureEffect::TargetFeature {
                    dependency,
                    feature,
                    strength,
                } => {
                    let target = match strength {
                        FeatureStrength::Strong => {
                            out.active_deps.insert(dependency.as_str().into());
                            &mut out.dep_features
                        }
                        FeatureStrength::Weak => &mut out.weak_dep_features,
                    };
                    target
                        .entry(dependency.as_str().into())
                        .or_default()
                        .insert(feature.as_str().into());
                }
                FeatureEffect::ExportedSubskill(path) => {
                    out.active_subskills.insert(path.clone());
                }
                FeatureEffect::ConfigValue(key) => {
                    let value = declarations
                        .config_values
                        .get(key)
                        .ok_or_else(|| EffectError::UnknownConfig(key.as_str().into()))?;
                    out.config_values.insert(key.as_str().into(), value.clone());
                }
            }
        }
    }
    out.apply_weak_requests();

    // Exclusive-group check.
    for (group, members) in &program.exclusive {
        let active_in_group: Vec<&LocalFeatureRef> = members
            .iter()
            .filter(|m| out.active_features.contains(m.as_str()))
            .collect();
        if active_in_group.len() > 1 {
            return Err(FeatureError::ExclusiveViolation {
                group: group.clone(),
                active: active_in_group
                    .iter()
                    .map(|s| s.as_str().to_string())
                    .collect(),
            });
        }
    }

    Ok(out)
}

/// Static structural validation of a `[features]` table — runs at
/// `vibe check` time. Returns a list of diagnostics
/// (empty = valid).
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#features")]
pub fn validate_features_table(table: &FeaturesTable) -> Vec<String> {
    match FeatureProgram::lower_table(table) {
        Err(e) => vec![format!("features table: {e}")],
        Ok(program) => match expand_program(
            &program,
            &FeatureDeclarations::default(),
            &FeatureRequest::default(),
            &BTreeSet::new(),
        ) {
            Ok(_) => Vec::new(),
            Err(e) => vec![format!("default features: {e}")],
        },
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
#[spec(implements = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#features")]
pub enum FeatureError {
    #[error(transparent)]
    Declaration(EffectError),
    #[error("conflicting declared config value `{0}`")]
    ConfigConflict(String),
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

impl From<EffectError> for FeatureError {
    fn from(error: EffectError) -> Self {
        match error {
            EffectError::Malformed(s) => Self::Malformed(s),
            EffectError::UnknownFeature(s) => Self::UnknownFeature(s),
            other => Self::Declaration(other),
        }
    }
}

#[cfg(test)]
mod m_05_b_1_cases;

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
        a.merge(&b).unwrap();
        assert!(a.active_features.contains("foo"));
        assert!(a.active_features.contains("baz"));
        assert!(a.active_deps.contains("bar"));
        assert!(a.dep_features.get("dep1").unwrap().contains("feat-x"));
    }
}
