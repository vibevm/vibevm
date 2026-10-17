//! Public production-planner regressions: vibe-install has lib.test=false.
use std::{
    collections::BTreeSet,
    path::Path,
    sync::{Arc, Mutex},
};
use vibe_core::manifest::{feature_constraints::*, feature_effects::*};
use vibe_core::{
    PackageRef,
    manifest::{Lockfile, Manifest, SpecFormat},
    progress::*,
};
use vibe_install::{InstallRequest, InstallSource, Plan, PlanEvent, PlanObserver};
use vibe_registry::{CachedPackage, CandidateMetadata, InPlaceMaterialised, RegistryError};
use vibe_resolver::{
    FeatureRequest, ResolvedGraph, SolveError,
    features::configuration::ConfigurationDefinition,
    resolvo_engine::{
        CatalogInspection,
        configuration::{CandidateConfiguration, DefaultBinding},
        request::{ConfigurationInput, NormalizedRequest},
    },
};
fn local(s: &str) -> LocalFeatureRef {
    LocalFeatureRef::new(s).unwrap()
}
fn consumer() -> vibe_wire::generated::consumer_configuration::e1::request::Request {
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
struct MetadataSource(CandidateMetadata);
impl vibe_resolver::DepProvider for MetadataSource {
    fn resolve_version(
        &self,
        _: &PackageRef,
    ) -> Result<semver::Version, vibe_resolver::DepProviderError> {
        Ok(semver::Version::new(1, 0, 0))
    }
    fn fetch_manifest(
        &self,
        _: &vibe_core::Group,
        _: &str,
        _: &semver::Version,
    ) -> Result<Manifest, vibe_resolver::DepProviderError> {
        Ok(self.0.manifest.clone())
    }
    fn candidate_metadata(
        &self,
        _: &vibe_core::Group,
        _: &str,
        _: &semver::Version,
    ) -> Result<CandidateMetadata, vibe_resolver::DepProviderError> {
        Ok(self.0.clone())
    }
}
impl vibe_resolver::VersionEnumerator for MetadataSource {
    fn list_versions(
        &self,
        _: &vibe_core::Group,
        _: &str,
    ) -> Result<Vec<semver::Version>, vibe_resolver::DepProviderError> {
        Ok(vec![semver::Version::new(1, 0, 0)])
    }
}
struct Source {
    current: Option<NormalizedRequest>,
    catalog: CatalogInspection,
}
impl InstallSource for Source {
    fn normalized_request(&self) -> Option<&NormalizedRequest> {
        self.current.as_ref()
    }
    fn catalog_inspection(&self) -> Option<&CatalogInspection> {
        Some(&self.catalog)
    }
    fn begin_normalized_transaction(
        &self,
        r: &NormalizedRequest,
    ) -> Result<Option<Box<dyn InstallSource + '_>>, SolveError> {
        if let Some(current) = &self.current {
            assert_eq!(
                vibe_install::requested_record(r).unwrap(),
                vibe_install::requested_record(current).unwrap()
            );
        }
        Ok(None)
    }
    fn solve(&self, _: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        panic!("no solve before exact-replay decision")
    }
    fn solve_masked(
        &self,
        _: &[PackageRef],
        _: &BTreeSet<(String, String)>,
    ) -> Result<ResolvedGraph, SolveError> {
        panic!("no solve")
    }
    fn manifest_of(&self, _: &PackageRef) -> Result<Manifest, SolveError> {
        panic!("no provider")
    }
    fn resolve_and_fetch(
        &self,
        _: &PackageRef,
        _: &Path,
        _: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        panic!("no fetch")
    }
    fn materialise_in_place(
        &self,
        _: &PackageRef,
        _: &Path,
    ) -> Result<InPlaceMaterialised, RegistryError> {
        panic!("no write")
    }
}
#[derive(Default)]
struct Evidence {
    decisions: Mutex<Vec<String>>,
}
impl ProgressObserver for Evidence {
    fn observe(&self, event: ProgressEvent) {
        if let ProgressEventKind::Skipped { reason } = event.kind {
            self.decisions.lock().unwrap().push(reason);
        }
    }
}
struct Observer(Arc<Evidence>);
impl PlanObserver for Observer {
    fn on(&self, event: PlanEvent) {
        if let PlanEvent::Reresolving { reason } = event {
            self.0.decisions.lock().unwrap().push(reason);
        }
    }
    fn progress(&self) -> Progress {
        Progress::new(self.0.clone())
    }
}
fn fixture(empty: bool) -> (tempfile::TempDir, Source, Observer) {
    let dir = tempfile::tempdir().unwrap();
    let requires = if empty {
        ""
    } else {
        "[requires.packages]\n'org.demo/demo'='*'\n"
    };
    std::fs::write(
        dir.path().join("vibe.toml"),
        format!("[project]\nname='consumer'\nversion='1.0.0'\n{requires}"),
    )
    .unwrap();
    let candidate_dir = dir.path().join("candidate");
    std::fs::create_dir(&candidate_dir).unwrap();
    std::fs::write(
        candidate_dir.join("vibe.toml"),
        "[package]\ngroup='org.demo'\nname='demo'\nkind='flow'\nversion='1.0.0'\n",
    )
    .unwrap();
    let metadata =
        CandidateMetadata::read_local(&candidate_dir, "fixture:admitted-source".into()).unwrap();
    let package = PackageRef::parse("org.demo/demo").unwrap();
    let request = consumer();
    let candidate = CandidateConfiguration {
        package: package.clone(),
        version: semver::Version::new(1, 0, 0),
        identity: metadata.identity.clone(),
        definition: ConfigurationDefinition {
            instance: vibe_wire::behaviour::consumer_configuration::admit_request(&request)
                .unwrap()[0]
                .instance
                .clone(),
            program: FeatureProgram {
                features: [
                    (local("red"), Default::default()),
                    (local("blue"), Default::default()),
                ]
                .into(),
                exclusive: Default::default(),
            },
            declarations: Default::default(),
            constraints: Constraints {
                groups: [(
                    local("mode"),
                    ConstrainedGroup {
                        cardinality: Cardinality::ExactlyOne,
                        members: [local("red"), local("blue")].into(),
                        guards: vec![],
                        defaults: Some([local("red")].into()),
                    },
                )]
                .into(),
                member_guards: Default::default(),
            },
        },
    };
    let roots = if empty { vec![] } else { vec![package.clone()] };
    let mut current = NormalizedRequest::legacy(&roots, &FeatureRequest::default());
    for origin in &mut current.root_origins {
        origin.consumer = "fixture-consumer".into();
        origin.declaration = "requires.packages:demo".into();
    }
    current.configuration = Some(ConfigurationInput {
        request: request.clone(),
        definitions: vec![candidate.clone()],
        defaults: vec![DefaultBinding::shown(0, &request, &candidate)],
    });
    let requested = vibe_install::requested_record(&current).unwrap();
    let outcome = vibe_resolver::features::configuration::resolve_consumer_configuration(
        &request,
        &[candidate.definition.clone()],
    )
    .unwrap();
    let resolved=serde_json::from_value(serde_json::json!({"epoch":"1","request_digest":"pending","packages":[],"edges":[],
        "configurations":[{"package":"org.demo/demo@1.0.0","result":serde_json::to_string(&outcome.result).unwrap(),"dormant":[]}]})).unwrap();
    let mut lock = Lockfile::empty("E1", "2026-10-17T00:00:00Z");
    if !empty {
        lock.meta.root_dependencies = roots;
        lock.packages.push(serde_json::from_value(serde_json::json!({"kind":"flow","group":"org.demo","name":"demo","version":"1.0.0",
            "registry":"fixture","source_url":"fixture:admitted-source","content_hash":metadata.identity.content,
            "dependencies":[],"features":[],"files_written":[],"subskills_active":[],"overridden":false})).unwrap());
        let slot = vibe_workspace::Workspace::discover(dir.path())
            .unwrap()
            .vibedeps_slot(
                &metadata.manifest.require_package().unwrap().group,
                "demo",
                &semver::Version::new(1, 0, 0),
            );
        std::fs::create_dir_all(&slot).unwrap();
        std::fs::copy(candidate_dir.join("vibe.toml"), slot.join("vibe.toml")).unwrap();
    }
    lock.meta.configuration =
        Some(vibe_wire::behaviour::lock_configuration::encode(requested, resolved).unwrap());
    lock.write(dir.path().join("vibe.lock")).unwrap();
    let catalog = vibe_resolver::resolvo_engine::TransactionCatalog::capture(
        &MetadataSource(metadata),
        &[package],
    )
    .unwrap()
    .inspect()
    .clone();
    (
        dir,
        Source {
            current: Some(current),
            catalog,
        },
        Observer(Arc::new(Evidence::default())),
    )
}
fn plan(dir: &Path, source: &Source, observer: &Observer) -> Result<Plan, vibe_install::Error> {
    let before = ["vibe.toml", "vibe.lock"].map(|p| std::fs::read(dir.join(p)).unwrap());
    let mut manifest = Manifest::read(dir.join("vibe.toml")).unwrap();
    let mut workspace = vibe_workspace::Workspace::discover(dir).unwrap();
    let result = vibe_install::plan_prepared_with_spec_format(
        source,
        dir,
        &mut manifest,
        &mut workspace,
        InstallRequest {
            roots: vec![],
            features: FeatureRequest::default(),
            language: None,
            exact: false,
            generated_by: "E1".into(),
            offline: true,
        },
        SpecFormat::Mixed,
        observer,
    );
    assert_eq!(
        before,
        ["vibe.toml", "vibe.lock"].map(|p| std::fs::read(dir.join(p)).unwrap())
    );
    result
}
fn changed(mut change: impl FnMut(&mut NormalizedRequest)) {
    for empty in [false, true] {
        let (dir, mut source, observer) = fixture(empty);
        change(source.current.as_mut().unwrap());
        let error = plan(dir.path(), &source, &observer)
            .unwrap_err()
            .to_string();
        assert!(error.contains("exact replay refused"), "{error}");
        let decisions = observer.0.decisions.lock().unwrap();
        assert!(
            decisions
                .iter()
                .any(|s| s.contains("request or definition changed")
                    && s.contains("current=sha256:")
                    && s.contains("retained=sha256:")),
            "{decisions:?}"
        );
    }
}
#[test]
fn choice_only_defeats_fresh() {
    changed(|r| {
        r.configuration.as_mut().unwrap().request.edges[0]
            .selections
            .get_mut("mode")
            .unwrap()
            .members = Some(vec!["blue".into()])
    });
}
#[test]
fn feature_only_defeats_fresh() {
    changed(|r| r.features.explicit.push("extra".into()));
}
#[test]
fn definition_only_defeats_fresh() {
    changed(|r| {
        r.configuration.as_mut().unwrap().definitions[0]
            .definition
            .constraints
            .groups
            .get_mut(&local("mode"))
            .unwrap()
            .defaults = Some([local("blue")].into())
    });
}
#[test]
fn unchanged_request_and_definition_reuses_fresh() {
    for empty in [false, true] {
        let (dir, source, observer) = fixture(empty);
        assert!(matches!(
            plan(dir.path(), &source, &observer).unwrap(),
            Plan::Fresh
        ));
        let lock = Lockfile::read(dir.path().join("vibe.lock")).unwrap();
        let digest = &lock.meta.configuration.unwrap().request_digest;
        assert!(
            observer
                .0
                .decisions
                .lock()
                .unwrap()
                .iter()
                .any(|s| s.contains(&format!("current={digest} retained={digest}")))
        );
    }
}
#[test]
fn missing_current_authority_never_fresh() {
    for empty in [false, true] {
        let (dir, mut source, observer) = fixture(empty);
        source.current = None;
        assert!(
            plan(dir.path(), &source, &observer)
                .unwrap_err()
                .to_string()
                .contains("current authored configuration authority unavailable")
        );
    }
}
#[test]
fn changed_default_receipt_defeats_fresh() {
    changed(|r| {
        let c = r.configuration.as_mut().unwrap();
        c.defaults.clear();
    });
}
#[test]
fn malformed_and_unknown_epoch_refuse() {
    for mode in [0, 1] {
        let (dir, source, observer) = fixture(false);
        let lock = Lockfile::read(dir.path().join("vibe.lock")).unwrap();
        let record = lock.meta.configuration.as_ref().unwrap();
        // Lockfile::write validates; corrupt the retained bytes instead.
        let mut value = std::fs::read_to_string(dir.path().join("vibe.lock")).unwrap();
        if mode == 0 {
            value = value.replace("epoch = 1", "epoch = 99");
        } else {
            value = value.replace(&record.request_digest, "sha256:malformed");
        }
        std::fs::write(dir.path().join("vibe.lock"), value).unwrap();
        assert!(plan(dir.path(), &source, &observer).is_err());
    }
}
#[test]
fn legacy_nondefault_flags_defeat_fresh() {
    let lock = Lockfile::empty("E1", "now");
    for flag in ["explicit", "no_defaults", "all"] {
        let mut r = NormalizedRequest::legacy(&[], &FeatureRequest::default());
        match flag {
            "explicit" => r.features.explicit.push("extra".into()),
            "no_defaults" => r.features.no_defaults = true,
            _ => r.features.all = true,
        };
        assert!(!vibe_install::request_freshness(&lock, &r).unwrap().matches);
    }
}
#[test]
fn empty_cached_discovery_no_provider_or_writes() {
    let view = vibe_workspace::configuration_discovery::discover([]);
    assert_eq!(view.status, "unavailable");
    assert_eq!(
        (
            view.counters.provider_calls,
            view.counters.fetches,
            view.counters.authored_writes,
            view.counters.cache_writes
        ),
        (0, 0, 0, 0)
    );
}
#[test]
fn repeated_unresolved_needs_selection_no_loop() {
    let request = consumer();
    let (dir, source, _) = fixture(true);
    let definition = &source
        .current
        .as_ref()
        .unwrap()
        .configuration
        .as_ref()
        .unwrap()
        .definitions[0]
        .definition;
    let mut unresolved = request.clone();
    unresolved.edges[0].selections.clear();
    let query = || {
        vibe_resolver::features::configuration::resolve_consumer_configuration(
            &unresolved,
            std::slice::from_ref(definition),
        )
        .unwrap()
    };
    let first = query();
    assert_eq!(
        first.result.outcomes[0].status,
        vibe_wire::generated::consumer_configuration::e1::result::OutcomeStatus::NeedsSelection
    );
    assert_eq!(first.result, query().result);
    assert!(first.expansions[0].is_none());
    drop(dir);
}
