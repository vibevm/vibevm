//! Reds for the PREPARED planning seam.
//!
//! `plan_prepared_with_spec_format` holds the real planning algorithm and must
//! reach the disk for neither the selected `vibe.toml` nor the workspace tree.
//! Every test here makes the prepared values disagree with disk — by editing
//! them in memory, or by destroying the file after preparation — and requires
//! the prepared values to be the ones that decide.
//!
//! The mutation they exist to kill is a single line: any `Manifest::read` or
//! `Workspace::discover` reintroduced inside the prepared function. Since the
//! compatibility wrapper still performs exactly those two reads, each test
//! runs it over the same tree as a control: it is the "before" the prepared
//! seam is supposed to differ from.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::progress::{Progress, ProgressEvent, ProgressEventKind, ProgressObserver};
use vibe_core::{PackageRef, VersionSpec};
use vibe_install::{InstallRequest, InstallSource, NullObserver, Plan, PlanEvent, PlanObserver};
use vibe_registry::source_candidates::RetainedSourceCandidate;
use vibe_registry::{CachedPackage, PackageAcquisition, RegistryError};
use vibe_resolver::{
    FeatureRequest, ResolvedGraph, SolveError, SourceResolvedGraph, SourceSolveLimits,
    SourceSolveRequest,
};
use vibe_workspace::Workspace;

/// A source that RECORDS the root set planning handed it, then refuses.
///
/// Refusing is the point. Whether the solve ran at all, and with which roots,
/// is the only observable that distinguishes "planning read the prepared tree"
/// from "planning re-read disk" — and a refusal reaches it without a registry,
/// a network, or a fixture package.
#[derive(Default)]
struct RecordingSource {
    solves: Mutex<Vec<Vec<String>>>,
}

struct EventRecorder {
    started: Arc<AtomicBool>,
    events: Mutex<Vec<ProgressEventKind>>,
}

impl ProgressObserver for EventRecorder {
    fn observe(&self, event: ProgressEvent) {
        if matches!(event.kind, ProgressEventKind::Started { .. }) {
            self.started.store(true, Ordering::SeqCst);
        }
        self.events.lock().unwrap().push(event.kind);
    }
}

struct ObservedPlan {
    progress: Progress,
}

impl PlanObserver for ObservedPlan {
    fn on(&self, _event: PlanEvent) {}

    fn progress(&self) -> Progress {
        self.progress.clone()
    }
}

struct StartCheckingSource {
    started: Arc<AtomicBool>,
}

impl StartCheckingSource {
    fn refuse(&self) -> SolveError {
        assert!(
            self.started.load(Ordering::SeqCst),
            "progress must start before the blocking resolver call"
        );
        SolveError::CapabilityUnmet {
            capability: "expected refusal".into(),
            requirer: "progress fixture".into(),
        }
    }
}

impl InstallSource for StartCheckingSource {
    fn source_solve_limits(&self) -> Result<SourceSolveLimits, SolveError> {
        Ok(SourceSolveLimits::standard_operation())
    }

    fn solve_metadata(
        &self,
        _request: &SourceSolveRequest<'_>,
    ) -> Result<SourceResolvedGraph, SolveError> {
        Err(self.refuse())
    }

    fn acquire_candidate_for_install(
        &self,
        _candidate: &RetainedSourceCandidate,
        _store_root: &Path,
        _expected_hash: Option<&str>,
        _slot: &Path,
        _progress: &Progress,
    ) -> Result<PackageAcquisition, RegistryError> {
        unreachable!()
    }

    fn solve(&self, _roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        Err(self.refuse())
    }

    fn resolve_and_fetch(
        &self,
        _pkgref: &PackageRef,
        _store_root: &Path,
        _expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        unreachable!()
    }

    fn manifest_of(&self, _pkg: &PackageRef) -> Result<Manifest, SolveError> {
        unreachable!()
    }

    fn solve_masked(
        &self,
        roots: &[PackageRef],
        _blocked: &BTreeSet<(String, String)>,
    ) -> Result<ResolvedGraph, SolveError> {
        self.solve(roots)
    }

    fn materialise_in_place(
        &self,
        _pkgref: &PackageRef,
        _slot: &Path,
    ) -> Result<vibe_registry::InPlaceMaterialised, RegistryError> {
        unreachable!()
    }
}

