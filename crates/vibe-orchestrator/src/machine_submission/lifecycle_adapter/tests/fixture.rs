//! Actual one-package install through a controlled local fixture source.
//! The bound source port checks its serving root against the selected tree.

use super::*;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use vibe_core::PackageRef;
use vibe_core::manifest::{Lockfile, Manifest};
use vibe_install::InstallSource;
use vibe_lifecycle::process::capture_package_tree_witness;
use vibe_package_source::{
    InstallResolver, PackageQualifier, PackageSourceOptions, RegistryPackageSource,
    build_install_resolver_with_progress,
};
use vibe_registry::{CachedPackage, InPlaceMaterialised, RegistryError};
use vibe_resolver::{ResolvedGraph, SolveError};

static FIXTURE_LOCK: Mutex<()> = Mutex::new(());
fn fixture_guard() -> std::sync::MutexGuard<'static, ()> {
    FIXTURE_LOCK.lock().unwrap()
}

#[derive(Clone, Copy)]
enum CacheFault {
    None,
    Changed,
    Extra,
    Missing,
    WrongHash,
    #[cfg(unix)]
    Symlink,
}

fn setup() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let isolated = vibe_test_support::isolated_home().expect("fixture tests require isolated home");
    let package_cache = isolated.join("cache/org.example/fixture");
    if package_cache.exists() {
        std::fs::remove_dir_all(&package_cache).unwrap();
    }
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("consumer");
    let registry = dir.path().join("registry");
    let source = registry.join("org.example/fixture/v1.1.0");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(project.join("vibe.toml"), install::FIXTURE_PROJECT_MANIFEST).unwrap();
    std::fs::write(source.join("vibe.toml"), install::FIXTURE_PACKAGE_MANIFEST).unwrap();
    std::fs::write(source.join("README.md"), b"# Fixture\n").unwrap();
    (dir, project, source)
}

fn fixture_request() -> DefaultLifecycleRequest {
    let mut request = install_request();
    request.policy.offline = true;
    request.policy.slot_integrity = SlotIntegrity::Verify;
    request
}

fn fixture_call(
    project: &Path,
    source: &Path,
    factory: &FixtureFactory,
    namespace: &SubmissionNamespace,
    scope: &SubmissionScope,
    key: &str,
) -> Result<SubmissionOutcome<RetainedLifecycleResult>> {
    install::submit_one_package_fixture_install_with_hook(
        project,
        source,
        factory,
        namespace,
        scope,
        key,
        fixture_request(),
        TargetOs::current().expect("supported fixture platform"),
        "2026-09-30T00:00:05Z".parse().unwrap(),
        None,
        None,
    )
}

struct ExactQualifier(PackageRef);
impl PackageQualifier for ExactQualifier {
    fn qualify(
        &self,
        _resolver: &InstallResolver,
        pkgref: &PackageRef,
        _locked: &Lockfile,
    ) -> Result<PackageRef> {
        ensure!(
            pkgref == &self.0,
            "fixture qualifier refused a nonfixed pin"
        );
        Ok(pkgref.clone())
    }
}

struct FixtureFactory {
    registry_root: PathBuf,
    source_root: PathBuf,
    witness: PackageTreeWitness,
    pin: PackageRef,
    builds: AtomicUsize,
    fetches: Arc<AtomicUsize>,
    fail_after_fetch: bool,
    cache_fault: CacheFault,
}

impl FixtureFactory {
    fn new(registry_root: PathBuf) -> Self {
        Self::with_pin(
            registry_root,
            PackageRef::parse(install::FIXTURE_PIN).unwrap(),
        )
    }
    fn with_pin(registry_root: PathBuf, pin: PackageRef) -> Self {
        let version = pin.version.to_string();
        let source_root = registry_root
            .join(pin.group.as_ref().unwrap().as_str())
            .join(pin.name.as_str())
            .join(format!("v{}", version.strip_prefix('=').unwrap()));
        let witness =
            capture_package_tree_witness(&source_root, &pin, install::FIXTURE_FILES).unwrap();
        Self {
            registry_root,
            source_root,
            witness,
            pin,
            builds: AtomicUsize::new(0),
            fetches: Arc::new(AtomicUsize::new(0)),
            fail_after_fetch: false,
            cache_fault: CacheFault::None,
        }
    }
}

