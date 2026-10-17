use super::*;
use crate::{InstallRequest, InstallSource, NullObserver, Plan};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};
use vibe_core::manifest::{feature_constraints::*, feature_effects::*};
use vibe_registry::{
    CachedPackage, CandidateMetadata, InPlaceMaterialised, RegistryError, ResolvedPackage,
};
use vibe_resolver::resolvo_engine::{
    CatalogInspection, RecordedResolution, TransactionCatalog,
    configuration::CandidateConfiguration,
    request::{ConfigurationInput, NormalizedRequest, RouteResolution},
};
use vibe_resolver::{
    DepProvider, DepProviderError, FeatureRequest, ResolvedGraph, SolveError, VersionEnumerator,
};

fn p(n: &str) -> PackageRef {
    format!("org.example/{n}").parse().unwrap()
}
fn f(n: &str) -> LocalFeatureRef {
    LocalFeatureRef::new(n).unwrap()
}
struct Provider(PathBuf);
impl DepProvider for Provider {
    fn resolve_version(&self, _: &PackageRef) -> Result<semver::Version, DepProviderError> {
        Ok(semver::Version::new(1, 0, 0))
    }
    fn fetch_manifest(
        &self,
        g: &vibe_core::Group,
        n: &str,
        v: &semver::Version,
    ) -> Result<Manifest, DepProviderError> {
        Ok(self.candidate_metadata(g, n, v)?.manifest)
    }
    fn candidate_metadata(
        &self,
        _: &vibe_core::Group,
        n: &str,
        _: &semver::Version,
    ) -> Result<CandidateMetadata, DepProviderError> {
        CandidateMetadata::read_local_registry(&self.0.join(n))
            .map_err(|e| DepProviderError::Other(e.to_string()))
    }
}
impl VersionEnumerator for Provider {
    fn list_versions(
        &self,
        _: &vibe_core::Group,
        _: &str,
    ) -> Result<Vec<semver::Version>, DepProviderError> {
        Ok(vec![semver::Version::new(1, 0, 0)])
    }
}
struct Source {
    provider: Provider,
    intent: Option<NormalizedRequest>,
    route: RouteResolution,
    catalog: CatalogInspection,
}
impl InstallSource for Source {
    fn normalized_request(&self) -> Option<&NormalizedRequest> {
        self.intent.as_ref()
    }
    fn route_resolution(&self) -> Option<RouteResolution> {
        Some(self.route.clone())
    }
    fn catalog_inspection(&self) -> Option<&CatalogInspection> {
        Some(&self.catalog)
    }
    fn solve(&self, _: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        Ok(self.route.recorded.graph.clone())
    }
    fn solve_masked(
        &self,
        r: &[PackageRef],
        _: &BTreeSet<(String, String)>,
    ) -> Result<ResolvedGraph, SolveError> {
        self.solve(r)
    }
    fn manifest_of(&self, p: &PackageRef) -> Result<Manifest, SolveError> {
        Ok(self.route.manifests[&format!("{}@1.0.0", p.qualified_name())].clone())
    }
    fn catalog_manifest_of(&self, p: &PackageRef) -> Result<Manifest, SolveError> {
        Ok(self.provider.fetch_manifest(
            p.group.as_ref().unwrap(),
            p.name.as_str(),
            &semver::Version::new(1, 0, 0),
        )?)
    }
    fn resolve_and_fetch(
        &self,
        p: &PackageRef,
        _: &Path,
        _: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        let dir = self.provider.0.join(p.name.as_str());
        let manifest = Manifest::read(dir.join("vibe.toml")).unwrap();
        Ok(CachedPackage {
            resolved: ResolvedPackage {
                group: p.group.clone().unwrap(),
                name: p.name.to_string(),
                version: semver::Version::new(1, 0, 0),
                source_dir: dir.clone(),
            },
            cache_dir: dir.clone(),
            manifest,
            content_hash: vibe_registry::compute_content_hash(&dir)?,
            source_uri: format!("file://{}", dir.display()),
            registry_name: None,
            source_ref: None,
            resolved_commit: None,
            overridden: false,
            is_git_source: false,
            is_path_source: false,
            is_embedded: false,
            is_local: true,
            via_redirect: None,
        })
    }
    fn materialise_in_place(
        &self,
        _: &PackageRef,
        _: &Path,
    ) -> Result<InPlaceMaterialised, RegistryError> {
        panic!("copy fixture")
    }
}
struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    source: Source,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("project");
        let registry = temp.path().join("registry");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("vibe.toml"),"[project]\nname='fixture'\ngroup='org.example'\nversion='1.0.0'\n[requires]\npackages={'org.example/owner'='*','org.example/provider'='*'}\n").unwrap();
        for (n, tail) in [
            ("dep", ""),
            (
                "owner",
                "[requires]\npackages={'org.example/dep'='*'}\ncapabilities=['capability:fixture']\n",
            ),
            (
                "provider",
                "[provides]\ncapabilities=['capability:fixture']\n",
            ),
        ] {
            let dir = registry.join(n);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("vibe.toml"),format!("[package]\ngroup='org.example'\nname='{n}'\nkind='flow'\nversion='1.0.0'\n{tail}")).unwrap();
        }
        let provider = Provider(registry);
        let mut wire = serde_json::json!({"epoch":"1","edges":[{"consumer":"project:fixture","dependency":"org.example/owner","role":"runtime","domain":{"definition":{"reference":"env","producer":"fixture"},"target":{"reference":"host","producer":"fixture"},"target_role":"executor","identity":"domain"},"instance":"owner","configuration":"owner-choice","package":{"kind":"vibe","reference":"org.example/owner"},"policy":"require_selection","selections":{"choice":{"members":["a"],"intent":"exact","provenance":"manifest:fixture"},"dormant":{"members":["later"],"intent":"exact","provenance":"manifest:fixture"}},"features":[],"all_features":false}]});
        let request = vibe_wire::behaviour::consumer_configuration::decode_request(
            &serde_json::to_vec(&wire).unwrap(),
        )
        .unwrap();
        let instance = vibe_wire::behaviour::consumer_configuration::admit_request(&request)
            .unwrap()[0]
            .instance
            .clone();
        let declarations = FeatureDeclarations::from_requires(
            &provider
                .fetch_manifest(
                    &p("owner").group.unwrap(),
                    "owner",
                    &semver::Version::new(1, 0, 0),
                )
                .unwrap()
                .requires,
            &BTreeSet::new(),
        )
        .unwrap();
        let definition = vibe_resolver::features::configuration::ConfigurationDefinition {
            instance,
            declarations,
            program: FeatureProgram {
                features: [
                    (f("a"), [FeatureEffect::Local(f("closure"))].into()),
                    (f("closure"), BTreeSet::new()),
                    (f("later"), BTreeSet::new()),
                    (f("gate"), BTreeSet::new()),
                ]
                .into(),
                exclusive: BTreeMap::new(),
            },
            constraints: Constraints {
                groups: [
                    (
                        f("choice"),
                        ConstrainedGroup {
                            cardinality: Cardinality::ExactlyOne,
                            members: [f("a"), f("gate")].into(),
                            guards: vec![],
                            defaults: None,
                        },
                    ),
                    (
                        f("dormant"),
                        ConstrainedGroup {
                            cardinality: Cardinality::ZeroOrOne,
                            members: [f("later")].into(),
                            guards: vec![Guard::Member(f("gate"))],
                            defaults: None,
                        },
                    ),
                ]
                .into(),
                member_guards: BTreeMap::new(),
            },
        };
        let identity = provider
            .candidate_metadata(
                &p("owner").group.unwrap(),
                "owner",
                &semver::Version::new(1, 0, 0),
            )
            .unwrap()
            .identity;
        let candidate = CandidateConfiguration {
            package: p("owner"),
            version: semver::Version::new(1, 0, 0),
            identity,
            definition,
        };
        let mut intent =
            NormalizedRequest::legacy(&[p("owner"), p("provider")], &FeatureRequest::default());
        intent.configuration = Some(ConfigurationInput {
            request,
            definitions: vec![candidate],
            defaults: vec![],
        });
        let catalog = TransactionCatalog::capture(&provider, &intent.catalog_roots()).unwrap();
        let c = intent.configuration.as_ref().unwrap();
        let solved = vibe_resolver::ResolvoDepSolver::new(&catalog)
            .solve_configuration(&intent.roots, &c.request, &c.definitions, &c.defaults)
            .unwrap();
        let route = RouteResolution::bind(
            &catalog,
            &intent,
            RecordedResolution {
                graph: solved.graph,
                choices: solved.choices,
            },
            solved.manifests,
            solved.effects,
        )
        .unwrap();
        let inspection = catalog.inspect().clone();
        // Keep the fixture's request value live independently of the saved lock.
        wire["epoch"] = serde_json::json!("1");
        Self {
            _temp: temp,
            root,
            source: Source {
                provider,
                intent: Some(intent),
                route,
                catalog: inspection,
            },
        }
    }
    fn with_defaults(mut self) -> Self {
        use vibe_resolver::resolvo_engine::configuration::DefaultBinding;
        use vibe_resolver::resolvo_engine::request::consumer_wire::EdgePolicy;
        let intent = self.source.intent.as_mut().unwrap();
        let c = intent.configuration.as_mut().unwrap();
        c.request.edges[0]
            .selections
            .get_mut("choice")
            .unwrap()
            .members = None;
        c.request.edges[0].policy = EdgePolicy::UseDeclaredDefaults;
        c.definitions[0]
            .definition
            .constraints
            .groups
            .get_mut(&f("choice"))
            .unwrap()
            .defaults = Some([f("a")].into());
        c.defaults = vec![DefaultBinding::shown(0, &c.request, &c.definitions[0])];
        let catalog =
            TransactionCatalog::capture(&self.source.provider, &intent.catalog_roots()).unwrap();
        let c = intent.configuration.as_ref().unwrap();
        let solved = vibe_resolver::ResolvoDepSolver::new(&catalog)
            .solve_configuration(&intent.roots, &c.request, &c.definitions, &c.defaults)
            .unwrap();
        self.source.route = RouteResolution::bind(
            &catalog,
            intent,
            RecordedResolution {
                graph: solved.graph,
                choices: solved.choices,
            },
            solved.manifests,
            solved.effects,
        )
        .unwrap();
        self
    }
    fn request(&self) -> InstallRequest {
        InstallRequest {
            roots: vec![],
            features: FeatureRequest::default(),
            language: None,
            exact: false,
            generated_by: "D1 test".into(),
            offline: true,
        }
    }
    fn apply(&self) -> Lockfile {
        let plan = crate::plan(&self.source, &self.root, self.request(), &NullObserver).unwrap();
        let Plan::Ready(planned) = plan else {
            panic!("expected admitted resolution")
        };
        crate::apply(
            &self.source,
            *planned,
            vibe_core::user_config::SlotIntegrity::TrustPresence,
            &vibe_workspace::hooks::HookPolicy {
                allowed_groups: vec![],
                allow_hooks: false,
            },
        )
        .unwrap();
        Lockfile::read(self.root.join(Lockfile::FILENAME)).unwrap()
    }
}
mod m_05_e_3_cases {
    use super::*;