impl RecordingSource {
    /// Every solve this source was asked to run, as sorted `group/name` lists.
    fn solves(&self) -> Vec<Vec<String>> {
        self.solves.lock().unwrap().clone()
    }

    /// The union of every root any solve saw — the question all these tests
    /// actually ask, since the pin-held fallback may solve twice.
    fn every_root(&self) -> BTreeSet<String> {
        self.solves().into_iter().flatten().collect()
    }

    /// Record one solve's roots, then refuse it.
    fn record(&self, roots: &[PackageRef]) -> SolveError {
        let mut seen: Vec<String> = roots
            .iter()
            .map(|root| match root.group.as_ref() {
                Some(group) => format!("{group}/{}", root.name),
                None => root.name.to_string(),
            })
            .collect();
        seen.sort();
        self.solves.lock().unwrap().push(seen);
        SolveError::CapabilityUnmet {
            capability: "recorded".to_string(),
            requirer: "the prepared-planning red".to_string(),
        }
    }
}

impl InstallSource for RecordingSource {
    fn source_solve_limits(&self) -> Result<SourceSolveLimits, SolveError> {
        Ok(SourceSolveLimits::standard_operation())
    }

    /// The held root set is what the legacy `solve(roots)` received. This
    /// source consumes no package configuration, so a configured request takes
    /// the legacy configured route, which refuses before any solve is recorded.
    fn solve_metadata(
        &self,
        request: &SourceSolveRequest<'_>,
    ) -> Result<SourceResolvedGraph, SolveError> {
        if let Some(snapshot) = request.configuration {
            self.solve_configured(request.held_roots, snapshot, request.blocked)?;
        }
        Err(self.record(request.held_roots))
    }

    fn acquire_candidate_for_install(
        &self,
        _candidate: &RetainedSourceCandidate,
        _store_root: &Path,
        _expected_hash: Option<&str>,
        _slot: &Path,
        _progress: &Progress,
    ) -> Result<PackageAcquisition, RegistryError> {
        unreachable!("the solve above refuses before anything is acquired")
    }

    fn solve(&self, roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        Err(self.record(roots))
    }

    fn resolve_and_fetch(
        &self,
        _pkgref: &PackageRef,
        _store_root: &Path,
        _expected_hash: Option<&str>,
    ) -> Result<CachedPackage, RegistryError> {
        unreachable!("the solve above refuses before anything is fetched")
    }

    fn manifest_of(&self, _pkg: &PackageRef) -> Result<Manifest, SolveError> {
        unreachable!("the solve above refuses before any manifest is read")
    }

    fn solve_masked(
        &self,
        roots: &[PackageRef],
        _blocked: &BTreeSet<(String, String)>,
    ) -> Result<ResolvedGraph, SolveError> {
        self.solve(roots)
    }

    fn materialise_in_place(
        &self,
        _pkgref: &PackageRef,
        _slot: &Path,
    ) -> Result<vibe_registry::InPlaceMaterialised, RegistryError> {
        unreachable!("the solve above refuses before anything is materialised")
    }
}

/// A project that declares NOTHING. Its root union is empty, so planning it as
/// written short-circuits to `Plan::Fresh` without ever asking the source —
/// which makes "the source was asked" a clean signal that something other than
/// the file on disk supplied the roots.
fn bare_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname = \"demo\"\ngroup = \"org.demo\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    dir
}

/// Read the pair a prepared caller owns: the raw selected manifest and the
/// finalised tree.
fn prepare(root: &Path) -> (Manifest, Workspace) {
    let manifest = Manifest::read(root.join(Manifest::FILENAME)).expect("the manifest reads");
    let workspace = Workspace::discover(root).expect("the tree discovers");
    (manifest, workspace)
}

fn request() -> InstallRequest {
    InstallRequest {
        roots: Vec::new(),
        features: FeatureRequest::default(),
        language: None,
        exact: false,
        generated_by: "vibe test".to_string(),
        offline: false,
        no_recommends: false,
    }
}

fn tools_ref() -> PackageRef {
    PackageRef::parse("flow:org.demo/tools@^1.0").expect("a well-formed pkgref")
}

