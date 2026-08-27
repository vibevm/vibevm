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
use std::sync::Mutex;

use vibe_core::manifest::{Lockfile, Manifest};
use vibe_core::{PackageRef, VersionSpec};
use vibe_install::{InstallRequest, InstallSource, NullObserver, Plan};
use vibe_registry::{CachedPackage, RegistryError};
use vibe_resolver::{FeatureRequest, ResolvedGraph, SolveError};
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
}

impl InstallSource for RecordingSource {
    fn solve(&self, roots: &[PackageRef]) -> Result<ResolvedGraph, SolveError> {
        let mut seen: Vec<String> = roots
            .iter()
            .map(|root| match root.group.as_ref() {
                Some(group) => format!("{group}/{}", root.name),
                None => root.name.to_string(),
            })
            .collect();
        seen.sort();
        self.solves.lock().unwrap().push(seen);
        Err(SolveError::CapabilityUnmet {
            capability: "recorded".to_string(),
            requirer: "the prepared-planning red".to_string(),
        })
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
    }
}

fn tools_ref() -> PackageRef {
    PackageRef::parse("flow:org.demo/tools@^1.0").expect("a well-formed pkgref")
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

/// Case-c migration mutates BOTH supplied values, and the tree gets a delta.
///
/// PROP-002 §2.7: an empty entry manifest whose lockfile still carries
/// `meta.root_dependencies` is seeded from that snapshot and persisted before
/// the solve. The wrapper then re-discovered so the root union would see it;
/// the prepared seam instead applies the same concrete entries to the exact
/// selected node — which this asserts three ways: the file was written, the
/// caller's own `Workspace` value now carries the entry, and the solve saw it.
///
/// The delta is applied to the node, never as a wholesale copy of the raw
/// table: an unrelated in-memory value on that node — here the expanded
/// version a `[workspace.versions]` placeholder would have resolved to —
/// survives the migration.
#[test]
fn case_c_migration_writes_the_manifest_and_deltas_the_supplied_node() {
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
        BTreeSet::from([
            "org.demo/expanded".to_string(),
            "org.demo/tools".to_string()
        ]),
        "the migrated entry AND the finalised-only one both reached the solve",
    );
    let names: Vec<String> = workspace
        .root_manifest
        .requires
        .packages
        .iter()
        .map(|p| p.name.to_string())
        .collect();
    assert!(
        names.contains(&"tools".to_string()) && names.contains(&"expanded".to_string()),
        "the caller's tree got the migration as a DELTA, keeping what it had: {names:?}",
    );
    let persisted = Manifest::read(project.path().join("vibe.toml"))
        .expect("the migration wrote a parseable manifest");
    assert_eq!(
        persisted.requires.packages.len(),
        1,
        "and the FILE got the lockfile snapshot verbatim — the finalised-only \
         entry is in-memory state, not something to persist",
    );
    assert_eq!(persisted.requires.packages[0].name.to_string(), "tools");
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