impl BoundOnePackageSource for FixtureFactory {
    fn source_witness(&self) -> &PackageTreeWitness {
        &self.witness
    }
    fn recheck_source_binding(&self, witness: &PackageTreeWitness) -> Result<()> {
        ensure!(
            self.source_root == witness.root,
            "fixture source root differs from witness"
        );
        Ok(recheck_package_tree_witness(witness)?)
    }
}

impl PackageSourceFactory for FixtureFactory {
    fn build(&self, input: PackageSourceBuild<'_>) -> Result<Box<dyn PackageSource>> {
        ensure!(input.offline, "fixture source must be offline");
        self.builds.fetch_add(1, Ordering::SeqCst);
        let options = PackageSourceOptions {
            registry: Some(self.registry_root.clone()),
            no_default_registry: true,
            no_prefer_local: true,
            ..Default::default()
        };
        let resolver = build_install_resolver_with_progress(
            &options,
            input.manifest,
            input.embedded_root,
            input.project_root,
            input.global,
            input.offline,
            input.locked,
            input.progress,
        )?;
        Ok(Box::new(FixtureSource {
            inner: RegistryPackageSource::new(resolver, Box::new(ExactQualifier(self.pin.clone()))),
            pin: self.pin.clone(),
            fetches: Arc::clone(&self.fetches),
            fail_after_fetch: self.fail_after_fetch,
            cache_fault: self.cache_fault,
            project_root: input.project_root.to_path_buf(),
        }))
    }
}

struct FixtureSource {
    inner: RegistryPackageSource,
    pin: PackageRef,
    fetches: Arc<AtomicUsize>,
    fail_after_fetch: bool,
    cache_fault: CacheFault,
    project_root: PathBuf,
}

struct MutatingConfirm {
    cache_file: PathBuf,
    calls: AtomicUsize,
}

impl ConfirmGate for MutatingConfirm {
    fn confirm_install(&self, packages: usize) -> Result<()> {
        ensure!(packages == 1, "fixture confirmation expected one package");
        ensure!(
            self.cache_file.is_file(),
            "cache was not fetched before confirmation"
        );
        self.calls.fetch_add(1, Ordering::SeqCst);
        std::fs::write(&self.cache_file, b"changed after fetch before apply\n")?;
        Ok(())
    }
}

impl PackageSource for FixtureSource {
    fn qualify(&self, pkgref: &PackageRef, _locked: &Lockfile) -> Result<PackageRef> {
        ensure!(pkgref == &self.pin, "fixture source refused a nonfixed pin");
        Ok(pkgref.clone())
    }
}