#[test]
fn progress_starts_before_the_resolver_blocks_and_fails_honestly() {
    let project = bare_project();
    let (mut manifest, mut workspace) = prepare(project.path());
    workspace.root_manifest.requires.packages.push(tools_ref());

    let started = Arc::new(AtomicBool::new(false));
    let recorder = Arc::new(EventRecorder {
        started: Arc::clone(&started),
        events: Mutex::new(Vec::new()),
    });
    let observer = ObservedPlan {
        progress: Progress::new(recorder.clone()),
    };
    let source = StartCheckingSource { started };

    let error = vibe_install::plan_prepared_with_spec_format(
        &source,
        project.path(),
        &mut manifest,
        &mut workspace,
        request(),
        vibe_core::manifest::SpecFormat::Mixed,
        &observer,
    )
    .expect_err("the fixture resolver refuses");
    assert!(error.to_string().contains("expected refusal"));
    let events = recorder.events.lock().unwrap();
    assert!(matches!(
        events.first(),
        Some(ProgressEventKind::Started { label, .. }) if label == "Resolving dependencies"
    ));
    assert!(matches!(
        events.last(),
        Some(ProgressEventKind::Failed { message, .. }) if message == "dependency resolution failed"
    ));
}

fn plan_prepared(
    source: &RecordingSource,
    root: &Path,
    manifest: &mut Manifest,
    workspace: &mut Workspace,
    request: InstallRequest,
) -> Result<Plan, vibe_install::Error> {
    vibe_install::plan_prepared_with_spec_format(
        source,
        root,
        manifest,
        workspace,
        request,
        vibe_core::manifest::SpecFormat::Mixed,
        &NullObserver,
    )
}

/// The root union — and the freshness check behind it — read the SUPPLIED
/// tree.
///
/// Disk declares nothing. The prepared tree is edited in memory to declare one
/// package, and that edit alone must carry planning all the way into a solve.
/// Two production lines die with this test:
///
/// * the root union: rediscover there and the in-memory entry vanishes, the
///   union is empty, and `##EMPTY-REQUIRES-IS-A-NO-OP` returns `Fresh` before
///   the source is ever consulted;
/// * the freshness check: rediscover *only* there and the union is still
///   non-empty, but the rediscovered tree declares nothing that the (empty)
///   lock fails to satisfy, so freshness answers `Fresh` and short-circuits.
///
/// Either way the recorded solves stay empty, which is what this asserts.
#[test]
fn the_root_union_reads_the_supplied_tree_not_the_file() {
    let project = bare_project();
    let (mut manifest, mut workspace) = prepare(project.path());
    workspace.root_manifest.requires.packages.push(tools_ref());

    let source = RecordingSource::default();
    let error = plan_prepared(
        &source,
        project.path(),
        &mut manifest,
        &mut workspace,
        request(),
    )
    .expect_err("the recording source refuses every solve");
    assert!(
        error.to_string().contains("recorded"),
        "planning failed at the SOLVE, not before it: {error}",
    );
    assert_eq!(
        source.every_root(),
        BTreeSet::from(["org.demo/tools".to_string()]),
        "the in-memory declaration is the root set planning solved: {:?}",
        source.solves(),
    );

    // The control: the compatibility wrapper re-reads the same tree from disk,
    // where nothing is declared, and never reaches the source at all.
    let before = source.solves().len();
    let plan = vibe_install::plan_with_spec_format(
        &source,
        project.path(),
        request(),
        vibe_core::manifest::SpecFormat::Mixed,
        &NullObserver,
    )
    .expect("an undeclared world is fresh, not an error");
    assert!(matches!(plan, Plan::Fresh), "the wrapper sees the file");
    assert_eq!(
        source.solves().len(),
        before,
        "and asked the source nothing, which is the difference being proved",
    );
}

/// The EXPLICIT-pkgref union reads the supplied tree too.
///
/// `##EXPLICIT-PKGREF-FULL-SOLVE`: a named package JOINS the manifest union
/// rather than replacing it, so a partial solve can never reach apply. That
/// join iterates the workspace, and this proves it iterates the prepared one:
/// the named root arrives through the request, the second root exists only in
/// memory, and the solve must see both.
#[test]
fn the_explicit_pkgref_union_reads_the_supplied_tree() {
    let project = bare_project();
    let (mut manifest, mut workspace) = prepare(project.path());
    workspace.root_manifest.requires.packages.push(tools_ref());

    let mut explicit = request();
    explicit.roots = vec![PackageRef::parse("flow:org.demo/other@^2.0").unwrap()];

    let source = RecordingSource::default();
    plan_prepared(
        &source,
        project.path(),
        &mut manifest,
        &mut workspace,
        explicit,
    )
    .expect_err("the recording source refuses every solve");
    assert_eq!(
        source.every_root(),
        BTreeSet::from(["org.demo/other".to_string(), "org.demo/tools".to_string()]),
        "the named root JOINED the prepared tree's union: {:?}",
        source.solves(),
    );
}

