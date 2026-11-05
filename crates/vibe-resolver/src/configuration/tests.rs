//! Actual resolvo candidate selection, distinct from configuration projection tests.
use super::*;

struct Sources(BTreeMap<(String, semver::Version), Manifest>);
impl DepProvider for Sources {
    fn resolve_version(&self, p: &PackageRef) -> Result<semver::Version, DepProviderError> {
        self.list_versions(p.group.as_ref().unwrap(), p.name.as_str())?
            .into_iter()
            .filter(|v| p.version.matches(v))
            .max()
            .ok_or_else(|| DepProviderError::Other("missing test version".into()))
    }
    fn fetch_manifest(
        &self,
        g: &Group,
        n: &str,
        v: &semver::Version,
    ) -> Result<Manifest, DepProviderError> {
        self.0
            .get(&(format!("{g}/{n}"), v.clone()))
            .cloned()
            .ok_or_else(|| DepProviderError::UnknownPackage {
                group: g.clone(),
                name: n.into(),
            })
    }
}
impl VersionEnumerator for Sources {
    fn list_versions(&self, g: &Group, n: &str) -> Result<Vec<semver::Version>, DepProviderError> {
        Ok(self
            .0
            .keys()
            .filter(|(name, _)| name == &format!("{g}/{n}"))
            .map(|(_, v)| v.clone())
            .collect())
    }
}
fn manifest(name: &str, version: &str, body: &str) -> Manifest {
    Manifest::parse_str(&format!(
        "[package]\ngroup='org.test'\nname='{name}'\nversion='{version}'\nkind='flow'\n{body}"
    ))
    .unwrap()
}
fn sources(manifests: Vec<Manifest>) -> Sources {
    Sources(
        manifests
            .into_iter()
            .map(|m| {
                let p = m.package.as_ref().unwrap();
                ((format!("{}/{}", p.group, p.name), p.version.clone()), m)
            })
            .collect(),
    )
}
fn snapshot(input: &str) -> ConfigurationSnapshot {
    ConfigurationSnapshot::new(BTreeMap::from([(
        "org.test/owner".into(),
        BTreeMap::from([(
            "./vibe.toml".into(),
            Some(
                toml::from_str(&format!(
                    "policy='require-selection'\ndefault_features=false\n{input}"
                ))
                .unwrap(),
            ),
        )]),
    )]))
}
fn roots() -> Vec<PackageRef> {
    vec![PackageRef::parse("org.test/owner@^1").unwrap()]
}
fn v(s: &str) -> semver::Version {
    semver::Version::parse(s).unwrap()
}
fn question() -> &'static str {
    "[[configuration.features]]\nid='pick'\nname='Pick'\n[[configuration.groups]]\nmode='choice'\nid='mode'\nname='Mode'\ncardinality='exactly-one'\nmembers=['pick']"
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#DEFAULT-CONSENT")]
fn newest_feasible_unanswered_candidate_does_not_choose_older_question_free_definition() {
    let source = sources(vec![
        manifest("owner", "1.0.0", "[configuration]"),
        manifest("owner", "1.1.0", question()),
    ]);
    let error = snapshot("")
        .solve(source, &roots(), &BTreeSet::new())
        .unwrap_err();
    let SolveError::Provider(DepProviderError::Configuration(
        PackageConfigurationError::NeedsSelection { questions, .. },
    )) = error
    else {
        panic!("wrong error {error:?}");
    };
    assert_eq!(questions[0].group, "mode");
    assert_eq!(questions[0].consumers, ["./vibe.toml"]);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#CANDIDATE-BOUNDARY")]
fn dependency_unsatisfiable_newest_can_backtrack_without_retaining_its_questions() {
    let source = sources(vec![
        manifest("owner", "1.0.0", "[configuration]"),
        manifest(
            "owner",
            "1.1.0",
            &format!(
                "[requires.packages]\n'org.test/missing'='^1'\n{}",
                question()
            ),
        ),
    ]);
    let snapshot = snapshot("");
    let graph = snapshot.solve(source, &roots(), &BTreeSet::new()).unwrap();
    assert_eq!(graph.packages.len(), 1);
    assert_eq!(graph.packages[0].version, v("1.0.0"));
    assert!(
        snapshot
            .configuration(&Group::parse("org.test").unwrap(), "owner", &v("1.0.0"))
            .unwrap()
            .unwrap()
            .features
            .is_empty()
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#CANDIDATE-BOUNDARY")]
fn explicit_feature_incompatible_candidate_can_backtrack_without_changing_input() {
    let source = sources(vec![
        manifest(
            "owner",
            "1.0.0",
            "[[configuration.features]]\nid='stable'\nname='Stable'",
        ),
        manifest(
            "owner",
            "1.1.0",
            "[[configuration.features]]\nid='other'\nname='Other'",
        ),
    ]);
    let snapshot = snapshot("features=['stable']");
    let graph = snapshot.solve(source, &roots(), &BTreeSet::new()).unwrap();
    assert_eq!(graph.packages[0].version, v("1.0.0"));
    let record = snapshot
        .configuration(&Group::parse("org.test").unwrap(), "owner", &v("1.0.0"))
        .unwrap()
        .unwrap();
    assert_eq!(record.consumers["./vibe.toml"].features, ["stable"]);
    assert_eq!(record.features, ["stable"]);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#DEFINITION")]
fn malformed_newest_definition_is_not_candidate_incompatibility() {
    let mut broken = manifest(
        "owner",
        "1.1.0",
        "[[configuration.features]]\nid='stable'\nname='Stable'",
    );
    broken.configuration.as_mut().unwrap().features[0]
        .requires
        .push("missing".into());
    let source = sources(vec![
        manifest(
            "owner",
            "1.0.0",
            "[[configuration.features]]\nid='stable'\nname='Stable'",
        ),
        broken,
    ]);
    let error = snapshot("features=['stable']")
        .solve(source, &roots(), &BTreeSet::new())
        .unwrap_err();
    assert!(
        matches!(
            error,
            SolveError::Provider(DepProviderError::Configuration(
                PackageConfigurationError::Invalid { .. }
            ))
        ),
        "{error:?}"
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#DORMANT-SOURCE-BOUNDARY"
)]
fn optional_off_has_no_foreign_read_or_selected_edge_while_mandatory_remains() {
    let owner = manifest(
        "owner",
        "1.0.0",
        "[requires.packages]\n'org.test/base'='^1'\n'org.test/extra'={version='^1',optional=true}\n[[configuration.features]]\nid='on'\nname='On'\neffects=[{kind='dependency',target='org.test/extra'}]\n[[configuration.features]]\nid='mention-base'\nname='Mention base'\neffects=[{kind='dependency',target='org.test/base'}]",
    );
    let base = manifest("base", "1.0.0", "");
    let snapshot = snapshot("");
    let graph = snapshot
        .solve(sources(vec![owner, base]), &roots(), &BTreeSet::new())
        .unwrap();
    assert_eq!(graph.packages.len(), 2);
    assert!(
        !snapshot
            .versions
            .borrow()
            .keys()
            .any(|(_, name)| name == "extra")
    );
    let owner = graph
        .find(&Group::parse("org.test").unwrap(), "owner")
        .unwrap();
    assert_eq!(
        owner
            .dependencies
            .iter()
            .map(|p| p.qualified_name())
            .collect::<Vec<_>>(),
        ["org.test/base"]
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#CANDIDATE-BOUNDARY")]
fn rejected_optional_activation_leaves_only_selected_candidate_graph_and_effects() {
    let owner = |version: &str, target: &str| {
        manifest(
            "owner",
            version,
            &format!(
                "[requires.packages]\n'org.test/{target}'={{version='^1',optional=true}}\n[[configuration.features]]\nid='enable'\nname='Enable'\neffects=[{{kind='dependency',target='org.test/{target}'}}]"
            ),
        )
    };
    let source = sources(vec![
        owner("1.1.0", "absent"),
        owner("1.0.0", "leaf"),
        manifest("leaf", "1.0.0", ""),
    ]);
    let snapshot = snapshot("features=['enable']");
    let graph = snapshot.solve(source, &roots(), &BTreeSet::new()).unwrap();
    assert_eq!(graph.packages.len(), 2);
    let selected = graph
        .find(&Group::parse("org.test").unwrap(), "owner")
        .unwrap();
    assert_eq!(selected.version, v("1.0.0"));
    assert_eq!(
        selected
            .dependencies
            .iter()
            .map(|p| p.qualified_name())
            .collect::<Vec<_>>(),
        ["org.test/leaf"]
    );
    let record = snapshot
        .configuration(&selected.group, &selected.name, &selected.version)
        .unwrap()
        .unwrap();
    assert_eq!(record.active_dependencies, ["org.test/leaf"]);
    assert_eq!(
        record.dependency_causes,
        BTreeMap::from([("org.test/leaf".into(), vec!["enable".into()])])
    );
    assert_eq!(record.consumers["./vibe.toml"].features, ["enable"]);
    assert!(!record.declarations.optional.contains_key("org.test/absent"));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#CANDIDATE-BOUNDARY")]
fn unsupported_optional_recommendation_does_not_poison_selected_result() {
    let source = sources(vec![
        manifest(
            "owner",
            "1.0.0",
            "[recommends]\npackages=['org.test/question@^1']\n[configuration]",
        ),
        manifest("question", "1.0.0", question()),
    ]);
    let snapshot = snapshot("");
    let graph = snapshot.solve(source, &roots(), &BTreeSet::new()).unwrap();
    assert_eq!(graph.packages.len(), 1);
    assert_eq!(graph.packages[0].name, "owner");
    assert!(
        snapshot
            .configuration(&Group::parse("org.test").unwrap(), "owner", &v("1.0.0"))
            .unwrap()
            .is_some()
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#CANDIDATE-BOUNDARY")]
fn failed_metadata_observation_is_not_replaced_after_prescan() {
    struct Changing<'a> {
        reads: &'a std::cell::Cell<usize>,
    }
    impl DepProvider for Changing<'_> {
        fn resolve_version(&self, _: &PackageRef) -> Result<semver::Version, DepProviderError> {
            Ok(v("1.0.0"))
        }
        fn fetch_manifest(
            &self,
            _: &Group,
            _: &str,
            _: &semver::Version,
        ) -> Result<Manifest, DepProviderError> {
            let count = self.reads.get();
            self.reads.set(count + 1);
            if count == 0 {
                Err(DepProviderError::Other(
                    "first captured metadata failed".into(),
                ))
            } else {
                Ok(manifest("owner", "1.0.0", "[configuration]"))
            }
        }
    }
    impl VersionEnumerator for Changing<'_> {
        fn list_versions(
            &self,
            _: &Group,
            _: &str,
        ) -> Result<Vec<semver::Version>, DepProviderError> {
            Ok(vec![v("1.0.0")])
        }
    }
    let reads = std::cell::Cell::new(0);
    let error = snapshot("")
        .solve(Changing { reads: &reads }, &roots(), &BTreeSet::new())
        .unwrap_err();
    assert!(
        error.to_string().contains("first captured metadata failed"),
        "{error}"
    );
    assert_eq!(
        reads.get(),
        1,
        "one operation cannot replace its observed source failure"
    );
}
