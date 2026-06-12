//! Unit tests for the install recording helpers.

specmark::scope!("spec://vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use super::recording::{finalize_pkgref_for_manifest, merge_manifest_requires};
use vibe_core::PackageRef;
use vibe_core::manifest::{Manifest, ProjectSection};

fn empty_manifest() -> Manifest {
    Manifest {
        project: Some(ProjectSection {
            name: "demo".to_string(),
            version: "0.0.1".to_string(),
            authors: vec![],
        }),
        ..Default::default()
    }
}

#[test]
fn merge_manifest_requires_appends_new_pkgref() {
    let mut m = empty_manifest();
    let r = PackageRef::parse("flow:wal@^0.1").unwrap();
    let changed = merge_manifest_requires(&mut m, std::slice::from_ref(&r));
    assert!(changed);
    assert_eq!(m.requires.packages.len(), 1);
    assert_eq!(m.requires.packages[0], r);
}

#[test]
fn merge_manifest_requires_idempotent_on_repeat() {
    let mut m = empty_manifest();
    let r = PackageRef::parse("flow:wal@^0.1").unwrap();
    merge_manifest_requires(&mut m, std::slice::from_ref(&r));
    // Second call with the same pkgref must not duplicate the entry
    // and must not mark the manifest dirty.
    let changed_again = merge_manifest_requires(&mut m, std::slice::from_ref(&r));
    assert!(
        !changed_again,
        "second merge of the same pkgref must be a no-op"
    );
    assert_eq!(m.requires.packages.len(), 1);
}

#[test]
fn merge_manifest_requires_overwrites_constraint_change() {
    let mut m = empty_manifest();
    let r1 = PackageRef::parse("flow:wal@^0.1").unwrap();
    merge_manifest_requires(&mut m, std::slice::from_ref(&r1));
    let r2 = PackageRef::parse("flow:wal@=0.2.0").unwrap();
    let changed = merge_manifest_requires(&mut m, std::slice::from_ref(&r2));
    assert!(changed, "constraint change must mark the manifest dirty");
    assert_eq!(m.requires.packages.len(), 1);
    assert_eq!(m.requires.packages[0], r2);
}

fn vsemver(s: &str) -> semver::Version {
    semver::Version::parse(s).unwrap()
}

#[test]
fn finalize_caret_when_cli_had_no_version() {
    // `vibe install flow:wal` → resolves 0.1.0 → manifest gets
    // `flow:wal@^0.1.0`. Same default as Cargo / npm / Poetry.
    let cli = PackageRef::parse("flow:wal").unwrap();
    let out = finalize_pkgref_for_manifest(&cli, &vsemver("0.1.0"), false);
    assert_eq!(out.to_string(), "flow:wal@^0.1.0");
}

#[test]
fn finalize_preserves_explicit_caret() {
    let cli = PackageRef::parse("flow:wal@^0.1").unwrap();
    let out = finalize_pkgref_for_manifest(&cli, &vsemver("0.1.5"), false);
    // CLI form preserved — we don't tighten the operator's
    // explicitly stated constraint.
    assert_eq!(out, cli);
}

#[test]
fn finalize_preserves_explicit_eq() {
    let cli = PackageRef::parse("flow:wal@=0.1.0").unwrap();
    let out = finalize_pkgref_for_manifest(&cli, &vsemver("0.1.0"), false);
    assert_eq!(out, cli);
}

#[test]
fn finalize_preserves_explicit_tilde_and_range() {
    for raw in ["flow:wal@~0.1.0", "flow:wal@>=0.1, <0.3"] {
        let cli = PackageRef::parse(raw).unwrap();
        let out = finalize_pkgref_for_manifest(&cli, &vsemver("0.1.5"), false);
        assert_eq!(out, cli, "explicit constraint `{raw}` must be preserved");
    }
}

#[test]
fn finalize_exact_overrides_cli_form_to_eq_resolved() {
    // `--exact` is always-pin: even `@^0.1` becomes `=0.1.5`.
    let cli = PackageRef::parse("flow:wal@^0.1").unwrap();
    let out = finalize_pkgref_for_manifest(&cli, &vsemver("0.1.5"), true);
    assert_eq!(out.to_string(), "flow:wal@=0.1.5");
}

#[test]
fn finalize_exact_with_no_cli_version() {
    let cli = PackageRef::parse("flow:wal").unwrap();
    let out = finalize_pkgref_for_manifest(&cli, &vsemver("0.1.5"), true);
    assert_eq!(out.to_string(), "flow:wal@=0.1.5");
}

#[test]
fn merge_manifest_requires_keeps_unrelated_entries() {
    let mut m = empty_manifest();
    let other = PackageRef::parse("stack:rust-cli").unwrap();
    m.requires.packages.push(other.clone());
    let r = PackageRef::parse("flow:wal").unwrap();
    merge_manifest_requires(&mut m, std::slice::from_ref(&r));
    assert_eq!(m.requires.packages.len(), 2);
    // Unrelated entry survives.
    assert!(m.requires.packages.contains(&other));
    assert!(m.requires.packages.contains(&r));
}