/// A selected manifest destroyed AFTER preparation changes nothing.
///
/// The sharpest form of the claim: there is no file left to fall back to, so a
/// prepared plan can only come from the values in hand. The wrapper, on the
/// same tree, refuses — which is what makes the line above a real difference
/// rather than a tautology.
#[test]
fn a_deleted_selected_manifest_does_not_reach_prepared_planning() {
    let project = bare_project();
    let (mut manifest, mut workspace) = prepare(project.path());
    std::fs::remove_file(project.path().join("vibe.toml")).unwrap();

    let source = RecordingSource::default();
    let plan = plan_prepared(
        &source,
        project.path(),
        &mut manifest,
        &mut workspace,
        request(),
    )
    .expect("the prepared values still describe an undeclared world");
    assert!(matches!(plan, Plan::Fresh));

    assert!(
        vibe_install::plan_with_spec_format(
            &source,
            project.path(),
            request(),
            vibe_core::manifest::SpecFormat::Mixed,
            &NullObserver,
        )
        .is_err(),
        "the wrapper really does read the file, and the file really is gone",
    );
}

/// A CORRUPT selected manifest is refused by the wrapper and unseen by the
/// prepared seam — and the prepared seam still solves the roots it was given.
///
/// Deletion proves "no read"; corruption proves "no read even when a parse
/// would succeed at producing a *different* answer". Together they cover both
/// halves of the reintroduced-read mutation.
#[test]
fn a_corrupt_selected_manifest_does_not_reach_prepared_planning() {
    let project = bare_project();
    let (mut manifest, mut workspace) = prepare(project.path());
    workspace.root_manifest.requires.packages.push(tools_ref());
    std::fs::write(project.path().join("vibe.toml"), "[project\nbroken\n").unwrap();

    let source = RecordingSource::default();
    plan_prepared(
        &source,
        project.path(),
        &mut manifest,
        &mut workspace,
        request(),
    )
    .expect_err("the recording source refuses every solve");
    assert_eq!(
        source.every_root(),
        BTreeSet::from(["org.demo/tools".to_string()]),
        "planning solved the prepared union while the file was unparseable",
    );

    assert!(
        vibe_install::plan_with_spec_format(
            &source,
            project.path(),
            request(),
            vibe_core::manifest::SpecFormat::Mixed,
            &NullObserver,
        )
        .is_err(),
        "the wrapper observes disk and refuses",
    );
}

/// Finalized declarations are authoritative even when the raw selected table
/// is empty. PROP-009 INSTALL-CONSUMER-RECORDING forbids legacy seeding in that
/// case: a snapshot must not become a second consumer of an existing member's
/// dependency or override an already-expanded version variable.
#[test]
fn prepared_declarations_prevent_legacy_snapshot_seeding() {
    let project = bare_project();
    let mut lockfile = Lockfile::empty("vibe test", "2026-08-27T00:00:00Z");
    lockfile.meta.root_dependencies = vec![tools_ref()];
    lockfile
        .write(project.path().join("vibe.lock"))
        .expect("the seed lockfile writes");

    let (mut manifest, mut workspace) = prepare(project.path());
    // A value that exists only in the finalised tree. A wholesale raw copy
    // would drop it; a delta cannot.
    let expanded = PackageRef::new(
        Some(vibe_core::PackageKind::Flow),
        Some(vibe_core::Group::parse("org.demo").unwrap()),
        "expanded".to_string(),
        VersionSpec::Latest,
    )
    .unwrap();
    workspace.root_manifest.requires.packages.push(expanded);

    let source = RecordingSource::default();
    plan_prepared(
        &source,
        project.path(),
        &mut manifest,
        &mut workspace,
        request(),
    )
    .expect_err("the recording source refuses every solve");

    assert_eq!(
        source.every_root(),
        BTreeSet::from(["org.demo/expanded".to_string()]),
        "the finalized declaration alone reached the solve",
    );
    let names: Vec<String> = workspace
        .root_manifest
        .requires
        .packages
        .iter()
        .map(|p| p.name.to_string())
        .collect();
    assert_eq!(names, ["expanded"]);
    let persisted = Manifest::read(project.path().join("vibe.toml"))
        .expect("the original manifest remains parseable");
    assert!(persisted.requires.packages.is_empty());
}