impl InstallSource for FixtureSource {
    fn resolve_and_fetch(
        &self,
        pkgref: &PackageRef,
        store_root: &Path,
        expected_hash: Option<&str>,
    ) -> std::result::Result<CachedPackage, RegistryError> {
        let entered = self.project_root.join(".vibe/lifecycle-outcomes");
        let stage_present = std::fs::read_dir(&entered).ok().is_some_and(|entries| {
            entries.flatten().any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".install-entered.json")
            })
        });
        if !stage_present {
            return Err(RegistryError::Io {
                path: entered,
                source: std::io::Error::other("source fetch preceded install-entered stage"),
            });
        }
        let mut cached = self
            .inner
            .resolve_and_fetch(pkgref, store_root, expected_hash)?;
        self.fetches.fetch_add(1, Ordering::SeqCst);
        match self.cache_fault {
            CacheFault::None => {}
            CacheFault::Changed => {
                std::fs::write(cached.cache_dir.join("README.md"), b"wrong cache bytes\n").unwrap();
            }
            CacheFault::Extra => {
                std::fs::write(cached.cache_dir.join("extra.txt"), b"unexpected").unwrap();
            }
            CacheFault::Missing => {
                std::fs::remove_file(cached.cache_dir.join("README.md")).unwrap();
            }
            CacheFault::WrongHash => {
                cached.content_hash = format!("sha256:{}", "0".repeat(64));
            }
            #[cfg(unix)]
            CacheFault::Symlink => {
                use std::os::unix::fs::symlink;
                std::fs::remove_file(cached.cache_dir.join("README.md")).unwrap();
                symlink(
                    cached.cache_dir.join("vibe.toml"),
                    cached.cache_dir.join("README.md"),
                )
                .unwrap();
            }
        }
        if self.fail_after_fetch {
            return Err(RegistryError::Io {
                path: store_root.to_path_buf(),
                source: std::io::Error::other("fixture response lost after cache fetch"),
            });
        }
        Ok(cached)
    }

    fn solve(&self, roots: &[PackageRef]) -> std::result::Result<ResolvedGraph, SolveError> {
        self.inner.solve(roots)
    }

    fn manifest_of(&self, pkg: &PackageRef) -> std::result::Result<Manifest, SolveError> {
        self.inner.manifest_of(pkg)
    }

    fn solve_masked(
        &self,
        roots: &[PackageRef],
        blocked: &BTreeSet<(String, String)>,
    ) -> std::result::Result<ResolvedGraph, SolveError> {
        self.inner.solve_masked(roots, blocked)
    }

    fn materialise_in_place(
        &self,
        pkgref: &PackageRef,
        slot: &Path,
    ) -> std::result::Result<InPlaceMaterialised, RegistryError> {
        self.inner.materialise_in_place(pkgref, slot)
    }
}

#[test]
fn source_drift_after_stage_refuses_before_factory_or_install_effect() {
    let _guard = fixture_guard();
    assert!(vibe_test_support::isolated_home().is_some());
    let (_dir, project, source) = setup();
    let registry = source
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let factory = FixtureFactory::new(registry);
    let scope = scope();
    let namespace = issue(&project, &scope);
    let drift = || std::fs::write(source.join("README.md"), b"changed after stage\n").unwrap();
    let result = install::submit_one_package_fixture_install_with_hook(
        &project,
        &source,
        &factory,
        &namespace,
        &scope,
        "one",
        fixture_request(),
        TargetOs::current().expect("supported fixture platform"),
        "2026-09-30T00:00:05Z".parse().unwrap(),
        Some(&drift),
        None,
    )
    .unwrap();
    let SubmissionOutcome::RecoveryRequired { run_id } = result else {
        panic!("source drift after stage must retain recovery ID: {result:?}")
    };
    assert!(
        project
            .join(".vibe/lifecycle-outcomes")
            .join(format!("{run_id}.install-entered.json"))
            .is_file()
    );
    assert_eq!(factory.builds.load(Ordering::SeqCst), 0);
    assert_eq!(factory.fetches.load(Ordering::SeqCst), 0);
    assert!(!project.join("vibe.lock").exists());
    assert!(
        !project
            .join("vibevm/vibedeps/org.example.fixture/1.1.0")
            .exists()
    );
    assert!(!project.join("vibevm/vibespecs/boot/INDEX.md").exists());
    std::fs::write(source.join("README.md"), b"# Fixture\n").unwrap();
    assert_eq!(
        fixture_call(&project, &source, &factory, &namespace, &scope, "one").unwrap(),
        SubmissionOutcome::RecoveryRequired { run_id }
    );
    assert_eq!(factory.builds.load(Ordering::SeqCst), 0);
}