    #[test]
    fn catalog_drift_refuses_normal_apply_before_product_writes() {
        let mut fixture = Fixture::new();
        let Plan::Ready(plan) = crate::plan(
            &fixture.source,
            &fixture.root,
            fixture.request(),
            &NullObserver,
        )
        .unwrap() else {
            panic!("ready")
        };
        let before = std::fs::read(fixture.root.join("vibe.toml")).unwrap();
        fixture.source.catalog.candidates.clear();
        let error = crate::apply(
            &fixture.source,
            *plan,
            vibe_core::user_config::SlotIntegrity::TrustPresence,
            &vibe_workspace::hooks::HookPolicy {
                allowed_groups: vec![],
                allow_hooks: false,
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("catalog changed"));
        assert_eq!(
            before,
            std::fs::read(fixture.root.join("vibe.toml")).unwrap()
        );
        assert!(!fixture.root.join("vibe.lock").exists());
        assert!(
            !fixture
                .root
                .join(vibe_workspace::configuration_transaction::RECOVERY_DIRECTORY)
                .exists()
        );
    }

    #[test]
    fn payload_drift_refuses_normal_apply_without_manifest_or_slot_writes() {
        let fixture = Fixture::new();
        let Plan::Ready(plan) = crate::plan(
            &fixture.source,
            &fixture.root,
            fixture.request(),
            &NullObserver,
        )
        .unwrap() else {
            panic!("ready")
        };
        std::fs::write(
            fixture.source.provider.0.join("owner/new.txt"),
            "substituted bytes",
        )
        .unwrap();
        let before = std::fs::read(fixture.root.join("vibe.toml")).unwrap();
        let error = crate::apply(
            &fixture.source,
            *plan,
            vibe_core::user_config::SlotIntegrity::TrustPresence,
            &vibe_workspace::hooks::HookPolicy {
                allowed_groups: vec![],
                allow_hooks: false,
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("captured payload changed"));
        assert_eq!(
            before,
            std::fs::read(fixture.root.join("vibe.toml")).unwrap()
        );
        assert!(!fixture.root.join("vibe.lock").exists());
    }
}

#[test]
fn production_apply_records_requested_and_resolved_separately() {
    let mut defaults = Fixture::new().with_defaults();
    let locked = defaults.apply();
    assert!(
        !vibe_wire::behaviour::lock_configuration::decode(
            locked.meta.configuration.as_ref().unwrap()
        )
        .unwrap()
        .0
        .defaults
        .is_empty()
    );
    assert!(
        crate::plan(
            &defaults.source,
            &defaults.root,
            defaults.request(),
            &NullObserver
        )
        .is_ok()
    );
    defaults
        .source
        .intent
        .as_mut()
        .unwrap()
        .configuration
        .as_mut()
        .unwrap()
        .defaults
        .clear();
    assert!(
        crate::plan(
            &defaults.source,
            &defaults.root,
            defaults.request(),
            &NullObserver
        )
        .is_err()
    );

    let fixture = Fixture::new();
    let lock = fixture.apply();
    let record = lock.meta.configuration.as_ref().unwrap();
    let (request, resolved) = vibe_wire::behaviour::lock_configuration::decode(record).unwrap();
    assert!(
        request
            .configuration
            .as_ref()
            .unwrap()
            .contains("manifest:fixture")
    );
    let owner = resolved
        .packages
        .iter()
        .find(|p| p.package == "org.example/owner@1.0.0")
        .unwrap();
    assert_eq!(owner.feature_closure, vec!["a", "closure"]);
    assert_eq!(resolved.configurations[0].dormant[0].members, vec!["later"]);
    assert!(!request.configuration.unwrap().contains("closure"));
    assert!(matches!(
        crate::plan(
            &fixture.source,
            &fixture.root,
            fixture.request(),
            &NullObserver
        )
        .unwrap(),
        Plan::Ready(_)
    ));
}

#[test]
fn m_05_d_2_preflight_rejects_selected_directory_without_declaration() {
    let fixture = Fixture::new();
    let cached = fixture
        .source
        .resolve_and_fetch(&p("owner"), &fixture.root, None)
        .unwrap();
    std::fs::create_dir_all(cached.cache_dir.join("subskills/stack/rust")).unwrap();
    let mut fetched = crate::Fetched {
        cached,
        feature_expansion: Default::default(),
        meta: crate::NodeInstallMeta {
            dependencies: vec![],
            is_root: true,
        },
        embedded_sources: vec![],
        in_place_incremental: false,
    };
    fetched
        .feature_expansion
        .active_subskills
        .insert("stack/rust".into());
    let error = super::configuration_record(None, &[], std::slice::from_ref(&fetched)).unwrap_err();
    assert!(error.to_string().contains("missing captured declaration"));
    assert!(!fixture.root.join(Lockfile::FILENAME).exists());

    let root = fetched.cached.cache_dir.clone();
    std::fs::write(
        root.join("subskills/stack/rust/vibe-subskill.toml"),
        "[subskill]\npath='stack/rust'\n[content]\nfiles_written=['guide.md']\n",
    )
    .unwrap();
    std::fs::write(
        root.join("subskills/stack/rust/guide.md"),
        "Captured guide\n",
    )
    .unwrap();
    std::fs::write(
        root.join("unselected.txt"),
        "Unselected source stays intact\n",
    )
    .unwrap();
    fetched.cached = fixture
        .source
        .resolve_and_fetch(&p("owner"), &fixture.root, None)
        .unwrap();
    let before = vibe_registry::compute_content_hash(&root).unwrap();
    assert!(
        super::configuration_record(None, &[], std::slice::from_ref(&fetched))
            .unwrap()
            .is_none()
    );
    fetched.feature_expansion.active_subskills.clear();
    assert!(
        super::configuration_record(None, &[], &[fetched])
            .unwrap()
            .is_none()
    );
    assert_eq!(before, vibe_registry::compute_content_hash(&root).unwrap());
    assert_eq!(
        std::fs::read_to_string(root.join("unselected.txt")).unwrap(),
        "Unselected source stays intact\n"
    );
}

#[test]
fn m_05_d_2_install_loading_keeps_complete_slots_and_provenance() {
    let mut hashes = Vec::new();
    for active in [true, false] {
        let mut fixture = Fixture::new();
        let source = fixture.source.provider.0.join("owner");
        for (name, mode) in [
            ("eager", "eager"),
            ("push", "lazy-push"),
            ("pull", "lazy-pull"),
        ] {
            let dir = source.join("subskills").join(name);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("vibe-subskill.toml"), format!("[subskill]\npath='{name}'\ndelivery='{mode}'\ndescription='Use for {name} work'\n[content]\nfiles_written=['guide.md']\n")).unwrap();
            std::fs::write(dir.join("guide.md"), format!("# {name} selected content\n")).unwrap();
        }
        std::fs::write(source.join("unselected.txt"), "complete source\n").unwrap();
        let mut manifest = Manifest::read(source.join("vibe.toml")).unwrap();
        // Independent Agent Skill declaration deliberately has a different name/path.
        let skill: Manifest = Manifest::parse_str("[package]\ngroup='org.example'\nname='owner'\nkind='flow'\nversion='1.0.0'\n[[skill]]\nname='independent'\npath='skills/unrelated'\n").unwrap();
        manifest.skills = skill.skills;
        manifest.write(source.join("vibe.toml")).unwrap();
        std::fs::create_dir_all(source.join("skills/unrelated")).unwrap();
        std::fs::write(source.join("skills/unrelated/SKILL.md"), "---\nname: independent\ndescription: Independent guidance\n---\nIndependent skill bytes\n").unwrap();
        let before = vibe_registry::compute_content_hash(&source).unwrap();
        let intent = fixture.source.intent.as_mut().unwrap();
        let config = intent.configuration.as_mut().unwrap();
        let definition = &mut config.definitions[0];
        definition.identity = fixture
            .source
            .provider
            .candidate_metadata(
                &p("owner").group.unwrap(),
                "owner",
                &semver::Version::new(1, 0, 0),
            )
            .unwrap()
            .identity;
        definition.definition.declarations.exported_subskills =
            ["eager".into(), "push".into(), "pull".into()].into();
        definition.definition.program.features.insert(
            f("a"),
            ["eager", "push", "pull"]
                .into_iter()
                .map(|p| FeatureEffect::ExportedSubskill(p.into()))
                .collect(),
        );
        if !active {
            config.request.edges[0]
                .selections
                .get_mut("choice")
                .unwrap()
                .members = Some(vec!["gate".into()]);
        }
        let catalog =
            TransactionCatalog::capture(&fixture.source.provider, &intent.catalog_roots()).unwrap();
        let config = intent.configuration.as_ref().unwrap();
        let solved = vibe_resolver::ResolvoDepSolver::new(&catalog)
            .solve_configuration(
                &intent.roots,
                &config.request,
                &config.definitions,
                &config.defaults,
            )
            .unwrap();
        fixture.source.route = RouteResolution::bind(
            &catalog,
            intent,
            RecordedResolution {
                graph: solved.graph,
                choices: solved.choices,
            },
            solved.manifests,
            solved.effects,
        )
        .unwrap();
        fixture.source.catalog = catalog.inspect().clone();
        let lock = fixture.apply();
        let owner = lock.find(&p("owner").group.unwrap(), "owner").unwrap();
        assert!(owner.subskills_active.is_empty());
        let slot = vibe_workspace::vibedeps::slot_abs_path(
            &fixture.root,
            &owner.group,
            &owner.name,
            &owner.version,
        );
        assert_eq!(
            before,
            vibe_registry::compute_content_hash(&source).unwrap()
        );
        fn assert_full_source(source: &Path, slot: &Path) {
            for entry in std::fs::read_dir(source).unwrap() {
                let entry = entry.unwrap();
                let target = slot.join(entry.file_name());
                if entry.file_type().unwrap().is_dir() {
                    assert_full_source(&entry.path(), &target);
                } else {
                    assert_eq!(
                        std::fs::read(entry.path()).unwrap(),
                        std::fs::read(&target).unwrap(),
                        "{}",
                        target.display()
                    );
                }
            }
        }
        assert_full_source(&source, &slot);
        hashes.push(before);
        assert_eq!(
            std::fs::read(slot.join("unselected.txt")).unwrap(),
            b"complete source\n"
        );
        let selected = vibe_workspace::install::effective_subskills::from_locked_package(
            &fixture.root,
            &lock,
            owner,
        )
        .unwrap();
        assert_eq!(selected.len(), if active { 3 } else { 0 });
        let boot_dir = fixture.root.join(vibe_core::layout::current_boot_dir());
        let static_text = std::fs::read_to_string(boot_dir.join("STATIC.md")).unwrap_or_default();
        let index = std::fs::read_to_string(boot_dir.join("INDEX.md")).unwrap();
        assert_eq!(
            static_text.contains("eager selected content"),
            active,
            "{static_text}"
        );
        assert!(!static_text.contains("push selected content"));
        assert!(!static_text.contains("pull selected content"));
        assert_eq!(
            index.contains("subskills/push/vibe-subskill.toml"),
            active,
            "{index}"
        );
        assert!(!index.contains("subskills/pull/guide.md"));
        if active {
            assert!(index.contains("Use for push work"));
            assert!(index.contains("read_subskill"));
        }
        let workspace = vibe_workspace::Workspace::load(&fixture.root).unwrap();
        vibe_workspace::install::regenerate_boot(&workspace).unwrap();
        assert_eq!(
            std::fs::read_to_string(boot_dir.join("STATIC.md")).unwrap_or_default(),
            static_text
        );
        let analyzed = vibe_workspace::install::analyze_node_lane(&workspace, ".", None).unwrap();
        assert_eq!(analyzed.is_some(), active);
        if let Some(analyzed) = analyzed {
            assert!(
                String::from_utf8_lossy(analyzed.artifact.bytes())
                    .contains("eager selected content")
            );
        }
        assert!(!fixture.root.join("guide.md").exists());
        let (_, resolved) = vibe_wire::behaviour::lock_configuration::decode(
            lock.meta.configuration.as_ref().unwrap(),
        )
        .unwrap();
        let row = resolved
            .packages
            .iter()
            .find(|row| row.package == "org.example/owner@1.0.0")
            .unwrap();
        assert_eq!(row.content, owner.content_hash.to_string());
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&row.effects).unwrap()["subskills"]
                .as_array()
                .unwrap()
                .len(),
            if active { 3 } else { 0 }
        );
    }
    assert_eq!(hashes[0], hashes[1]);
}
#[test]
fn exact_replay_rejects_changed_definition_or_lost_request() {
    let mut fixture = Fixture::new();
    let lock = fixture.apply();
    let saved = fixture.source.catalog.candidates.clone();
    for (_, identity) in &mut fixture.source.catalog.candidates {
        identity.definition = "sha256:changed-at-same-version".into();
    }
    assert!(
        crate::plan(
            &fixture.source,
            &fixture.root,
            fixture.request(),
            &NullObserver
        )
        .unwrap_err()
        .to_string()
        .contains("capture changed")
    );
    fixture.source.catalog.candidates = saved;
    let (authored, mut derived) =
        vibe_wire::behaviour::lock_configuration::decode(lock.meta.configuration.as_ref().unwrap())
            .unwrap();
    derived.packages[0].feature_closure.push("tampered".into());
    let mut tampered = lock.clone();
    tampered.meta.configuration =
        Some(vibe_wire::behaviour::lock_configuration::encode(authored, derived).unwrap());
    tampered
        .write(fixture.root.join(Lockfile::FILENAME))
        .unwrap();
    assert!(
        crate::plan(
            &fixture.source,
            &fixture.root,
            fixture.request(),
            &NullObserver
        )
        .unwrap_err()
        .to_string()
        .contains("resolved choices")
    );
    lock.write(fixture.root.join(Lockfile::FILENAME)).unwrap();
    fixture
        .source
        .intent
        .as_mut()
        .unwrap()
        .configuration
        .as_mut()
        .unwrap()
        .definitions[0]
        .definition
        .program
        .features
        .insert(f("changed"), BTreeSet::new());
    assert!(
        crate::plan(
            &fixture.source,
            &fixture.root,
            fixture.request(),
            &NullObserver
        )
        .unwrap_err()
        .to_string()
        .contains("definition changed")
    );
    fixture.source.intent = None;
    assert!(
        crate::plan(
            &fixture.source,
            &fixture.root,
            fixture.request(),
            &NullObserver
        )
        .unwrap_err()
        .to_string()
        .contains("authority unavailable")
    );
    let path = fixture.root.join(Lockfile::FILENAME);
    let text = std::fs::read_to_string(&path).unwrap();
    let digest = &lock.meta.configuration.as_ref().unwrap().request_digest;
    std::fs::write(&path, text.replace(digest, "sha256:lost")).unwrap();
    assert!(Lockfile::read(path).is_err());
}
#[test]
fn induced_edges_retain_declaration_and_consumer_intent() {
    let fixture = Fixture::new();
    let lock = fixture.apply();
    let record = lock.meta.configuration.as_ref().unwrap();
    let (requested, resolved) = vibe_wire::behaviour::lock_configuration::decode(record).unwrap();
    assert!(
        resolved
            .edges
            .iter()
            .any(|e| e.declaration.starts_with("requires.capabilities["))
    );
    assert!(
        resolved
            .edges
            .iter()
            .any(|e| e.declaration.starts_with("configuration.dependencies["))
    );
    for edge in &resolved.edges {
        assert_eq!(edge.request_digest, record.request_digest);
        assert_eq!(
            edge.owner_identity.definition,
            requested.definitions[0].identity.definition
        );
        assert!(!edge.provider_identity.content.is_empty());
    }
    let mut explicit = fixture.request();
    explicit.roots = vec![p("owner")];
    assert!(matches!(
        crate::plan(&fixture.source, &fixture.root, explicit, &NullObserver).unwrap(),
        Plan::Ready(_)
    ));
}

