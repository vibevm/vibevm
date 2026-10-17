use super::configuration::*;
use super::*;
use crate::features::configuration::ConfigurationDefinition;
use crate::{DepProvider, FeatureExpansion};
use std::collections::{BTreeMap, BTreeSet};
use vibe_core::manifest::{Manifest, feature_constraints::*, feature_effects::*};
use vibe_wire::behaviour::consumer_configuration as codec;
use vibe_wire::generated::consumer_configuration::e1::request as wire;

fn p(name: &str) -> PackageRef {
    PackageRef::parse(&format!("org.example/{name}")).unwrap()
}
fn f(name: &str) -> LocalFeatureRef {
    LocalFeatureRef::new(name).unwrap()
}
fn id(name: &str) -> DependencyDeclarationId {
    DependencyDeclarationId::new(format!("org.example/{name}")).unwrap()
}
struct Source(BTreeMap<(String, semver::Version), vibe_registry::CandidateMetadata>);
impl DepProvider for Source {
    fn resolve_version(&self, package: &PackageRef) -> Result<semver::Version, DepProviderError> {
        Ok(self
            .list_versions(package.group.as_ref().unwrap(), package.name.as_str())?
            .into_iter()
            .filter(|v| package.version.matches(v))
            .max()
            .unwrap())
    }
    fn fetch_manifest(
        &self,
        group: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<Manifest, DepProviderError> {
        Ok(self.candidate_metadata(group, name, version)?.manifest)
    }
    fn candidate_metadata(
        &self,
        _: &Group,
        name: &str,
        version: &semver::Version,
    ) -> Result<vibe_registry::CandidateMetadata, DepProviderError> {
        Ok(self.0[&(name.into(), version.clone())].clone())
    }
}
impl VersionEnumerator for Source {
    fn list_versions(
        &self,
        group: &Group,
        name: &str,
    ) -> Result<Vec<semver::Version>, DepProviderError> {
        let versions: Vec<_> = self
            .0
            .keys()
            .filter(|(n, _)| n == name)
            .map(|(_, v)| v.clone())
            .collect();
        if versions.is_empty() {
            Err(DepProviderError::UnknownPackage {
                group: group.clone(),
                name: name.into(),
            })
        } else {
            Ok(versions)
        }
    }
}
pub(super) fn catalog() -> TransactionCatalog {
    let directory = tempfile::tempdir().unwrap();
    let mut source = Source(BTreeMap::new());
    for (name, version, body) in [
        (
            "app",
            1,
            "[requires.packages]\n'org.example/good'='*'\n'org.example/bad'='*'\n",
        ),
        (
            "app",
            2,
            "[requires.packages]\n'org.example/good'='*'\n'org.example/bad'='*'\n",
        ),
        (
            "good",
            1,
            "[provides]\ncapabilities=['capability:good@1.0.0']\n",
        ),
        (
            "bad",
            1,
            "[provides]\ncapabilities=['capability:bad@1.0.0']\n[conflicts]\npackages=['org.example/blocker']\n",
        ),
        ("blocker", 1, ""),
    ] {
        let path = directory.path().join(format!("{name}-{version}"));
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("vibe.toml"), format!("[package]\ngroup='org.example'\nname='{name}'\nkind='flow'\nversion='{version}.0.0'\n{body}")).unwrap();
        let metadata = vibe_registry::CandidateMetadata::read_local(
            &path,
            format!("file://{}", path.display()),
        )
        .unwrap();
        source
            .0
            .insert((name.into(), semver::Version::new(version, 0, 0)), metadata);
    }
    TransactionCatalog::capture(&source, &[p("app"), p("blocker")]).unwrap()
}
pub(super) fn request() -> wire::Request {
    let mut request = codec::decode_request(include_bytes!(
        "../../../../formats/corpora/consumer-configuration-request/e1/valid/omitted.json"
    ))
    .unwrap();
    request.edges[0].package = wire::Package::Vibe(Box::new(wire::PackageVibe {
        reference: "org.example/app".into(),
    }));
    request.edges[0].features = vec!["ordinary".into()];
    request.edges[0].selections.clear();
    request
}
pub(super) fn definitions(
    catalog: &TransactionCatalog,
    request: &wire::Request,
) -> Vec<CandidateConfiguration> {
    (1..=2)
        .map(|version| {
            let package = p("app");
            let version = semver::Version::new(version, 0, 0);
            let metadata = catalog
                .metadata(package.group.as_ref().unwrap(), "app", &version)
                .unwrap();
            let mut declarations = FeatureDeclarations::from_requires(
                &metadata.manifest.requires,
                &[id("good"), id("bad")].into(),
            )
            .unwrap();
            declarations.exported_subskills = ["selected.md".into(), "rejected.md".into()].into();
            declarations
                .config_values
                .insert(f("flag"), ConfigValue::Bool(version.major == 1));
            let target = if version.major == 1 { "good" } else { "bad" };
            let subskill = if version.major == 1 {
                "selected.md"
            } else {
                "rejected.md"
            };
            CandidateConfiguration {
                package,
                version,
                identity: metadata.identity.clone(),
                definition: ConfigurationDefinition {
                    instance: codec::admit_request(request).unwrap()[0].instance.clone(),
                    program: FeatureProgram {
                        features: [
                            (
                                f("ordinary"),
                                [
                                    FeatureEffect::ActivateDependency(id(target)),
                                    FeatureEffect::ExportedSubskill(subskill.into()),
                                    FeatureEffect::ConfigValue(f("flag")),
                                ]
                                .into(),
                            ),
                            (f("a"), BTreeSet::new()),
                            (f("b"), BTreeSet::new()),
                        ]
                        .into(),
                        exclusive: BTreeMap::new(),
                    },
                    declarations,
                    constraints: Constraints::default(),
                },
            }
        })
        .collect()
}
fn full_graph(graph: &ResolvedGraph) -> Vec<(String, bool, Vec<String>)> {
    graph
        .iter()
        .map(|n| {
            (
                format!("{}/{}@{}", n.group, n.name, n.version),
                n.is_root,
                n.dependencies.iter().map(ToString::to_string).collect(),
            )
        })
        .collect()
}
pub(super) fn assert_selected(result: &ConfiguredResolution, extra: &[&str]) {
    let mut expected = FeatureExpansion {
        active_features: ["ordinary".into()].into(),
        active_deps: ["org.example/good".into()].into(),
        active_subskills: ["selected.md".into()].into(),
        config_values: [("flag".into(), ConfigValue::Bool(true))].into(),
        ..Default::default()
    };
    expected
        .active_features
        .extend(extra.iter().map(|s| (*s).into()));
    assert_eq!(
        result.effects,
        [
            ("org.example/app@1.0.0".into(), expected),
            (
                "org.example/blocker@1.0.0".into(),
                FeatureExpansion::default()
            ),
            ("org.example/good@1.0.0".into(), FeatureExpansion::default()),
        ]
        .into()
    );
    assert_eq!(
        result.providers,
        [
            ("org.example/app@1.0.0".into(), vec![]),
            ("org.example/blocker@1.0.0".into(), vec![]),
            (
                "org.example/good@1.0.0".into(),
                vec!["capability:good@^1.0.0".into()]
            ),
        ]
        .into()
    );
    assert_eq!(
        full_graph(&result.graph),
        vec![
            (
                "org.example/app@1.0.0".into(),
                true,
                vec!["org.example/good@=1.0.0".into()]
            ),
            ("org.example/blocker@1.0.0".into(), true, vec![]),
            ("org.example/good@1.0.0".into(), false, vec![]),
        ]
    );
}