#[test]
fn failure_after_cache_fetch_is_retained_uncertain_and_never_refetched() {
    let _guard = fixture_guard();
    assert!(vibe_test_support::isolated_home().is_some());
    let (_dir, project, source) = setup();
    let registry = source
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let mut factory = FixtureFactory::new(registry);
    factory.fail_after_fetch = true;
    let scope = scope();
    let namespace = issue(&project, &scope);
    let first = fixture_call(&project, &source, &factory, &namespace, &scope, "one").unwrap();
    let SubmissionOutcome::Invoked { run_id, outcome } = first else {
        panic!("failed domain run should retain an uncertain marker: {first:?}")
    };
    assert_eq!(outcome.marker.outcome, MarkerOutcome::Failed);
    assert_eq!(outcome.marker.effect_state, EffectState::Uncertain);
    assert_eq!(factory.fetches.load(Ordering::SeqCst), 1);
    assert!(
        project
            .join(".vibe/lifecycle-outcomes")
            .join(format!("{run_id}.install-entered.json"))
            .is_file()
    );
    let cache = vibe_registry::store::store_root().unwrap();
    assert!(cache.join("org.example/fixture/v1.1.0/README.md").is_file());
    assert!(!project.join("vibe.lock").exists());
    assert!(
        !project
            .join("vibevm/vibedeps/org.example.fixture/1.1.0")
            .exists()
    );
    assert_eq!(
        fixture_call(&project, &source, &factory, &namespace, &scope, "one").unwrap(),
        SubmissionOutcome::Replayed { run_id, outcome }
    );
    assert_eq!(factory.fetches.load(Ordering::SeqCst), 1);
}