#[test]
fn m_05_e_2_exact_capture_replays_induced_edges_and_dormant_choices() {
    let fixture = Fixture::new();
    let Plan::Ready(plan) = crate::plan(
        &fixture.source,
        &fixture.root,
        fixture.request(),
        &NullObserver,
    )
    .unwrap() else {
        panic!("ready")
    };
    let retained = plan.clone();
    crate::apply(
        &fixture.source,
        *plan,
        vibe_core::user_config::SlotIntegrity::TrustPresence,
        &vibe_workspace::hooks::HookPolicy {
            allowed_groups: vec![],
            allow_hooks: false,
        },
    )
    .unwrap();
    let lock = Lockfile::read(fixture.root.join(Lockfile::FILENAME)).unwrap();
    let replay = crate::evolution::PreparedReinstall::from_planned(
        &lock,
        &fixture.source,
        &retained,
        &fixture.source.catalog,
    )
    .unwrap();
    assert_eq!(
        replay.report.decision,
        crate::evolution::Decision::ExactReuse
    );
    assert_eq!(replay.resolution().len(), lock.packages.len());
    let report = replay.report.value();
    assert_eq!(
        report["differences"]["resolved"]["before"],
        report["differences"]["resolved"]["after"]
    );
    assert!(report.to_string().contains("later"));
    assert!(report.to_string().contains("configuration.dependencies["));
}

