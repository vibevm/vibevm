use std::collections::{BTreeMap, BTreeSet};
use vibe_core::{
    PackageRef,
    manifest::{Lockfile, Manifest, SpecFormat, feature_constraints::*, feature_effects::*},
};
use vibe_install::{InstallRequest, InstallSource, NullObserver, Plan};
use vibe_package_source::{InstallResolver, PackageSourceOptions, build_install_resolver};
use vibe_resolver::{
    FeatureRequest,
    features::configuration::ConfigurationDefinition,
    resolvo_engine::{configuration::CandidateConfiguration, request::*},
};

pub struct Fixture {
    pub temp: tempfile::TempDir,
    pub source: InstallResolver,
    pub request: NormalizedRequest,
    pub root: std::path::PathBuf,
}
pub fn feature(n: &str) -> LocalFeatureRef {
    LocalFeatureRef::new(n).unwrap()
}
impl Fixture {
    pub fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let group = format!(
            "org.e2fixture.p{}.n{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("project");
        std::fs::create_dir(&root).unwrap();
        let registry = temp.path().join("registry");
        let package = registry.join(format!("{group}/app/v1.0.0"));
        std::fs::create_dir_all(&package).unwrap();
        std::fs::write(
            package.join("vibe.toml"),
            format!("[package]\ngroup='{group}'\nname='app'\nkind='flow'\nversion='1.0.0'\n"),
        )
        .unwrap();
        std::fs::write(package.join("payload.txt"), "exact source payload\n").unwrap();
        std::fs::write(root.join("vibe.toml"), format!("[project]\nname='e2fixture'\nversion='1.0.0'\n[requires.packages]\n'{group}/app'='*'\n")).unwrap();
        let manifest = Manifest::read(root.join("vibe.toml")).unwrap();
        let source = build_install_resolver(
            &PackageSourceOptions {
                registry: Some(registry),
                ..Default::default()
            },
            &manifest,
            None,
            &root,
            &vibe_core::GlobalRegistryConfig::default(),
            true,
            &[],
        )
        .unwrap();
        let metadata = vibe_registry::CandidateMetadata::read_local_registry(&package).unwrap();
        let p: PackageRef = format!("{group}/app").parse().unwrap();
        let mut wire = consumer_codec::decode_request(include_bytes!(
            "../../../../formats/corpora/consumer-configuration-request/e1/valid/omitted.json"
        ))
        .unwrap();
        wire.edges[0].package =
            consumer_wire::Package::Vibe(Box::new(consumer_wire::PackageVibe {
                reference: p.to_string(),
            }));
        wire.edges[0].features.clear();
        wire.edges[0].selections.clear();
        wire.edges[0].selections.insert(
            "choice".into(),
            consumer_wire::Selection {
                members: Some(vec!["a".into()]),
                intent: consumer_wire::SelectionIntent::Exact,
                provenance: "manifest:e2fixture".into(),
            },
        );
        wire.edges[0].selections.insert(
            "dormant".into(),
            consumer_wire::Selection {
                members: Some(vec!["later".into()]),
                intent: consumer_wire::SelectionIntent::Exact,
                provenance: "manifest:e2fixture".into(),
            },
        );
        let definition = ConfigurationDefinition {
            instance: consumer_codec::admit_request(&wire).unwrap()[0]
                .instance
                .clone(),
            program: FeatureProgram {
                features: [
                    (feature("a"), BTreeSet::new()),
                    (feature("b"), BTreeSet::new()),
                    (feature("later"), BTreeSet::new()),
                ]
                .into(),
                exclusive: BTreeMap::new(),
            },
            declarations: FeatureDeclarations::default(),
            constraints: Constraints {
                groups: [
                    (
                        feature("choice"),
                        ConstrainedGroup {
                            cardinality: Cardinality::ExactlyOne,
                            members: [feature("a"), feature("b")].into(),
                            guards: vec![],
                            defaults: Some([feature("a")].into()),
                        },
                    ),
                    (
                        feature("dormant"),
                        ConstrainedGroup {
                            cardinality: Cardinality::ZeroOrOne,
                            members: [feature("later")].into(),
                            guards: vec![Guard::Member(feature("b"))],
                            defaults: None,
                        },
                    ),
                ]
                .into(),
                member_guards: BTreeMap::new(),
            },
        };
        let mut request = NormalizedRequest::legacy(&[p.clone()], &FeatureRequest::default());
        request.configuration = Some(ConfigurationInput {
            request: wire,
            definitions: vec![CandidateConfiguration {
                package: p,
                version: semver::Version::new(1, 0, 0),
                identity: metadata.identity,
                definition,
            }],
            defaults: vec![],
        });
        Self {
            temp,
            source,
            request,
            root,
        }
    }
    pub fn install_request(&self) -> InstallRequest {
        InstallRequest {
            roots: vec![],
            features: Default::default(),
            language: None,
            exact: false,
            generated_by: "E2 fixture".into(),
            offline: true,
        }
    }
    pub fn installed(&self) -> (Lockfile, vibe_install::PlannedInstall) {
        let source = self.source.bind_request(self.request.clone()).unwrap();
        let Plan::Ready(plan) =
            vibe_install::plan(&source, &self.root, self.install_request(), &NullObserver).unwrap()
        else {
            panic!("ready")
        };
        let retained = plan.clone();
        vibe_install::apply(
            &source,
            *plan,
            vibe_core::user_config::SlotIntegrity::TrustPresence,
            &vibe_workspace::hooks::HookPolicy {
                allowed_groups: vec![],
                allow_hooks: false,
            },
        )
        .unwrap();
        (
            Lockfile::read(self.root.join("vibe.lock")).unwrap(),
            *retained,
        )
    }
    pub fn replay(
        &self,
        lock: &Lockfile,
        plan: &vibe_install::PlannedInstall,
    ) -> vibe_install::evolution::PreparedReinstall {
        let source = self.source.bind_request(self.request.clone()).unwrap();
        vibe_install::evolution::PreparedReinstall::from_planned(
            lock,
            &source,
            plan,
            source.catalog_inspection().unwrap(),
        )
        .unwrap()
    }
    pub fn update(&self) -> vibe_install::evolution::UpdatePlan {
        let source = self.source.bind_request(self.request.clone()).unwrap();
        let mut workspace = vibe_workspace::Workspace::discover(&self.root).unwrap();
        let mut manifest = Manifest::read(self.root.join("vibe.toml")).unwrap();
        vibe_install::evolution::plan_update(
            &source,
            &self.root,
            &mut manifest,
            &mut workspace,
            self.install_request(),
            SpecFormat::Mixed,
            &NullObserver,
        )
        .unwrap()
    }
}
