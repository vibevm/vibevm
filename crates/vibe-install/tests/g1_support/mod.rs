//! Shared source-backed G1 fixtures; every caller uses the standard builder.
use super::package_source::{InstallResolver, PackageSourceOptions, build_install_resolver};
use std::path::{Path, PathBuf};
use vibe_core::{PackageRef, manifest::Manifest};
use vibe_resolver::resolvo_engine::request::{
    ConfigurationInput, NormalizedRequest, consumer_codec, consumer_wire,
};

// The standard source fetches through the process-local test cache. These
// fixtures deliberately reuse coordinates with different candidate bytes.
static FIXTURE_CACHE: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn p(name: &str) -> PackageRef {
    PackageRef::parse(&format!("org.g1/{name}")).unwrap()
}
pub struct Fixture {
    _cache_lease: std::sync::MutexGuard<'static, ()>,
    pub _temp: tempfile::TempDir,
    pub root: PathBuf,
    pub registry: PathBuf,
}
impl Fixture {
    pub fn new() -> Self {
        let lease = FIXTURE_CACHE.lock().unwrap_or_else(|e| e.into_inner());
        let isolated = vibe_test_support::isolated_home().expect("test-only settings isolation");
        let fixture_cache = isolated.join("cache/org.g1");
        if fixture_cache.exists() {
            std::fs::remove_dir_all(&fixture_cache).unwrap();
        }
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("project");
        let registry = temp.path().join("registry");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&registry).unwrap();
        std::fs::write(
            root.join("vibe.toml"),
            "[project]\nname='g1'\nversion='1.0.0'\n",
        )
        .unwrap();
        Self {
            _cache_lease: lease,
            _temp: temp,
            root,
            registry,
        }
    }
    pub fn package(&self, name: &str, version: u64, body: &str) {
        let dir = self.registry.join(format!("org.g1/{name}/v{version}.0.0"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("vibe.toml"), format!("[package]\ngroup='org.g1'\nname='{name}'\nkind='flow'\nversion='{version}.0.0'\n{body}")).unwrap();
    }
    pub fn legacy(&self) {
        self.package(
            "provider",
            1,
            "[provides]\ncapabilities=['capability:engine@1.0.0']\n",
        );
        self.package("app", 1, "[requires.packages]\n'org.g1/dep'='^1'\n[requires]\ncapabilities=['capability:engine@1.0.0']\n[[requires_any]]\none_of=['org.g1/provider','org.g1/other']\n[features]\ndefault=['helper','org.g1/dep?/trace']\nhelper=['dep:org.g1/dep']\n");
        self.package("dep", 1, "[features]\ntrace=[]\n");
        self.package("other", 1, "");
    }
    pub fn configured(&self) -> NormalizedRequest {
        self.legacy();
        self.package("blocker", 1, "");
        self.package("good", 1, "");
        self.package("bad", 1, "[conflicts]\npackages=['org.g1/blocker']\n");
        for (version, dependency) in [(1, "good"), (2, "bad")] {
            let members = if version == 1 { "['a','b']" } else { "['a']" };
            let b = if version == 1 { "b=[]\n" } else { "" };
            self.package("app", version, &format!(
                "[requires.packages]\n'org.g1/good'='*'\n'org.g1/bad'='*'\n[requires]\ncapabilities=['capability:engine@1.0.0']\n[[requires_any]]\none_of=['org.g1/provider','org.g1/other']\n[features]\nprofile='constrained-r1'\noptional=['org.g1/good','org.g1/bad']\n[features.effects]\na=[{{dependency='org.g1/{dependency}'}}]\n{b}[features.groups.choice]\ncardinality='zero-or-more'\nmembers={members}\n"
            ));
        }
        let mut wire = consumer_codec::decode_request(include_bytes!(
            "../../../../formats/corpora/consumer-configuration-request/e1/valid/omitted.json"
        ))
        .unwrap();
        wire.edges[0].package =
            consumer_wire::Package::Vibe(Box::new(consumer_wire::PackageVibe {
                reference: "org.g1/app".into(),
            }));
        wire.edges[0].features.clear();
        wire.edges[0].selections.get_mut("choice").unwrap().members = Some(vec!["a".into()]);
        let mut second = wire.edges[0].clone();
        second.consumer = "g1-second-consumer".into();
        second.selections.get_mut("choice").unwrap().members = Some(vec!["b".into()]);
        wire.edges.push(second);
        let mut request = NormalizedRequest::legacy(
            &[p("provider"), p("app"), p("blocker")],
            &Default::default(),
        );
        request.configuration = Some(ConfigurationInput {
            request: wire,
            definitions: vec![],
            defaults: vec![],
        });
        request
    }
    pub fn source(&self, backend: Option<&str>, mode: &str) -> InstallResolver {
        let mut options = PackageSourceOptions {
            solver: backend.map(str::to_owned),
            ..Default::default()
        };
        let mut manifest = Manifest::read(self.root.join("vibe.toml")).unwrap();
        let embedded: Option<&Path> = match mode {
            "local" => {
                options.registry = Some(self.registry.clone());
                None
            }
            "embedded" => Some(&self.registry),
            "multi" => {
                // A declared file registry exercises Multi without network or credentials.
                let declaration = Manifest::parse_str(&format!("[project]\nname='registry'\nversion='1.0.0'\n[[registry]]\nname='g1'\nurl='file://{}'\n", self.registry.display())).unwrap();
                manifest.registries = declaration.registries;
                None
            }
            _ => panic!("unknown fixture source mode"),
        };
        let source = build_install_resolver(
            &options,
            &manifest,
            embedded,
            &self.root,
            &Default::default(),
            true,
            &[],
        )
        .unwrap();
        assert!(matches!(
            (&source, mode),
            (InstallResolver::Local(..), "local")
                | (InstallResolver::Multi(..), "multi")
                | (InstallResolver::Embedded { .. }, "embedded")
        ));
        source
    }
}
