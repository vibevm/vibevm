//! Shared test-only admitted fixture used by service and transport regressions.
use super::discovery_api::{
    AnalysisCapture, ConfigurationAdmission, prepare_configuration_discovery,
};
use vibe_core::manifest::{feature_constraints::*, feature_effects::*};
use vibe_resolver::{
    features::configuration::ConfigurationDefinition,
    resolvo_engine::configuration::CandidateConfiguration,
};
pub fn local(s: &str) -> LocalFeatureRef {
    LocalFeatureRef::new(s).unwrap()
}
pub fn request() -> vibe_wire::generated::consumer_configuration::e1::request::Request {
    serde_json::from_value(serde_json::json!({"epoch":"1","edges":[{
        "consumer":"fixture-consumer", "dependency":"demo-edge", "role":"runtime",
        "domain":{"definition":{"reference":"fixture-definition","producer":"fixture-owner"},
            "target":{"reference":"fixture-target","producer":"fixture-owner"},
            "target_role":"executor","identity":"fixture-domain"},
        "instance":"fixture-instance","configuration":"fixture-configuration",
        "package":{"kind":"vibe","reference":"org.demo/demo"},
        "features":[],"all_features":false,"policy":"require_selection",
        "selections":{"mode":{"intent":"exact","provenance":"fixture-authored","members":["red"]}}
    }]}))
    .unwrap()
}
pub fn candidate(metadata: &vibe_registry::CandidateMetadata) -> CandidateConfiguration {
    let dep = DependencyDeclarationId::new("helper").unwrap();
    CandidateConfiguration {
        package: vibe_core::PackageRef::parse("org.demo/demo").unwrap(),
        version: semver::Version::new(1, 0, 0),
        identity: metadata.identity.clone(),
        definition: ConfigurationDefinition {
            instance: vibe_wire::behaviour::consumer_configuration::admit_request(&request())
                .unwrap()[0]
                .instance
                .clone(),
            program: FeatureProgram {
                features: [
                    (
                        local("red"),
                        [
                            FeatureEffect::ActivateDependency(dep.clone()),
                            FeatureEffect::ExportedSubskill("guide.md".into()),
                            FeatureEffect::ConfigValue(local("flag")),
                        ]
                        .into(),
                    ),
                    (local("blue"), Default::default()),
                    (local("verbose"), Default::default()),
                ]
                .into(),
                exclusive: Default::default(),
            },
            declarations: FeatureDeclarations {
                dependencies: [(
                    dep,
                    DependencyDeclaration {
                        source: DependencySource::Registry(
                            vibe_core::PackageRef::parse("org.demo/helper").unwrap(),
                        ),
                        optional: true,
                        link: None,
                        access: None,
                        friend: None,
                        exclude: None,
                    },
                )]
                .into(),
                exported_subskills: ["guide.md".into()].into(),
                config_values: [(local("flag"), ConfigValue::Bool(true))].into(),
                ..Default::default()
            },
            constraints: Constraints {
                groups: [
                    (
                        local("mode"),
                        ConstrainedGroup {
                            cardinality: Cardinality::ExactlyOne,
                            members: [local("red"), local("blue")].into(),
                            guards: vec![],
                            defaults: Some([local("red")].into()),
                        },
                    ),
                    (
                        local("detail"),
                        ConstrainedGroup {
                            cardinality: Cardinality::ZeroOrOne,
                            members: [local("verbose")].into(),
                            guards: vec![Guard::Member(local("blue"))],
                            defaults: None,
                        },
                    ),
                ]
                .into(),
                member_guards: Default::default(),
            },
        },
    }
}
pub fn fixture() -> (tempfile::TempDir, AnalysisCapture, ConfigurationAdmission) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname='consumer'\nversion='1.0.0'\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("vibe.lock"), "retained lock bytes").unwrap();
    std::fs::create_dir(dir.path().join("cache")).unwrap();
    let path = dir.path().join("cache/vibe.toml");
    std::fs::write(
        &path,
        "[package]\ngroup='org.demo'\nname='demo'\nkind='flow'\nversion='1.0.0'\n",
    )
    .unwrap();
    let metadata = vibe_registry::CandidateMetadata::read_local(
        &dir.path().join("cache"),
        "fixture:admitted-source".into(),
    )
    .unwrap();
    let capture = prepare_configuration_discovery(dir.path(), &["cache/vibe.toml".into()]).unwrap();
    let resource = capture
        .resources()
        .find(|(_, r)| r.path() == "cache/vibe.toml")
        .unwrap()
        .0;
    let admission = ConfigurationAdmission {
        resource,
        manifest_bytes: std::fs::read(&path).unwrap(),
        candidate: candidate(&metadata),
        metadata,
    };
    (dir, capture, admission)
}