#[test]
fn compatible_older_candidate_keeps_only_selected_effects() {
    let catalog = catalog();
    let request = request();
    let definitions = definitions(&catalog, &request);
    let result = ResolvoDepSolver::new(&catalog)
        .solve_configuration(&[p("app"), p("blocker")], &request, &definitions, &[])
        .unwrap();
    assert_selected(&result, &[]);
    assert_eq!(result.request, request);
    let inspection = catalog.inspect().clone();
    let mut unsupported = self::definitions(&catalog, &request);
    unsupported[0]
        .definition
        .program
        .features
        .get_mut(&f("ordinary"))
        .unwrap()
        .insert(FeatureEffect::TargetFeature {
            dependency: id("good"),
            feature: f("forwarded"),
            strength: FeatureStrength::Strong,
        });
    assert!(matches!(
        ResolvoDepSolver::new(&catalog).solve_configuration(
            &[p("app")],
            &request,
            &unsupported,
            &[]
        ),
        Err(ConfigurationSolveError::Unsupported(_))
    ));
    assert_eq!(*catalog.inspect(), inspection);
    // Corrupt identity is operational even when that candidate could be rejected.
    let mut corrupt = definitions;
    corrupt[1].identity.definition.push_str("-corrupt");
    assert!(matches!(
        ResolvoDepSolver::new(&catalog).solve_configuration(
            &[p("app"), p("blocker")],
            &request,
            &corrupt,
            &[]
        ),
        Err(ConfigurationSolveError::Source(_))
    ));
}