#[test]
fn changed_extra_or_missing_cached_bytes_refuse_before_slot_lock_and_boot() {
    let _guard = fixture_guard();
    assert!(vibe_test_support::isolated_home().is_some());
    for fault in [CacheFault::Changed, CacheFault::Extra, CacheFault::Missing] {
        let (_dir, project, source) = setup();
        let registry = source
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let mut factory = FixtureFactory::new(registry);
        factory.cache_fault = fault;
        let scope = scope();
        let namespace = issue(&project, &scope);
        let first = fixture_call(&project, &source, &factory, &namespace, &scope, "one").unwrap();
        let SubmissionOutcome::Invoked { run_id, outcome } = first else {
            panic!("mismatched cache must retain uncertain result: {first:?}")
        };
        assert_eq!(outcome.marker.outcome, MarkerOutcome::Failed);
        assert_eq!(outcome.marker.effect_state, EffectState::Uncertain);
        assert!(
            project
                .join(".vibe/lifecycle-outcomes")
                .join(format!("{run_id}.install-entered.json"))
                .is_file()
        );
        assert_eq!(factory.fetches.load(Ordering::SeqCst), 1);
        assert!(!project.join("vibe.lock").exists());
        assert!(
            !project
                .join("vibevm/vibedeps/org.example.fixture/1.1.0")
                .exists()
        );
        assert!(!project.join("vibevm/vibespecs/boot/INDEX.md").exists());
        assert_eq!(
            fixture_call(&project, &source, &factory, &namespace, &scope, "one").unwrap(),
            SubmissionOutcome::Replayed { run_id, outcome }
        );
        assert_eq!(factory.fetches.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn forged_registry_content_hash_refuses_before_lock_and_slot() {
    let _guard = fixture_guard();
    let (_dir, project, source) = setup();
    let registry = source
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let mut factory = FixtureFactory::new(registry);
    factory.cache_fault = CacheFault::WrongHash;
    let scope = scope();
    let namespace = issue(&project, &scope);
    let first = fixture_call(&project, &source, &factory, &namespace, &scope, "one").unwrap();
    let SubmissionOutcome::Invoked { run_id, outcome } = first else {
        panic!("wrong hash must retain uncertain result: {first:?}")
    };
    assert_eq!(outcome.marker.outcome, MarkerOutcome::Failed);
    assert_eq!(outcome.marker.effect_state, EffectState::Uncertain);
    assert_eq!(factory.fetches.load(Ordering::SeqCst), 1);
    assert!(
        project
            .join(".vibe/lifecycle-outcomes")
            .join(format!("{run_id}.install-entered.json"))
            .is_file()
    );
    assert!(!project.join("vibe.lock").exists());
    assert!(
        !project
            .join("vibevm/vibedeps/org.example.fixture/1.1.0")
            .exists()
    );
    assert!(!project.join("vibevm/vibespecs/boot/INDEX.md").exists());
    assert_eq!(
        fixture_call(&project, &source, &factory, &namespace, &scope, "one").unwrap(),
        SubmissionOutcome::Replayed { run_id, outcome }
    );
    assert_eq!(factory.fetches.load(Ordering::SeqCst), 1);
}

#[test]
fn cache_drift_at_confirm_is_uncertain_before_apply_and_never_retried() {
    let _guard = fixture_guard();
    let (_dir, project, source) = setup();
    let registry = source
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let factory = FixtureFactory::new(registry);
    let scope = scope();
    let namespace = issue(&project, &scope);
    let cache_file = vibe_registry::store::store_root()
        .unwrap()
        .join("org.example/fixture/v1.1.0/README.md");
    let confirm = MutatingConfirm {
        cache_file,
        calls: AtomicUsize::new(0),
    };
    let first = install::submit_one_package_fixture_install_with_hook(
        &project,
        &source,
        &factory,
        &namespace,
        &scope,
        "one",
        fixture_request(),
        TargetOs::current().expect("supported fixture platform"),
        "2026-09-30T00:00:05Z".parse().unwrap(),
        None,
        Some(&confirm),
    )
    .unwrap();
    let SubmissionOutcome::Invoked { run_id, outcome } = first else {
        panic!("post-fetch drift must retain an uncertain result: {first:?}")
    };
    assert_eq!(outcome.marker.outcome, MarkerOutcome::Failed);
    assert_eq!(outcome.marker.effect_state, EffectState::Uncertain);
    assert_eq!(confirm.calls.load(Ordering::SeqCst), 1);
    assert_eq!(factory.fetches.load(Ordering::SeqCst), 1);
    assert!(
        project
            .join(".vibe/lifecycle-outcomes")
            .join(format!("{run_id}.install-entered.json"))
            .is_file()
    );
    assert!(!project.join("vibe.lock").exists());
    assert!(
        !project
            .join("vibevm/vibedeps/org.example.fixture/1.1.0")
            .exists()
    );
    assert!(!project.join("vibevm/vibespecs/boot/INDEX.md").exists());
    assert_eq!(
        fixture_call(&project, &source, &factory, &namespace, &scope, "one").unwrap(),
        SubmissionOutcome::Replayed { run_id, outcome }
    );
    assert_eq!(confirm.calls.load(Ordering::SeqCst), 1);
    assert_eq!(factory.fetches.load(Ordering::SeqCst), 1);
}

#[cfg(unix)]
#[test]
fn symlinked_cache_child_refuses_before_slot_lock_and_boot() {
    let _guard = fixture_guard();
    let (_dir, project, source) = setup();
    let registry = source
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let mut factory = FixtureFactory::new(registry);
    factory.cache_fault = CacheFault::Symlink;
    let scope = scope();
    let namespace = issue(&project, &scope);
    let first = fixture_call(&project, &source, &factory, &namespace, &scope, "one").unwrap();
    let SubmissionOutcome::Invoked { outcome, .. } = first else {
        panic!("symlinked cache must retain uncertain result: {first:?}")
    };
    assert_eq!(outcome.marker.outcome, MarkerOutcome::Failed);
    assert_eq!(factory.fetches.load(Ordering::SeqCst), 1);
    assert!(!project.join("vibe.lock").exists());
    assert!(
        !project
            .join("vibevm/vibedeps/org.example.fixture/1.1.0")
            .exists()
    );
    assert!(!project.join("vibevm/vibespecs/boot/INDEX.md").exists());
}

#[test]
fn one_package_install_is_staged_then_retained_and_replayed_without_second_effect() {
    let _guard = fixture_guard();
    assert!(vibe_test_support::isolated_home().is_some());
    let (_dir, project, source) = setup();
    let factory = FixtureFactory::new(
        source
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf(),
    );
    let scope = scope();
    let namespace = issue(&project, &scope);
    let first = fixture_call(&project, &source, &factory, &namespace, &scope, "one").unwrap();
    let SubmissionOutcome::Invoked { run_id, outcome } = first else {
        panic!("one-package fixture must invoke")
    };
    assert_eq!(outcome.marker.outcome, MarkerOutcome::Completed);
    assert!(
        project
            .join(".vibe/lifecycle-outcomes")
            .join(format!("{run_id}.install-entered.json"))
            .is_file()
    );
    assert!(
        project
            .join("vibevm/vibedeps/org.example.fixture/1.1.0/README.md")
            .is_file()
    );
    assert!(project.join("vibe.lock").is_file());
    assert!(project.join("vibevm/vibespecs/boot/INDEX.md").is_file());
    assert_eq!(factory.fetches.load(Ordering::SeqCst), 1);
    let lock_before = std::fs::read(project.join("vibe.lock")).unwrap();
    let boot = project.join("vibevm/vibespecs/boot/INDEX.md");
    std::fs::write(&boot, b"after-first-run sentinel").unwrap();
    assert_eq!(
        fixture_call(&project, &source, &factory, &namespace, &scope, "one").unwrap(),
        SubmissionOutcome::Replayed { run_id, outcome }
    );
    assert_eq!(factory.fetches.load(Ordering::SeqCst), 1);
    assert_eq!(
        std::fs::read(project.join("vibe.lock")).unwrap(),
        lock_before
    );
    assert_eq!(std::fs::read(&boot).unwrap(), b"after-first-run sentinel");
}

#[test]
fn another_exact_local_package_uses_its_own_reserved_install_plan() {
    let _guard = fixture_guard();
    assert!(vibe_test_support::isolated_home().is_some());
    let (_dir, project, original) = setup();
    let registry = original
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let pin = PackageRef::parse("flow:org.example/second@=1.1.0").unwrap();
    let source = registry.join("org.example/second/v1.1.0");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(
        source.join("vibe.toml"),
        "[package]\ngroup = 'org.example'\nname = 'second'\nkind = 'flow'\nversion = '1.1.0'\n",
    )
    .unwrap();
    std::fs::write(source.join("README.md"), b"# Second\n").unwrap();
    std::fs::write(project.join("vibe.toml"),
        "[project]\nname = 'another-consumer'\nversion = '2.0.0'\n[requires.packages]\n'flow:org.example/second' = '=1.1.0'\n").unwrap();
    let factory = FixtureFactory::with_pin(registry, pin);
    let scope = scope();
    let namespace = issue(&project, &scope);
    let call = || {
        submit_one_package_local_install(
            &project,
            &factory,
            &namespace,
            &scope,
            "second",
            fixture_request(),
            TargetOs::current().unwrap(),
            "2026-09-30T00:00:05Z".parse().unwrap(),
        )
    };
    let first = call().unwrap();
    let SubmissionOutcome::Invoked { run_id, outcome } = first else {
        panic!("second package must invoke one Install: {first:?}")
    };
    assert_eq!(outcome.marker.outcome, MarkerOutcome::Completed);
    assert!(
        project
            .join("vibevm/vibedeps/org.example.second/1.1.0")
            .is_dir()
    );
    assert_eq!(factory.fetches.load(Ordering::SeqCst), 1);
    assert_eq!(
        call().unwrap(),
        SubmissionOutcome::Replayed { run_id, outcome }
    );
    assert_eq!(factory.fetches.load(Ordering::SeqCst), 1);
}

#[test]
fn local_source_pin_must_match_project_requirement_before_reservation() {
    let _guard = fixture_guard();
    let (_dir, project, original) = setup();
    let registry = original
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let source = registry.join("org.example/second/v1.1.0");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(
        source.join("vibe.toml"),
        "[package]\ngroup = 'org.example'\nname = 'second'\nkind = 'flow'\nversion = '1.1.0'\n",
    )
    .unwrap();
    std::fs::write(source.join("README.md"), b"# Second\n").unwrap();
    let factory = FixtureFactory::with_pin(
        registry,
        PackageRef::parse("flow:org.example/second@=1.1.0").unwrap(),
    );
    let scope = scope();
    let namespace = issue(&project, &scope);
    let error = submit_one_package_local_install(
        &project,
        &factory,
        &namespace,
        &scope,
        "mismatch",
        fixture_request(),
        TargetOs::current().unwrap(),
        "2026-09-30T00:00:05Z".parse().unwrap(),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("unreviewed root manifest"), "{error}");
    let lease = LifecycleLease::acquire(&project).unwrap();
    assert!(
        SubmissionReservations::inspect_existing_key(&lease, &namespace, &scope, "mismatch")
            .unwrap()
            .is_none()
    );
    assert_eq!(factory.fetches.load(Ordering::SeqCst), 0);
    assert!(!project.join("vibe.lock").exists());
}

#[test]
fn changed_source_subject_conflicts_with_same_key_without_second_fetch() {
    let _guard = fixture_guard();
    assert!(vibe_test_support::isolated_home().is_some());
    let (_dir, project, source) = setup();
    let registry = source
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let factory = FixtureFactory::new(registry.clone());
    let scope = scope();
    let namespace = issue(&project, &scope);
    assert!(matches!(
        fixture_call(&project, &source, &factory, &namespace, &scope, "one").unwrap(),
        SubmissionOutcome::Invoked { .. }
    ));
    std::fs::write(source.join("README.md"), b"changed fixture source\n").unwrap();
    let changed_factory = FixtureFactory::new(registry);
    let mismatch = fixture_call(
        &project,
        &source,
        &changed_factory,
        &namespace,
        &scope,
        "one",
    )
    .unwrap_err()
    .to_string();
    assert!(mismatch.contains("different request or plan"), "{mismatch}");
    assert_eq!(factory.fetches.load(Ordering::SeqCst), 1);
}

#[test]
fn project_authors_refuse_before_reservation_or_source_port() {
    let _guard = fixture_guard();
    assert!(vibe_test_support::isolated_home().is_some());
    let (_dir, project, source) = setup();
    std::fs::write(
        project.join("vibe.toml"),
        install::FIXTURE_PROJECT_MANIFEST.replace(
            "version = '1.1.0'",
            "version = '1.1.0'\nauthors = ['Someone']",
        ),
    )
    .unwrap();
    let registry = source
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let factory = FixtureFactory::new(registry);
    let scope = scope();
    let namespace = issue(&project, &scope);
    let refused = fixture_call(&project, &source, &factory, &namespace, &scope, "one")
        .unwrap_err()
        .to_string();
    assert!(refused.contains("simple project identity"), "{refused}");
    assert_eq!(factory.builds.load(Ordering::SeqCst), 0);
    assert!(!project.join(".vibe/lifecycle-outcomes").exists());
    assert!(!project.join("vibe.lock").exists());
}

#[test]
fn source_hook_declaration_refuses_before_reservation_or_fetch() {
    let _guard = fixture_guard();
    assert!(vibe_test_support::isolated_home().is_some());
    let (_dir, project, source) = setup();
    std::fs::write(
        source.join("vibe.toml"),
        format!(
            "{}\n[hooks]\npre-install = 'hooks/prepare'\n",
            install::FIXTURE_PACKAGE_MANIFEST
        ),
    )
    .unwrap();
    let registry = source
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let factory = FixtureFactory::new(registry);
    let scope = scope();
    let namespace = issue(&project, &scope);
    let refused = fixture_call(&project, &source, &factory, &namespace, &scope, "one")
        .unwrap_err()
        .to_string();
    assert!(refused.contains("unreviewed declarations"), "{refused}");
    assert_eq!(factory.builds.load(Ordering::SeqCst), 0);
    assert!(!project.join(".vibe/lifecycle-outcomes").exists());
}

#[test]
fn factory_bound_to_another_source_root_refuses_before_reservation() {
    let _guard = fixture_guard();
    let (_dir, project, source) = setup();
    let foreign = tempfile::tempdir().unwrap();
    let foreign_source = foreign.path().join("org.example/fixture/v1.1.0");
    std::fs::create_dir_all(&foreign_source).unwrap();
    std::fs::write(
        foreign_source.join("vibe.toml"),
        install::FIXTURE_PACKAGE_MANIFEST,
    )
    .unwrap();
    std::fs::write(foreign_source.join("README.md"), b"# Fixture\n").unwrap();
    let factory = FixtureFactory::new(foreign.path().to_path_buf());
    let scope = scope();
    let namespace = issue(&project, &scope);
    let refused = fixture_call(&project, &source, &factory, &namespace, &scope, "one")
        .unwrap_err()
        .to_string();
    assert!(refused.contains("differs from witnessed root"), "{refused}");
    assert_eq!(factory.builds.load(Ordering::SeqCst), 0);
    assert!(!project.join(".vibe/lifecycle-outcomes").exists());
}
