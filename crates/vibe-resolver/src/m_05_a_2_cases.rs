//! Bounded characterizations; defects are intentionally pinned, not repaired.
use super::*;
use specmark::verifies;
use vibe_core::manifest::FeaturesTable;

fn table(raw: &str) -> FeaturesTable {
    toml::from_str(raw).unwrap()
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#M05A2-SYNTAX")]
fn qualified_forwarding_first_slash_defect() {
    assert_eq!(
        FeatureValue::parse("dep:org.example/dep").unwrap(),
        FeatureValue::Dep {
            dep_name: "org.example/dep".into()
        }
    );
    for (raw, tail) in [
        ("org.example/dep/fast", "dep/fast"),
        ("org.example/dep?/fast", "dep?/fast"),
    ] {
        assert_eq!(
            FeatureValue::parse(raw).unwrap(),
            FeatureValue::DepFeature {
                dep_name: "org.example".into(),
                dep_feature: tail.into(),
                weak: false
            }
        );
        let t = table(&format!("default = [\"{raw}\"]"));
        let out = expand_features(&t, &FeatureRequest::default()).unwrap();
        assert_eq!(out.active_deps, ["org.example".into()].into());
        assert!(out.weak_dep_features.is_empty());
        assert_eq!(out.dep_features["org.example"], [tail.into()].into());
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#M05A2-SYNTAX")]
fn weak_forwarding_does_not_activate_dependency() {
    let weak = expand_features(
        &table("default = [\"dep?/fast\"]"),
        &FeatureRequest::default(),
    )
    .unwrap();
    assert!(weak.active_deps.is_empty());
    assert!(weak.dep_features.is_empty());
    assert_eq!(weak.weak_dep_features["dep"], ["fast".into()].into());
    let strong = expand_features(
        &table("default = [\"dep/fast\"]"),
        &FeatureRequest::default(),
    )
    .unwrap();
    assert_eq!(strong.active_deps, ["dep".into()].into());
    assert_eq!(strong.dep_features["dep"], ["fast".into()].into());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#M05A2-CONSTRAINTS")]
fn exclusive_at_most_one_and_all_features_validation_defect() {
    let t = table("a = []\nb = []\n[exclusive]\nchoice = [\"a\", \"b\"]");
    for explicit in [vec![], vec!["a".into()], vec!["b".into()]] {
        assert!(
            expand_features(
                &t,
                &FeatureRequest {
                    explicit,
                    no_defaults: true,
                    all: false
                }
            )
            .is_ok()
        );
    }
    assert!(
        matches!(expand_features(&t, &FeatureRequest { all: true, ..Default::default() }), Err(FeatureError::ExclusiveViolation { group, .. }) if group == "choice")
    );
    let findings = validate_features_table(&t);
    assert_eq!(findings.len(), 1);
    assert!(findings[0].contains("exclusive group `choice` violated"));
}

#[derive(Clone)]
struct World(Vec<Manifest>);
impl DepProvider for World {
    fn resolve_version(&self, r: &PackageRef) -> Result<semver::Version, DepProviderError> {
        let group = r.group.as_ref().unwrap();
        self.list_versions(group, &r.name)?
            .into_iter()
            .filter(|v| r.version.matches(v))
            .max()
            .ok_or_else(|| DepProviderError::NoMatchingVersion {
                group: group.clone(),
                name: r.name.to_string(),
                constraint: r.version.to_string(),
            })
    }
    fn fetch_manifest(
        &self,
        g: &Group,
        n: &str,
        v: &semver::Version,
    ) -> Result<Manifest, DepProviderError> {
        self.0
            .iter()
            .find(|m| {
                let p = m.require_package().unwrap();
                &p.group == g && p.name == n && &p.version == v
            })
            .cloned()
            .ok_or_else(|| DepProviderError::Other(format!("fixture missing {g}/{n}@{v}")))
    }
}
impl VersionEnumerator for World {
    fn list_versions(&self, g: &Group, n: &str) -> Result<Vec<semver::Version>, DepProviderError> {
        let versions: Vec<_> = self
            .0
            .iter()
            .filter_map(|m| {
                let p = m.require_package().unwrap();
                (&p.group == g && p.name == n).then(|| p.version.clone())
            })
            .collect();
        if versions.is_empty() {
            Err(DepProviderError::UnknownPackage {
                group: g.clone(),
                name: n.into(),
            })
        } else {
            Ok(versions)
        }
    }
}
fn pkg(n: &str, v: &str, body: &str) -> Manifest {
    Manifest::parse_str(&format!("[package]\ngroup = \"org.example\"\nname = \"{n}\"\nkind = \"flow\"\nversion = \"{v}\"\n{body}")).unwrap()
}
fn roots(names: &[&str]) -> Vec<PackageRef> {
    names
        .iter()
        .map(|n| PackageRef::parse(&format!("org.example/{n}")).unwrap())
        .collect()
}
fn solvers(w: World) -> Vec<Box<dyn DepSolver>> {
    vec![
        Box::new(NaiveDepSolver::new(w.clone())),
        Box::new(sat::SatDepSolver::new(w.clone())),
        Box::new(ResolvoDepSolver::new(w)),
    ]
}
fn has(g: &ResolvedGraph, n: &str) -> bool {
    g.find(&Group::parse("org.example").unwrap(), n).is_some()
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#M05A2-CONSTRAINTS")]
fn requires_any_allows_both_alternatives() {
    let w = World(vec![
        pkg(
            "a",
            "1.0.0",
            "[[requires_any]]\none_of = [\"org.example/b\", \"org.example/c\"]",
        ),
        pkg("b", "1.0.0", ""),
        pkg("c", "1.0.0", ""),
    ]);
    for solver in solvers(w) {
        let graph = solver.solve(&roots(&["b", "c", "a"])).unwrap();
        assert_eq!(graph.packages.len(), 3);
        assert!(has(&graph, "b") && has(&graph, "c"));
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#M05A2-BACKEND-MATRIX")]
fn absent_first_or_alternative_backend_gap() {
    let w = World(vec![
        pkg(
            "a",
            "1.0.0",
            "[[requires_any]]\none_of = [\"org.example/missing\", \"org.example/b\"]",
        ),
        pkg("b", "1.0.0", ""),
    ]);
    for solver in solvers(w.clone()).into_iter().take(2) {
        assert!(
            matches!(solver.solve(&roots(&["a"])), Err(SolveError::Provider(DepProviderError::UnknownPackage { name, .. })) if name == "missing")
        );
    }
    let graph = ResolvoDepSolver::new(w).solve(&roots(&["a"])).unwrap();
    assert!(has(&graph, "b"));
    assert!(!has(&graph, "missing"));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#M05A2-CONDITIONAL")]
fn wrong_version_name_present_disjunction_defect() {
    let w = World(vec![
        pkg(
            "a",
            "1.0.0",
            "[[requires_any]]\none_of = [\"org.example/b@^2\"]",
        ),
        pkg("b", "1.0.0", ""),
    ]);
    for solver in solvers(w.clone()).into_iter().take(2) {
        let graph = solver.solve(&roots(&["b", "a"])).unwrap();
        let selected = graph
            .find(&Group::parse("org.example").unwrap(), "b")
            .unwrap();
        assert!(
            !semver::VersionReq::parse("^2")
                .unwrap()
                .matches(&selected.version)
        );
    }
    assert!(matches!(
        ResolvoDepSolver::new(w).solve(&roots(&["b", "a"])),
        Err(SolveError::Unsatisfiable { .. })
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#M05A2-BACKEND-MATRIX")]
fn direct_routes_omit_features_and_conditional_targets() {
    let w = World(vec![
        pkg(
            "a",
            "1.0.0",
            "[features]\ndefault = [\"dep:org.example/b\"]\n[target.\"context(not flow:b)\".dependencies.packages]\n\"org.example/b\" = \"^2\"",
        ),
        pkg("b", "1.0.0", ""),
    ]);
    for solver in solvers(w) {
        let graph = solver.solve(&roots(&["a"])).unwrap();
        assert_eq!(graph.packages.len(), 1);
        assert!(!has(&graph, "b"));
        // Explicit submission is a control: the version constraint is enforced.
        assert!(solver.solve(&roots(&["a", "b@=1.0.0", "b@^2"])).is_err());
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#M05A2-CONDITIONAL")]
fn negated_self_trigger_changes_truth_without_iteration_claim() {
    use conditional::{ConditionalPredicate, PredicateError};
    let negative = ConditionalPredicate::parse("context(not flow:b)").unwrap();
    let positive = ConditionalPredicate::parse("context(flow:b)").unwrap();
    let mut ctx = ActivationContext::default();
    assert!(negative.evaluate(&ctx));
    ctx.add_present(CapabilityTag::parse("flow:b").unwrap());
    assert!(!negative.evaluate(&ctx));
    assert!(positive.evaluate(&ctx));
    // Presence has no version implication; an explicit version-looking key is literal.
    assert!(
        !ConditionalPredicate::parse("context(flow:b@^2)")
            .unwrap()
            .evaluate(&ctx)
    );
    assert!(matches!(
        ConditionalPredicate::parse("context(if_files = 'Cargo.toml')"),
        Err(PredicateError::Unsupported(_))
    ));
}