#[test]
fn all_incoming_consumers_survive_candidate_rejection() {
    let catalog = catalog();
    let mut request = request();
    request.edges[0].features.push("a".into());
    let mut second = request.edges[0].clone();
    second.consumer = "consumer-b".into();
    second.features = vec!["b".into()];
    request.edges.push(second);
    let mut definitions = definitions(&catalog, &request);
    // Only the second consumer rejects newest; rebuilding from the first
    // consumer would incorrectly admit it or lose b in the final expansion.
    definitions[1].definition.program.features.remove(&f("b"));
    let result = ResolvoDepSolver::new(&catalog)
        .solve_configuration(&[p("app"), p("blocker")], &request, &definitions, &[])
        .unwrap();
    assert_selected(&result, &["a", "b"]);
    assert_eq!(result.request, request);
    assert_eq!(result.rejected[0].edges, vec![0, 1]);
}

fn mandatory(definition: &mut CandidateConfiguration, default: Option<&str>) {
    definition.definition.constraints.groups.insert(
        f("choice"),
        ConstrainedGroup {
            cardinality: Cardinality::ExactlyOne,
            members: [f("a"), f("b")].into(),
            guards: vec![],
            defaults: default.map(|s| [f(s)].into()),
        },
    );
}

#[test]
fn unreachable_mandatory_group_has_no_prompt() {
    let catalog = catalog();
    let request = request();
    let mut definitions = definitions(&catalog, &request);
    mandatory(&mut definitions[1], None);
    let result = ResolvoDepSolver::new(&catalog)
        .solve_configuration(
            &[p("app@=1.0.0"), p("blocker")],
            &request,
            &definitions,
            &[],
        )
        .unwrap();
    assert_selected(&result, &[]);
    let result = ResolvoDepSolver::new(&catalog)
        .solve_configuration(&[p("blocker")], &request, &definitions, &[])
        .unwrap();
    assert_eq!(
        full_graph(&result.graph),
        vec![("org.example/blocker@1.0.0".into(), true, vec![])]
    );
    assert_eq!(
        result.effects,
        [(
            "org.example/blocker@1.0.0".into(),
            FeatureExpansion::default()
        )]
        .into()
    );
    assert_eq!(
        result.providers,
        [("org.example/blocker@1.0.0".into(), vec![])].into()
    );
    assert!(matches!(
        ResolvoDepSolver::new(&catalog).solve_configuration(
            &[p("app@=2.0.0")],
            &request,
            &definitions,
            &[]
        ),
        Err(ConfigurationSolveError::NeedsSelection { .. })
    ));
    // Even a reachable newest candidate must not prompt after ordinary SAT
    // dependencies discard it. Mandatory groups are not discovery-time errors.
    definitions[1]
        .definition
        .declarations
        .dependencies
        .get_mut(&id("bad"))
        .unwrap()
        .optional = false;
    let result = ResolvoDepSolver::new(&catalog)
        .solve_configuration(&[p("app"), p("blocker")], &request, &definitions, &[])
        .unwrap();
    assert_selected(&result, &[]);
}

#[test]
fn changed_semantic_default_requires_rediscovery() {
    let catalog = catalog();
    let mut request = request();
    request.edges[0].policy = wire::EdgePolicy::UseDeclaredDefaults;
    let mut definitions = definitions(&catalog, &request);
    mandatory(&mut definitions[0], Some("a"));
    mandatory(&mut definitions[1], Some("b"));
    let binding = DefaultBinding::shown(0, &request, &definitions[1]);
    let error = ResolvoDepSolver::new(&catalog)
        .solve_configuration(
            &[p("app"), p("blocker")],
            &request,
            &definitions,
            &[binding],
        )
        .err()
        .unwrap();
    let ConfigurationSolveError::Rediscovery {
        request: original,
        candidates,
    } = error
    else {
        panic!("{error:?}")
    };
    assert_eq!(original, request);
    assert_eq!(candidates[0].coordinate, "org.example/app@1.0.0");
    let binding = DefaultBinding::shown(0, &request, &definitions[0]);
    let result = ResolvoDepSolver::new(&catalog)
        .solve_configuration(
            &[p("app@=1.0.0"), p("blocker")],
            &request,
            &definitions,
            &[binding.clone()],
        )
        .unwrap();
    assert_selected(&result, &["a"]);
    definitions[0]
        .definition
        .constraints
        .groups
        .get_mut(&f("choice"))
        .unwrap()
        .defaults = Some([f("b")].into());
    assert!(matches!(
        ResolvoDepSolver::new(&catalog).solve_configuration(
            &[p("app@=1.0.0")],
            &request,
            &definitions,
            &[binding]
        ),
        Err(ConfigurationSolveError::Rediscovery { .. })
    ));
}

