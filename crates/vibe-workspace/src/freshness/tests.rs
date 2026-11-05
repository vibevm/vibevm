//! Existing freshness behavior tests, separated from production admission.
use super::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;
use vibe_core::manifest::CURRENT_SCHEMA_VERSION;

fn write(dir: &Path, rel: &str, body: &str) {
    let p = dir.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, body).unwrap();
}

fn ver(s: &str) -> semver::Version {
    semver::Version::parse(s).unwrap()
}

/// A standalone workspace whose `vibe.toml` carries `body` as the
/// `[requires]` section (already including the `[requires...]` header).
fn workspace_with_requires(requires_toml: &str) -> (TempDir, Workspace) {
    let tmp = TempDir::new().unwrap();
    write(
        tmp.path(),
        "vibe.toml",
        &format!("[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n{requires_toml}"),
    );
    let ws = Workspace::load(tmp.path()).unwrap();
    (tmp, ws)
}

/// Parse a `Lockfile` from a `[[package]]` body, prepending the meta
/// block. `roots` becomes `meta.root_dependencies`. The schema number
/// comes from the constant the reader checks, so a bump cannot rot it.
fn lockfile(roots: &[&str], packages_toml: &str) -> Lockfile {
    let roots_list = roots
        .iter()
        .map(|r| format!("\"{r}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let text = format!(
        "[meta]\ngenerated_by = \"test\"\ngenerated_at = \"x\"\nschema_version = {CURRENT_SCHEMA_VERSION}\n\
         root_dependencies = [{roots_list}]\n\n{packages_toml}"
    );
    toml::from_str(&text).unwrap()
}

/// Create an empty dependency-slot directory so `is_materialised`
/// reports the package as present.
fn materialise_slot(ws: &Workspace, name: &str, version: &str) {
    let group = vibe_core::Group::parse("org.vibevm").unwrap();
    fs::create_dir_all(ws.vibedeps_slot(&group, name, &ver(version))).unwrap();
}

/// A `[[package]]` table for a registry-resolved dependency. Carries a
/// `group` field — the PROP-008 qualified-naming lockfile.
fn registry_pkg(kind: &str, name: &str, version: &str) -> String {
    format!(
        "[[package]]\nkind = \"{kind}\"\ngroup = \"org.vibevm\"\nname = \"{name}\"\n\
         version = \"{version}\"\nsource_url = \"https://example/{name}\"\n\
         content_hash = \"sha256:abc\"\nsource_kind = \"registry\"\n"
    )
}

/// Build a `file://` URL for `root`/`sub` — the shape a local-directory
/// registry records. Windows `C:\…` → `file:///C:/…`; Unix `/…` →
/// `file:///…`.
fn file_url_under(root: &Path, sub: &str) -> String {
    let s = root.join(sub).to_string_lossy().replace('\\', "/");
    if s.starts_with('/') {
        format!("file://{s}")
    } else {
        format!("file:///{s}")
    }
}

/// A `[[package]]` for a dependency resolved from a local `file://`
/// directory registry (`--registry <path>`) at `source_url` (§2.6).
fn local_pkg(kind: &str, name: &str, version: &str, source_url: &str) -> String {
    format!(
        "[[package]]\nkind = \"{kind}\"\ngroup = \"org.vibevm\"\nname = \"{name}\"\n\
         version = \"{version}\"\nsource_url = \"{source_url}\"\n\
         content_hash = \"sha256:abc\"\nsource_kind = \"registry\"\n"
    )
}

/// As [`local_pkg`] but `in-place` materialised — the §2.6 mutable rule
/// excludes it, so a PROP-022 giant keeps the §2.2 fast path.
fn local_in_place_pkg(kind: &str, name: &str, version: &str, source_url: &str) -> String {
    format!(
        "[[package]]\nkind = \"{kind}\"\ngroup = \"org.vibevm\"\nname = \"{name}\"\n\
         version = \"{version}\"\nsource_url = \"{source_url}\"\n\
         content_hash = \"sha256:abc\"\nsource_kind = \"registry\"\n\
         materialization = \"in-place\"\n"
    )
}

#[test]
fn fresh_when_lock_satisfies_and_slots_present() {
    let (_t, ws) = workspace_with_requires("[requires.packages]\n\"org.vibevm/wal\" = \"^0.3\"\n");
    let lf = lockfile(&["org.vibevm/wal"], &registry_pkg("flow", "wal", "0.3.2"));
    materialise_slot(&ws, "wal", "0.3.2");
    assert_eq!(check(&ws, &lf), Freshness::Fresh);
}

#[test]
fn stale_when_a_declared_dep_is_absent_from_the_lock() {
    let (_t, ws) = workspace_with_requires("[requires.packages]\n\"org.vibevm/wal\" = \"^0.3\"\n");
    let lf = lockfile(&[], "");
    match check(&ws, &lf) {
        Freshness::Stale(r) => assert!(r.contains("absent from vibe.lock"), "{r}"),
        other => panic!("expected Stale, got {other:?}"),
    }
}

#[test]
fn stale_when_the_locked_version_is_outside_the_constraint() {
    // The constraint was tightened to `^0.4`; the lock still pins 0.3.2.
    let (_t, ws) = workspace_with_requires("[requires.packages]\n\"org.vibevm/wal\" = \"^0.4\"\n");
    let lf = lockfile(&["org.vibevm/wal"], &registry_pkg("flow", "wal", "0.3.2"));
    materialise_slot(&ws, "wal", "0.3.2");
    match check(&ws, &lf) {
        Freshness::Stale(r) => assert!(r.contains("outside the constraint"), "{r}"),
        other => panic!("expected Stale, got {other:?}"),
    }
}

#[test]
fn fresh_when_locked_version_still_satisfies_a_loosened_constraint() {
    // `^0.3` and the lock at 0.3.2 — no drift to a newer 0.3.x; the
    // locked version is honoured verbatim (the lockfile-respecting win).
    let (_t, ws) = workspace_with_requires("[requires.packages]\n\"org.vibevm/wal\" = \"^0.3\"\n");
    let lf = lockfile(&["org.vibevm/wal"], &registry_pkg("flow", "wal", "0.3.2"));
    materialise_slot(&ws, "wal", "0.3.2");
    assert!(check(&ws, &lf).is_fresh());
}

#[test]
fn stale_when_a_root_was_removed() {
    // `[requires]` declares only `flow:wal`, but the lock still records
    // `feat:auth` as a root — a dependency was dropped.
    let (_t, ws) = workspace_with_requires("[requires.packages]\n\"org.vibevm/wal\" = \"^0.3\"\n");
    let lf = lockfile(
        &["org.vibevm/wal", "org.vibevm/auth"],
        &format!(
            "{}\n{}",
            registry_pkg("flow", "wal", "0.3.2"),
            registry_pkg("feat", "auth", "1.0.0"),
        ),
    );
    materialise_slot(&ws, "wal", "0.3.2");
    materialise_slot(&ws, "auth", "1.0.0");
    match check(&ws, &lf) {
        Freshness::Stale(r) => assert!(r.contains("root set"), "{r}"),
        other => panic!("expected Stale, got {other:?}"),
    }
}

#[test]
fn stale_when_a_locked_slot_is_not_materialised() {
    let (_t, ws) = workspace_with_requires("[requires.packages]\n\"org.vibevm/wal\" = \"^0.3\"\n");
    let lf = lockfile(&["org.vibevm/wal"], &registry_pkg("flow", "wal", "0.3.2"));
    // No materialise_slot call — the slot is absent.
    match check(&ws, &lf) {
        Freshness::Stale(r) => assert!(r.contains("no materialised"), "{r}"),
        other => panic!("expected Stale, got {other:?}"),
    }
}

#[test]
fn stale_when_a_git_source_dependency_is_present() {
    let (_t, ws) = workspace_with_requires(
        "[requires.packages]\n\
         \"org.vibevm/internal\" = { git = \"https://example/i\", tag = \"v1.0.0\" }\n",
    );
    let lf = lockfile(&[], "");
    match check(&ws, &lf) {
        Freshness::Stale(r) => assert!(r.contains("git-source"), "{r}"),
        other => panic!("expected Stale, got {other:?}"),
    }
}

#[test]
fn stale_when_a_dep_resolves_from_an_in_workspace_file_source() {
    // The in-repo self-hosting registry (`--registry packages`) is a
    // mutable working tree UNDER the workspace root — never version-
    // immutable — so the lock cannot be proven fresh; it re-resolves to
    // pick up any source edit (§2.6).
    let (_t, ws) = workspace_with_requires("[requires.packages]\n\"org.vibevm/wal\" = \"^0.3\"\n");
    let src = file_url_under(&ws.root, &crate::layout_paths::packages("wal"));
    let lf = lockfile(
        &["org.vibevm/wal"],
        &local_pkg("flow", "wal", "0.3.2", &src),
    );
    materialise_slot(&ws, "wal", "0.3.2");
    match check(&ws, &lf) {
        Freshness::Stale(r) => assert!(
            r.contains("in-workspace file://") && r.contains("mutable working tree"),
            "{r}"
        ),
        other => panic!("expected Stale, got {other:?}"),
    }
}

#[test]
fn fresh_for_an_external_local_file_source_kept_immutable() {
    // A `file://` source OUTSIDE the workspace (an external local registry
    // or mirror, a test fixture) is left immutable and keeps the §2.2 fast
    // path — only the in-repo self-hosting registry is mutable (§2.6).
    let (_t, ws) = workspace_with_requires("[requires.packages]\n\"org.vibevm/wal\" = \"^0.3\"\n");
    let lf = lockfile(
        &["org.vibevm/wal"],
        &local_pkg("flow", "wal", "0.3.2", "file:///external/registry/wal"),
    );
    materialise_slot(&ws, "wal", "0.3.2");
    assert!(check(&ws, &lf).is_fresh());
}

#[test]
fn fresh_for_an_in_place_file_source_the_mutable_rule_excludes_giants() {
    // A `file://` source UNDER the workspace that is `in-place` (a PROP-022
    // giant) keeps the fast path: §2.6 excludes it from the mutable
    // treatment, so with the lock satisfying and the slot present it is
    // Fresh. The source is in-workspace, so `in-place` is the *only* reason
    // it is not mutable — isolating the §2.6 exclusion.
    let (_t, ws) =
        workspace_with_requires("[requires.packages]\n\"org.vibevm/giant\" = \"^1.0\"\n");
    let src = file_url_under(&ws.root, &crate::layout_paths::packages("giant"));
    let lf = lockfile(
        &["org.vibevm/giant"],
        &local_in_place_pkg("feat", "giant", "1.0.0", &src),
    );
    materialise_slot(&ws, "giant", "1.0.0");
    assert!(check(&ws, &lf).is_fresh());
}

#[test]
fn stale_when_a_capability_requirement_is_present() {
    let (_t, ws) =
        workspace_with_requires("[requires]\ncapabilities = [\"capability:wal-protocol\"]\n");
    let lf = lockfile(&[], "");
    match check(&ws, &lf) {
        Freshness::Stale(r) => assert!(r.contains("capability"), "{r}"),
        other => panic!("expected Stale, got {other:?}"),
    }
}

#[test]
fn fresh_for_an_empty_requires_and_empty_lock() {
    // A degenerate project with nothing declared and nothing locked is
    // trivially fresh — though `vibe install` bails earlier on "nothing
    // to install" before the check is ever reached.
    let (_t, ws) = workspace_with_requires("");
    let lf = lockfile(&[], "");
    assert!(check(&ws, &lf).is_fresh());
}

// --- PROP-011 §5.3 — minimum-churn re-resolution (`hold_pins`) --------

#[test]
fn hold_pins_pins_a_satisfied_registry_root() {
    let lf = lockfile(&["org.vibevm/wal"], &registry_pkg("flow", "wal", "0.3.2"));
    let declared = vec![PackageRef::parse("org.vibevm/wal@^0.3").unwrap()];
    let pinned = hold_pins(&declared, &lf);
    // `^0.3` becomes `=0.3.2` — the locked version is held.
    assert_eq!(pinned[0].to_string(), "org.vibevm/wal@=0.3.2");
}

#[test]
fn hold_pins_leaves_a_changed_root_free() {
    // The constraint moved to `^0.4`; the lock at 0.3.2 no longer
    // satisfies it — this is the change, it must resolve freely.
    let lf = lockfile(&["org.vibevm/wal"], &registry_pkg("flow", "wal", "0.3.2"));
    let declared = vec![PackageRef::parse("org.vibevm/wal@^0.4").unwrap()];
    let pinned = hold_pins(&declared, &lf);
    assert_eq!(pinned[0], declared[0]);
}

#[test]
fn hold_pins_leaves_an_unlocked_root_free() {
    let lf = lockfile(&[], "");
    let declared = vec![PackageRef::parse("org.vibevm/new@^1.0").unwrap()];
    let pinned = hold_pins(&declared, &lf);
    assert_eq!(pinned[0], declared[0]);
}

#[test]
fn hold_pins_does_not_pin_a_git_sourced_root() {
    // A git-source root resolves by ref, not version — pinning it to
    // `=<version>` would be meaningless, so it is left at its declared
    // form and resolves freely.
    let lf = lockfile(
        &["org.vibevm/internal"],
        "[[package]]\nkind = \"flow\"\ngroup = \"org.vibevm\"\nname = \"internal\"\n\
         version = \"0.1.0\"\nsource_url = \"https://example/i\"\n\
         content_hash = \"sha256:abc\"\nsource_kind = \"git\"\n",
    );
    let declared = vec![PackageRef::parse("org.vibevm/internal").unwrap()];
    let pinned = hold_pins(&declared, &lf);
    assert_eq!(pinned[0], declared[0]);
}

#[test]
fn hold_pins_mixes_held_and_free_roots() {
    // wal is satisfied (held); auth's constraint moved (free).
    let lf = lockfile(
        &["org.vibevm/wal", "org.vibevm/auth"],
        &format!(
            "{}\n{}",
            registry_pkg("flow", "wal", "0.3.2"),
            registry_pkg("feat", "auth", "1.0.0"),
        ),
    );
    let declared = vec![
        PackageRef::parse("org.vibevm/wal@^0.3").unwrap(),
        PackageRef::parse("org.vibevm/auth@^2.0").unwrap(),
    ];
    let pinned = hold_pins(&declared, &lf);
    assert_eq!(pinned[0].to_string(), "org.vibevm/wal@=0.3.2");
    assert_eq!(pinned[1], declared[1]);
}
