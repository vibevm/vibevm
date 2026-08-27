//! Producer-level reds for the durable prefix a scoped update measures.
//!
//! These drive the REAL producers — `prune_superseded` against a real
//! workspace and real `vibedeps::remove_slot` calls, and the exact draft
//! composition the two failure regions use — rather than the accumulator type
//! on its own. A test that only exercised [`Measured`] would stay green
//! through a producer that collected its removals into a local vector and
//! dropped them on the way out, which is the defect being refused here.

use std::path::Path;

use vibe_core::manifest::Manifest;
use vibe_registry::ResolvedPackage;

use super::*;

fn workspace_at(dir: &Path) -> Workspace {
    std::fs::write(
        dir.join("vibe.toml"),
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .expect("a fixture project manifest");
    Workspace::discover(dir).expect("the fixture tree loads")
}

fn package_manifest(group: &str, name: &str, version: &str) -> Manifest {
    toml::from_str(&format!(
        "[package]\ngroup = \"{group}\"\nname = \"{name}\"\nkind = \"flow\"\n\
         version = \"{version}\"\n"
    ))
    .expect("a valid package manifest")
}

fn cached(group: &str, name: &str, version: &str) -> CachedPackage {
    let dir = std::path::PathBuf::from("/fixture");
    CachedPackage {
        resolved: ResolvedPackage {
            group: Group::parse(group).expect("a valid group"),
            name: name.to_string(),
            version: semver::Version::parse(version).expect("a valid version"),
            source_dir: dir.clone(),
        },
        cache_dir: dir,
        manifest: package_manifest(group, name, version),
        content_hash: format!("sha256:{}", "0".repeat(64)),
        source_uri: "file:///fixture".into(),
        registry_name: None,
        source_ref: None,
        resolved_commit: None,
        overridden: false,
        is_git_source: false,
        is_path_source: false,
        is_embedded: false,
        is_local: false,
        via_redirect: None,
    }
}

/// A lockfile pinning each package at its OLD version — the state a bump moves
/// away from.
fn locked_at(olds: &[CachedPackage]) -> Lockfile {
    let mut lockfile = Lockfile::empty(
        "vibe test".to_string(),
        crate::commands::init::current_timestamp_utc(),
    );
    for old in olds {
        lockfile
            .packages
            .push(super::super::inputs::locked_package(old, &[], None));
    }
    lockfile
}

fn slot(workspace: &Workspace, package: &CachedPackage) -> std::path::PathBuf {
    vibedeps::slot_abs_path(
        &workspace.root,
        &package.resolved.group,
        &package.resolved.name,
        &package.resolved.version,
    )
}

/// The property the accumulator exists for, proved through the real producer.
///
/// Two packages bump. The first superseded slot is an ordinary directory and is
/// really removed; the second is a FILE where a slot directory is expected, so
/// `remove_dir_all` refuses on every platform. The run must report the removal
/// it actually performed.
///
/// A helper that returned `Result<Vec<String>>` and built its vector locally
/// would drop it on this `Err` and report a tree it had already changed as
/// untouched — which is the exact shape this refuses.
#[test]
fn a_prune_that_fails_half_way_keeps_the_slots_it_really_removed() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = workspace_at(dir.path());

    let first_old = cached("org.demo", "first", "0.1.0");
    let second_old = cached("org.demo", "second", "0.1.0");
    let lockfile = locked_at(&[first_old.clone(), second_old.clone()]);

    // A real slot directory, and a slot path that is a file.
    let removable = slot(&workspace, &first_old);
    std::fs::create_dir_all(&removable).unwrap();
    std::fs::write(removable.join("payload.txt"), "x").unwrap();
    let unremovable = slot(&workspace, &second_old);
    std::fs::create_dir_all(unremovable.parent().unwrap()).unwrap();
    std::fs::write(&unremovable, "not a directory").unwrap();

    let updated: Vec<Resolved> = vec![
        (cached("org.demo", "first", "0.2.0"), Vec::new(), None),
        (cached("org.demo", "second", "0.2.0"), Vec::new(), None),
    ];
    let mut measured = Measured::default();
    let error = prune_superseded(&workspace, &lockfile, &updated, &mut measured)
        .expect_err("the second removal cannot succeed");
    assert!(
        format!("{error:#}").contains("superseded"),
        "the failure keeps its own words: {error:#}",
    );

    assert!(
        !removable.exists(),
        "the first slot really is gone from the operator's disk",
    );
    assert_eq!(
        measured.pruned(),
        [vibedeps::slot_rel_path(
            &first_old.resolved.group,
            &first_old.resolved.name,
            &first_old.resolved.version,
        )],
        "and the partial record survived the later failure",
    );
    assert_eq!(
        measured.bumps().len(),
        2,
        "both bumps are facts about the resolution, whatever the removals did",
    );

    // The draft the failure region really builds from it.
    let identity = UpdateIdentity {
        project_root: workspace.root.clone(),
        scope: vibe_wire::generated::update_report::UpdateReportScope::Scoped,
        packages: vec!["org.demo/first".into(), "org.demo/second".into()],
    };
    let report = UpdateDraft::failed(
        &identity,
        2,
        measured.bumps().to_vec(),
        measured.joined(InstallProgress::default()),
        Vec::new(),
    )
    .into_report(None);
    assert!(!report.ok);
    assert!(!report.complete, "a failure never completed");
    assert_eq!(
        report.pruned.len(),
        1,
        "the failed report names the slot this run deleted: {report:?}",
    );
    assert_eq!(report.version_bumps.len(), 2);
}