#[test]
fn conflict_retains_consumer_and_candidate_reasons() {
    let catalog = catalog();
    let mut request = request();
    request.edges[0].selections.insert(
        "choice".into(),
        wire::Selection {
            intent: wire::SelectionIntent::Exact,
            provenance: "manifest:consumer-a".into(),
            members: Some(vec!["a".into()]),
        },
    );
    let mut second = request.edges[0].clone();
    second.consumer = "consumer-b".into();
    second.selections.get_mut("choice").unwrap().members = Some(vec!["b".into()]);
    second.selections.get_mut("choice").unwrap().provenance = "manifest:consumer-b".into();
    request.edges.push(second);
    let mut definitions = definitions(&catalog, &request);
    for d in &mut definitions {
        mandatory(d, None);
    }
    let error = ResolvoDepSolver::new(&catalog)
        .solve_configuration(&[p("app")], &request, &definitions, &[])
        .err()
        .unwrap();
    let ConfigurationSolveError::Conflict {
        request: original,
        candidates,
        solver,
    } = error
    else {
        panic!("{error:?}")
    };
    assert_eq!(original, request);
    assert_eq!(candidates.len(), 2);
    for (candidate, definition) in candidates.iter().zip(&definitions) {
        assert_eq!(candidate.identity, definition.identity);
        assert_eq!(candidate.edges, vec![0, 1]);
        assert!(
            candidate
                .reason
                .contains("conflicting consumer selections: [0, 1]")
        );
    }
    assert!(solver.to_string().contains("org.example/app"));
    assert!(solver.to_string().contains("consumer"));
}

#[test]
fn removed_optional_default_requires_rediscovery() {
    let catalog = catalog();
    let mut request = request();
    request.edges[0].policy = wire::EdgePolicy::UseDeclaredDefaults;
    let mut definitions = definitions(&catalog, &request);
    mandatory(&mut definitions[0], Some("a"));
    definitions[0]
        .definition
        .constraints
        .groups
        .get_mut(&f("choice"))
        .unwrap()
        .cardinality = Cardinality::ZeroOrOne;
    let binding = DefaultBinding::shown(0, &request, &definitions[0]);
    let result = ResolvoDepSolver::new(&catalog)
        .solve_configuration(
            &[p("app@=1.0.0"), p("blocker")],
            &request,
            &definitions,
            &[binding.clone()],
        )
        .unwrap();
    assert_selected(&result, &["a"]);
    definitions[0]
        .definition
        .constraints
        .groups
        .get_mut(&f("choice"))
        .unwrap()
        .defaults = None;
    let error = ResolvoDepSolver::new(&catalog)
        .solve_configuration(
            &[p("app@=1.0.0"), p("blocker")],
            &request,
            &definitions,
            &[binding.clone()],
        )
        .err()
        .expect("removed shown default must require rediscovery");
    let ConfigurationSolveError::Rediscovery {
        request: original,
        candidates,
    } = error
    else {
        panic!("{error:?}")
    };
    assert_eq!(original, request);
    assert_eq!(candidates[0].identity, definitions[0].identity);
    assert_eq!(candidates[0].coordinate, "org.example/app@1.0.0");
    request.edges[0].selections.insert(
        "choice".into(),
        wire::Selection {
            intent: wire::SelectionIntent::Exact,
            provenance: "manifest:consumer-a".into(),
            members: Some(vec!["a".into()]),
        },
    );
    let result = ResolvoDepSolver::new(&catalog)
        .solve_configuration(
            &[p("app@=1.0.0"), p("blocker")],
            &request,
            &definitions,
            &[binding],
        )
        .unwrap();
    assert_selected(&result, &["a"]);
    assert_eq!(result.request, request);
}

#[test]
fn unresolved_group_keeps_known_ordinary_implications() {
    let catalog = catalog();
    let request = request();
    let mut definitions = definitions(&catalog, &request);
    mandatory(&mut definitions[1], None);
    assert!(definitions[1].definition.declarations.dependencies[&id("bad")].optional);
    let result = ResolvoDepSolver::new(&catalog)
        .solve_configuration(&[p("app"), p("blocker")], &request, &definitions, &[])
        .unwrap();
    assert_selected(&result, &[]);
    assert_eq!(result.request, request);
}
