//! The lockfile's contribution to a phase fingerprint.
//!
//! `[meta].generated_at` is a fresh stamp on every lock write, so hashing the
//! raw bytes made a row's fingerprint depend on WHEN its lock was last
//! rewritten. That is not a hypothetical: an install writes the lock and then
//! parks a `slot:post-install` row, so the resume re-read a lock whose only
//! difference was the stamp, computed a different fingerprint, and reparked —
//! forever. These cases pin the canonicalisation and its exact boundary:
//! provenance TIME is excluded, and everything a resolution actually depends
//! on — including `generated_by` — still contributes.

use std::fs;

use vibe_core::manifest::Lockfile;

use super::support::{context, row};
use crate::fingerprint_execution;

/// Write `lockfile` where the fixture context expects it, and return the
/// fingerprint of the same unchanged row against it.
fn fingerprint_with(dir: &std::path::Path, lockfile: &Lockfile) -> String {
    fs::write(
        dir.join("vibe.lock"),
        toml::to_string_pretty(lockfile).unwrap(),
    )
    .unwrap();
    let one = row(dir, "one", "0.1.0", None);
    let ctx = context(dir, one.effective_config().unwrap());
    fingerprint_execution(&one, &ctx).unwrap()
}

fn scratch() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname='demo'\nversion='0.1.0'\n",
    )
    .unwrap();
    dir
}

/// The bug this exists to keep fixed: rewriting the lock with nothing changed
/// but the provenance stamp leaves every fingerprint identical, so a park
/// taken after the lock barrier can actually be satisfied on the next run.
#[test]
fn a_new_generated_at_stamp_leaves_the_fingerprint_unchanged() {
    let dir = scratch();
    let mut lockfile = Lockfile::empty("vibe 1.0.0", "2026-08-26T00:00:00Z");
    let before = fingerprint_with(dir.path(), &lockfile);

    lockfile.meta.generated_at = "2026-08-27T11:22:33Z".into();
    assert_eq!(
        fingerprint_with(dir.path(), &lockfile),
        before,
        "only the provenance stamp moved: the resolution is byte-identical",
    );

    // And once more, to a third stamp — the fingerprint is stamp-INDEPENDENT,
    // not merely stable across one particular pair of values.
    lockfile.meta.generated_at = "2027-01-01T00:00:00Z".into();
    assert_eq!(fingerprint_with(dir.path(), &lockfile), before);
}

/// The other half of the boundary. `generated_at` is the ONLY field dropped:
/// the producing toolchain and the resolved world both still move the
/// fingerprint, so canonicalising did not quietly stop fingerprinting the
/// lock.
#[test]
fn generated_by_and_the_resolved_world_still_move_the_fingerprint() {
    let dir = scratch();
    let base_lock = Lockfile::empty("vibe 1.0.0", "2026-08-26T00:00:00Z");
    let base = fingerprint_with(dir.path(), &base_lock);

    let mut other_toolchain = base_lock.clone();
    other_toolchain.meta.generated_by = "vibe 1.1.0".into();
    assert_ne!(
        fingerprint_with(dir.path(), &other_toolchain),
        base,
        "a different vibe can resolve differently, so its identity is input",
    );

    let mut other_solver = base_lock.clone();
    other_solver.meta.solver = Some("resolvo-0.x".into());
    assert_ne!(
        fingerprint_with(dir.path(), &other_solver),
        base,
        "the depsolver identity is a resolution fact",
    );

    let mut other_roots = base_lock.clone();
    other_roots.meta.root_dependencies = vec!["tool:org.demo/thing@1.0.0".parse().unwrap()];
    assert_ne!(
        fingerprint_with(dir.path(), &other_roots),
        base,
        "what the user asked for is a resolution fact",
    );

    let mut other_features = base_lock.clone();
    other_features.meta.active_features = vec!["org.demo/thing/extra".into()];
    assert_ne!(
        fingerprint_with(dir.path(), &other_features),
        base,
        "the active feature set is a resolution fact",
    );
}

/// A lock that does not parse is NOT silently dropped from the fingerprint:
/// it falls back to its raw bytes, so a corrupt-lock difference is still a
/// difference. Presence itself is framed separately from content.
#[test]
fn an_unparseable_lock_still_contributes_its_raw_bytes() {
    let dir = scratch();
    let one = row(dir.path(), "one", "0.1.0", None);
    let ctx = context(dir.path(), one.effective_config().unwrap());

    fs::write(dir.path().join("vibe.lock"), "this is not toml {{{").unwrap();
    let garbage = fingerprint_execution(&one, &ctx).unwrap();
    fs::write(dir.path().join("vibe.lock"), "this is not toml }}}").unwrap();
    let other_garbage = fingerprint_execution(&one, &ctx).unwrap();
    assert_ne!(
        garbage, other_garbage,
        "the raw-byte fallback still fingerprints an unparseable lock",
    );

    fs::remove_file(dir.path().join("vibe.lock")).unwrap();
    let absent = fingerprint_execution(&one, &ctx).unwrap();
    assert_ne!(
        absent, garbage,
        "an absent lock is a different world from a corrupt one",
    );
}