/// A prune that removes nothing records nothing — `pruned` names paths this run
/// really deleted, not paths it considered.
#[test]
fn an_absent_superseded_slot_is_not_reported_as_pruned() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = workspace_at(dir.path());
    let old = cached("org.demo", "gone", "0.1.0");
    let lockfile = locked_at(&[old]);
    let updated: Vec<Resolved> = vec![(cached("org.demo", "gone", "0.2.0"), Vec::new(), None)];

    let mut measured = Measured::default();
    prune_superseded(&workspace, &lockfile, &updated, &mut measured).expect("nothing to remove");
    assert!(measured.pruned().is_empty());
    assert_eq!(measured.bumps().len(), 1, "the bump still happened");
}

/// Region 2: an in-place slot really advanced, and the run then refused before
/// any lifecycle existed. The failed draft must say the slot moved.
///
/// This is the composition the staging failure closure performs, with the exact
/// values a real `materialise_in_place` produces: an empty
/// `InstallProgress::default()` here would tell the operator to retry against a
/// tree that has already been fetched onto.
#[test]
fn a_failure_before_the_lifecycle_still_reports_the_in_place_slot_it_moved() {
    let mut measured = Measured::default();
    measured.record_in_place("vibedeps/org.demo.tools".into(), true);
    measured.record_in_place("vibedeps/org.demo.quiet".into(), false);

    let identity = UpdateIdentity {
        project_root: std::path::PathBuf::from("/p"),
        scope: vibe_wire::generated::update_report::UpdateReportScope::Scoped,
        packages: vec!["org.demo/tools".into()],
    };
    let report = UpdateDraft::failed(
        &identity,
        2,
        measured.bumps().to_vec(),
        measured.progress(),
        Vec::new(),
    )
    .into_report(None);

    assert!(!report.ok);
    assert!(!report.complete);
    assert_eq!(
        report.materialised,
        ["vibedeps/org.demo.tools"],
        "the advanced working tree is durable and named",
    );
    assert_eq!(report.skipped, ["vibedeps/org.demo.quiet"]);
    assert!(
        report.nodes_regenerated.is_empty(),
        "nothing had compiled yet, and nothing is invented",
    );
}

/// Region 3: a run that produced a successful row and durable nodes, then
/// failed later. Both halves survive, in order, and neither is duplicated.
#[test]
fn a_failure_after_the_lifecycle_joins_the_prefix_with_the_measured_run() {
    let mut measured = Measured::default();
    measured.record_in_place("vibedeps/org.demo.tools".into(), true);
    measured.record_pruned("vibedeps/org.demo.tools/0.1.0".into());
    measured.record_bump("org.demo/tools 0.1.0 -> 0.2.0".into());

    // What the run itself measured: the prune prefix was transferred into it,
    // the materialise pass wrote a slot, and boot really regenerated.
    let run_progress = InstallProgress {
        complete: true,
        fresh: false,
        materialised: vec!["vibedeps/org.demo.tools/0.2.0".into()],
        skipped: Vec::new(),
        pruned: vec!["vibedeps/org.demo.tools/0.1.0".into()],
        nodes_regenerated: vec![".".into()],
    };
    let rows = vec![
        row("slot:pre-install", "ok"),
        row("slot:post-install", "fail"),
    ];

    let identity = UpdateIdentity {
        project_root: std::path::PathBuf::from("/p"),
        scope: vibe_wire::generated::update_report::UpdateReportScope::Scoped,
        packages: vec!["org.demo/tools".into()],
    };
    let report = UpdateDraft::failed(
        &identity,
        1,
        measured.bumps().to_vec(),
        measured.joined(run_progress),
        rows,
    )
    .into_report(None);

    assert!(!report.ok);
    assert!(
        !report.complete,
        "measured progress never makes it complete"
    );
    assert_eq!(
        report.materialised,
        ["vibedeps/org.demo.tools", "vibedeps/org.demo.tools/0.2.0"],
        "chronology preserved, nothing duplicated",
    );
    assert_eq!(
        report.pruned,
        ["vibedeps/org.demo.tools/0.1.0"],
        "exactly one copy of the transferred prune list",
    );
    assert_eq!(report.nodes_regenerated, ["."]);
    let statuses: Vec<&str> = report
        .contributions
        .iter()
        .map(|row| row.status.as_str())
        .collect();
    assert_eq!(
        statuses,
        ["ok", "fail"],
        "the earlier successful row precedes the later failed one",
    );
}

fn row(point: &str, status: &str) -> vibe_install::SlotLifecycleReport {
    vibe_install::SlotLifecycleReport {
        key: format!("org.demo/tools#{point}"),
        point: point.into(),
        handler: "builtin".into(),
        provider: "org.demo/tools".into(),
        tier: "dependency".into(),
        status: status.into(),
        message: None,
        version: None,
        reference: "spec://org.demo/tools".into(),
        flagged: false,
        stdout: None,
        stderr: None,
        stdout_truncated: false,
        stderr_truncated: false,
        slot_target: None,
    }
}