/// `Plan::Fresh` leaves the caller's workspace usable.
///
/// The fresh fast path is the one that returns without producing a
/// `PlannedInstall`, so the tree the caller still holds is the only tree left.
/// Taking it by `&mut` rather than by value is what guarantees that; this
/// pins the guarantee so a signature change to by-value fails a test rather
/// than a review.
#[test]
fn a_fresh_plan_leaves_the_callers_workspace_available() {
    let project = bare_project();
    let (mut manifest, mut workspace) = prepare(project.path());
    let source = RecordingSource::default();
    let plan = plan_prepared(
        &source,
        project.path(),
        &mut manifest,
        &mut workspace,
        request(),
    )
    .expect("an undeclared world is fresh");
    assert!(matches!(plan, Plan::Fresh));
    assert_eq!(
        workspace.root,
        Workspace::discover(project.path()).unwrap().root,
        "the caller still owns the tree it prepared",
    );
    assert!(
        workspace.root_manifest.project.is_some(),
        "and it is the whole value, not a husk planning moved out of",
    );
}

#[test]
fn prepared_sibling_configuration_input_refuses_before_solve_or_fresh() {
    let project = bare_project();
    let root = project.path();
    std::fs::write(
        root.join("vibe.toml"),
        "[project]\nname='root'\nversion='0.1.0'\n[workspace]\nmembers=['member']\n",
    )
    .unwrap();
    std::fs::create_dir_all(root.join("member")).unwrap();
    std::fs::write(root.join("member/vibe.toml"),"[project]\nname='member'\nversion='0.1.0'\n[requires.packages.'org.example/target']\nversion='*'\nconfiguration={policy='require-selection',parameters={message='SYN_NOT_IN_DIAGNOSTIC'}}\n").unwrap();
    let (mut manifest, mut workspace) = prepare(root);
    let before = std::fs::read(root.join("vibe.toml")).unwrap();
    let source = RecordingSource::default();
    let error = plan_prepared(&source, root, &mut manifest, &mut workspace, request()).unwrap_err();
    assert!(
        matches!(&error,vibe_install::Error::ConsumerConstraints {declarations,source: vibe_resolver::SolveError::Provider(vibe_resolver::DepProviderError::Configuration(vibe_core::manifest::PackageConfigurationError::Unsupported {..}))} if declarations==&["member/vibe.toml: org.example/target@*"]),
        "{error}"
    );
    assert!(!error.to_string().contains("SYN_NOT_IN_DIAGNOSTIC"));
    assert!(source.solves.lock().unwrap().is_empty());
    assert_eq!(std::fs::read(root.join("vibe.toml")).unwrap(), before);
    assert!(!root.join("vibe.lock").exists());
}

#[test]
fn raw_selected_input_refuses_before_case_c_can_seed_or_write() {
    let project = bare_project();
    let root = project.path();
    let mut lock = Lockfile::empty("test", "2026-11-04T00:00:00Z");
    lock.meta.root_dependencies = vec![tools_ref()];
    lock.write(root.join("vibe.lock")).unwrap();
    let (mut raw, mut workspace) = prepare(root);
    // The prepared raw and finalized views intentionally differ. Even this raw
    // metadata must not disappear through case-c migration or the empty path.
    let supplied=Manifest::parse_str("[project]\nname='demo'\nversion='0.1.0'\n[requires.packages.'org.demo/tools']\nversion='^1'\nconfiguration={policy='require-selection'}\n").unwrap();
    raw.requires.configurations = supplied.requires.configurations;
    let before = std::fs::read(root.join("vibe.toml")).unwrap();
    let lock_before = std::fs::read(root.join("vibe.lock")).unwrap();
    let source = RecordingSource::default();
    assert!(matches!(
        plan_prepared(&source, root, &mut raw, &mut workspace, request()),
        Err(vibe_install::Error::ConfigurationInputUnsupported { .. })
    ));
    assert!(source.solves.lock().unwrap().is_empty());
    assert!(raw.requires.packages.is_empty());
    assert!(workspace.root_manifest.requires.packages.is_empty());
    assert_eq!(std::fs::read(root.join("vibe.toml")).unwrap(), before);
    assert_eq!(std::fs::read(root.join("vibe.lock")).unwrap(), lock_before);
}