#[test]
fn m_05_e_2_missing_authority_and_unknown_epoch_refuse() {
    let fixture = Fixture::new();
    let mut lock = fixture.apply();
    assert!(crate::evolution::require_update_authority(&lock).is_err());
    lock.meta.configuration.as_mut().unwrap().epoch += 1;
    assert!(crate::evolution::require_update_authority(&lock).is_err());
    assert!(
        crate::evolution::require_update_authority(&Lockfile::empty(
            "test",
            "2026-10-17T00:00:00Z"
        ))
        .is_ok()
    );
}

#[test]
fn m_05_e_2_update_requires_new_shown_default_receipt() {
    let mut fixture = Fixture::new().with_defaults();
    fixture.apply();
    let before = std::fs::read(fixture.root.join(Lockfile::FILENAME)).unwrap();
    let c = fixture
        .source
        .intent
        .as_mut()
        .unwrap()
        .configuration
        .as_mut()
        .unwrap();
    c.definitions[0]
        .definition
        .constraints
        .groups
        .get_mut(&f("choice"))
        .unwrap()
        .defaults = Some([f("gate")].into());
    let mut workspace = vibe_workspace::Workspace::discover(&fixture.root).unwrap();
    let mut manifest = Manifest::read(fixture.root.join(Manifest::FILENAME)).unwrap();
    let update = crate::evolution::plan_update(
        &fixture.source,
        &fixture.root,
        &mut manifest,
        &mut workspace,
        fixture.request(),
        vibe_core::manifest::SpecFormat::Mixed,
        &NullObserver,
    )
    .unwrap();
    assert_eq!(
        update.report.decision,
        crate::evolution::Decision::ReconfigurationRequired
    );
    assert!(
        update
            .report
            .needs
            .iter()
            .any(|n| n.contains("shown-definition default authorization"))
    );
    assert!(update.plan.is_none());
    assert_eq!(
        before,
        std::fs::read(fixture.root.join(Lockfile::FILENAME)).unwrap()
    );
}
