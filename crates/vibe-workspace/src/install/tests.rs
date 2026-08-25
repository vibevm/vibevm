//! Install orchestration unit tests.

use super::test_helpers::*;
use super::*;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn apply_resolution_materialises_and_regenerates_a_standalone_project() {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/wal\" = \"^0.3\"\n",
    );
    write(ws_dir.path(), boot_rel("00-core.md"), "# core");

    let (dep, _pkg) = dep_with_boot(
        "wal",
        "0.3.0",
        "[boot_snippet]\nsource = \"boot/10-flow-wal.md\"\ncategory = \"flow\"\n",
        "boot/10-flow-wal.md",
        "# wal boot",
    );

    let ws = Workspace::load(ws_dir.path()).unwrap();
    let mut outcome = apply_resolution(
        &ws,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();

    assert_eq!(outcome.materialised, vec![deps_rel("org.vibevm.wal/0.3.0")]);
    assert_eq!(outcome.nodes_regenerated, vec!["."]);
    // The package tree is materialised verbatim into its slot.
    assert!(
        ws_dir
            .path()
            .join(deps_rel("org.vibevm.wal/0.3.0/boot/10-flow-wal.md"))
            .is_file()
    );
    assert!(
        ws_dir
            .path()
            .join(deps_rel("org.vibevm.wal/0.3.0/vibe.toml"))
            .is_file()
    );
    // INDEX.md names the node's own foundation boot and the dependency.
    let index = fs::read_to_string(ws_dir.path().join(boot_rel("INDEX.md"))).unwrap();
    assert!(index.contains(&boot_rel("00-core.md")), "{index}");
    assert!(
        index.contains(&deps_rel("org.vibevm.wal/0.3.0/boot/10-flow-wal.md")),
        "{index}"
    );
    // The redirect lands at the node root.
    assert!(ws_dir.path().join("CLAUDE.md").is_file());
    assert!(outcome.take_post_install_plan().is_some());
    assert!(outcome.take_post_install_plan().is_none());
}

#[test]
fn apply_resolution_with_no_dependencies_still_writes_index() {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"solo\"\nversion = \"0.1.0\"\n",
    );
    write(ws_dir.path(), boot_rel("00-core.md"), "# core");
    let ws = Workspace::load(ws_dir.path()).unwrap();
    let mut outcome = apply_resolution(&ws, &[], SlotIntegrity::TrustPresence, None).unwrap();
    assert!(outcome.materialised.is_empty());
    assert_eq!(outcome.nodes_regenerated, vec!["."]);
    assert!(outcome.take_post_install_plan().is_none());
    assert!(ws_dir.path().join(boot_rel("INDEX.md")).is_file());
}

#[test]
fn apply_resolution_inline_dependency_produces_inline_md() {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/crit\" = { version = \"^1.0\", link = \"static\" }\n",
    );
    write(ws_dir.path(), boot_rel("00-core.md"), "# core");

    let (dep, _pkg) = dep_with_boot(
        "crit",
        "1.0.0",
        "[boot_snippet]\nsource = \"boot/crit.md\"\n",
        "boot/crit.md",
        "# critical discipline",
    );

    let ws = Workspace::load(ws_dir.path()).unwrap();
    apply_resolution(
        &ws,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();

    // The consumer's static link concatenates the dependency boot.
    let inline = fs::read_to_string(ws_dir.path().join(boot_rel("STATIC.md"))).unwrap();
    assert!(inline.contains("# critical discipline"), "{inline}");
}

#[test]
fn apply_resolution_renders_when_from_a_boot_snippet() {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/win\" = \"^1.0\"\n",
    );
    write(ws_dir.path(), boot_rel("00-core.md"), "# core");

    let (dep, _pkg) = dep_with_boot(
        "win",
        "1.0.0",
        "[boot_snippet]\nsource = \"boot/win.md\"\nwhen = \"os:windows\"\n",
        "boot/win.md",
        "# windows-only guidance",
    );

    let ws = Workspace::load(ws_dir.path()).unwrap();
    apply_resolution(
        &ws,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();

    // A condition forces the dynamic INCLUDE form without an explicit link.
    let index = fs::read_to_string(ws_dir.path().join(boot_rel("INDEX.md"))).unwrap();
    assert!(
        index.contains(&deps_rel("org.vibevm.win/1.0.0/boot/win.md")),
        "{index}"
    );
    assert!(index.contains("kind = \"dynamic\""), "{index}");
    assert!(index.contains("when = \"os:windows\""), "{index}");
}

#[test]
fn apply_resolution_skips_a_dependency_outside_the_node_requires() {
    // The resolution carries `flow:extra`, but the project does not
    // require it — it is materialised, but contributes no boot entry.
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/wal\" = \"^0.3\"\n",
    );
    write(ws_dir.path(), boot_rel("00-core.md"), "# core");

    let (wal, _w) = dep_with_boot(
        "wal",
        "0.3.0",
        "[boot_snippet]\nsource = \"boot/wal.md\"\n",
        "boot/wal.md",
        "# wal",
    );
    let (extra, _e) = dep_with_boot(
        "extra",
        "0.1.0",
        "[boot_snippet]\nsource = \"boot/extra.md\"\n",
        "boot/extra.md",
        "# extra",
    );

    let ws = Workspace::load(ws_dir.path()).unwrap();
    apply_resolution(&ws, &[wal, extra], SlotIntegrity::TrustPresence, None).unwrap();

    let index = fs::read_to_string(ws_dir.path().join(boot_rel("INDEX.md"))).unwrap();
    assert!(
        index.contains(&deps_rel("org.vibevm.wal/0.3.0/boot/wal.md")),
        "{index}"
    );
    // `flow:extra` is materialised but not in the boot index.
    assert!(
        ws_dir
            .path()
            .join(deps_rel("org.vibevm.extra/0.1.0/boot/extra.md"))
            .is_file()
    );
    assert!(!index.contains("flow-extra"), "{index}");
}

#[test]
fn apply_resolution_prunes_a_stale_slot_on_version_bump() {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/wal\" = \"^0\"\n",
    );
    write(ws_dir.path(), boot_rel("00-core.md"), "# core");
    let ws = Workspace::load(ws_dir.path()).unwrap();

    let (wal_v1, _v1) = dep_with_boot(
        "wal",
        "0.1.0",
        "[boot_snippet]\nsource = \"boot/wal.md\"\n",
        "boot/wal.md",
        "# v1",
    );
    apply_resolution(
        &ws,
        std::slice::from_ref(&wal_v1),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();
    assert!(
        ws_dir
            .path()
            .join(deps_rel("org.vibevm.wal/0.1.0"))
            .is_dir()
    );

    // Re-apply with wal bumped to 0.2.0 — the 0.1.0 slot is now stale.
    let (wal_v2, _v2) = dep_with_boot(
        "wal",
        "0.2.0",
        "[boot_snippet]\nsource = \"boot/wal.md\"\n",
        "boot/wal.md",
        "# v2",
    );
    let outcome = apply_resolution(
        &ws,
        std::slice::from_ref(&wal_v2),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();
    assert!(
        ws_dir
            .path()
            .join(deps_rel("org.vibevm.wal/0.2.0"))
            .is_dir()
    );
    assert!(
        !ws_dir
            .path()
            .join(deps_rel("org.vibevm.wal/0.1.0"))
            .exists(),
        "the stale 0.1.0 slot must be pruned"
    );
    assert_eq!(outcome.pruned, vec![deps_rel("org.vibevm.wal/0.1.0")]);
}

// --- PROP-011 §2.3 — materialise only the diff -----------------------

#[test]
fn apply_resolution_skips_a_present_slot_under_trust_presence() {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/wal\" = \"^0.3\"\n",
    );
    write(ws_dir.path(), boot_rel("00-core.md"), "# core");
    let (dep, _pkg) = dep_with_boot(
        "wal",
        "0.3.0",
        "[boot_snippet]\nsource = \"boot/wal.md\"\n",
        "boot/wal.md",
        "# wal",
    );
    let ws = Workspace::load(ws_dir.path()).unwrap();

    // First apply — the slot is absent, so it is materialised.
    let first = apply_resolution(
        &ws,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();
    assert_eq!(first.materialised, vec![deps_rel("org.vibevm.wal/0.3.0")]);
    assert!(first.skipped.is_empty());

    // A sentinel inside the slot — a file the source never had. It remains
    // outside materialiser ownership; outcome accounting proves this path
    // skipped without entering the diff materialiser.
    let sentinel = ws_dir
        .path()
        .join(deps_rel("org.vibevm.wal/0.3.0/SENTINEL"));
    fs::write(&sentinel, "untouched").unwrap();

    let second = apply_resolution(
        &ws,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();
    assert!(
        second.materialised.is_empty(),
        "a present slot must not be rematerialised"
    );
    assert_eq!(second.skipped, vec![deps_rel("org.vibevm.wal/0.3.0")]);
    assert!(
        sentinel.is_file(),
        "TrustPresence must leave the slot untouched"
    );
}

#[test]
fn apply_resolution_rematerialises_a_present_slot_under_verify() {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/wal\" = \"^0.3\"\n",
    );
    write(ws_dir.path(), boot_rel("00-core.md"), "# core");
    let (dep, _pkg) = dep_with_boot(
        "wal",
        "0.3.0",
        "[boot_snippet]\nsource = \"boot/wal.md\"\n",
        "boot/wal.md",
        "# wal",
    );
    let ws = Workspace::load(ws_dir.path()).unwrap();

    apply_resolution(&ws, std::slice::from_ref(&dep), SlotIntegrity::Verify, None).unwrap();
    let sentinel = ws_dir
        .path()
        .join(deps_rel("org.vibevm.wal/0.3.0/SENTINEL"));
    fs::write(&sentinel, "unrecorded").unwrap();

    // Second apply under Verify reports a materialisation pass, but the
    // unrecorded sentinel remains outside materialiser ownership.
    let second =
        apply_resolution(&ws, std::slice::from_ref(&dep), SlotIntegrity::Verify, None).unwrap();
    assert_eq!(second.materialised, vec![deps_rel("org.vibevm.wal/0.3.0")]);
    assert!(second.skipped.is_empty());
    assert!(sentinel.is_file());
}

// --- PROP-011 §5.2 — the `verify` slot spot-check ----------------------

/// A [`SlotVerifier`] stub returning a fixed verdict — the unit-level
/// stand-in for vibe-install's registry-hash implementation, so the
/// trust branch is testable without the hash crates.
#[cfg(test)]
struct StubVerifier(SlotCheck);

#[cfg(test)]
impl SlotVerifier for StubVerifier {
    fn verify_slot(&self, _dep: &ResolvedDep, _slot_abs: &Path) -> SlotCheck {
        self.0.clone()
    }
}

/// A [`SlotVerifier`] whose consultation is itself the failure — proves
/// the `trust-presence` path never reaches the check.
#[cfg(test)]
struct UntouchedVerifier;

#[cfg(test)]
impl SlotVerifier for UntouchedVerifier {
    fn verify_slot(&self, _dep: &ResolvedDep, _slot_abs: &Path) -> SlotCheck {
        panic!("the slot verifier must not be consulted on this path");
    }
}

/// The standard §5.2 fixture: a one-node workspace plus a resolved `wal`
/// dep. First materialises the slot (under `trust-presence`), then drops
/// an unrecorded sentinel inside the slot. Both a skip and a diff
/// materialisation must keep it; outcome accounting distinguishes the paths.
/// The dep's content-tree `TempDir` rides along because diverged/untrusted
/// paths still prepare and reconcile its incoming footprint.
#[cfg(test)]
fn verified_slot_fixture() -> (Workspace, ResolvedDep, TempDir, TempDir, PathBuf) {
    let ws_dir = TempDir::new().unwrap();
    write(
        ws_dir.path(),
        "vibe.toml",
        "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n\n\
         [requires.packages]\n\"org.vibevm/wal\" = \"^0.3\"\n",
    );
    write(ws_dir.path(), boot_rel("00-core.md"), "# core");
    let (dep, pkg) = dep_with_boot(
        "wal",
        "0.3.0",
        "[boot_snippet]\nsource = \"boot/wal.md\"\n",
        "boot/wal.md",
        "# wal",
    );
    let ws = Workspace::load(ws_dir.path()).unwrap();
    apply_resolution(
        &ws,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        None,
    )
    .unwrap();
    let sentinel = ws_dir
        .path()
        .join(deps_rel("org.vibevm.wal/0.3.0/SENTINEL"));
    fs::write(&sentinel, "probe").unwrap();
    (ws, dep, ws_dir, pkg, sentinel)
}

#[test]
fn verify_accepts_a_hash_matching_slot_without_copying() {
    let (ws, dep, _ws_dir, _pkg, sentinel) = verified_slot_fixture();
    let verifier = StubVerifier(SlotCheck::Verified);

    let outcome = apply_resolution_with(
        &ws,
        std::slice::from_ref(&dep),
        SlotIntegrity::Verify,
        Some(&verifier),
        None,
    )
    .unwrap();
    assert!(
        outcome.materialised.is_empty(),
        "a hash-matching slot must NOT be rematerialised under verify"
    );
    assert_eq!(outcome.skipped, vec![deps_rel("org.vibevm.wal/0.3.0")]);
    assert!(outcome.integrity_warnings.is_empty());
    assert!(
        sentinel.is_file(),
        "the spot-check accepting the slot must leave it untouched"
    );
}

#[test]
fn verify_rematerialises_a_diverged_slot_and_warns_with_both_hashes() {
    let (ws, dep, _ws_dir, _pkg, sentinel) = verified_slot_fixture();
    let verifier = StubVerifier(SlotCheck::Diverged {
        expected: "sha256:lockedhash".to_string(),
        actual: "sha256:slotthash".to_string(),
    });

    let outcome = apply_resolution_with(
        &ws,
        std::slice::from_ref(&dep),
        SlotIntegrity::Verify,
        Some(&verifier),
        None,
    )
    .unwrap();
    assert_eq!(outcome.materialised, vec![deps_rel("org.vibevm.wal/0.3.0")]);
    assert!(outcome.skipped.is_empty(), "a diverged slot is not trusted");
    assert!(
        sentinel.is_file(),
        "a diverged slot refresh must preserve unrecorded content"
    );
    // The warn line names the package and BOTH hashes.
    assert_eq!(outcome.integrity_warnings.len(), 1);
    let warn = &outcome.integrity_warnings[0];
    assert!(
        warn.contains("org.vibevm/wal@0.3.0"),
        "warn names the package: {warn}"
    );
    assert!(
        warn.contains("sha256:lockedhash"),
        "warn carries the locked hash: {warn}"
    );
    assert!(
        warn.contains("sha256:slotthash"),
        "warn carries the slot hash: {warn}"
    );
}

#[test]
fn verify_falls_back_to_rematerialising_an_unverifiable_slot_silently() {
    let (ws, dep, _ws_dir, _pkg, sentinel) = verified_slot_fixture();
    let verifier = StubVerifier(SlotCheck::Unverifiable);

    let outcome = apply_resolution_with(
        &ws,
        std::slice::from_ref(&dep),
        SlotIntegrity::Verify,
        Some(&verifier),
        None,
    )
    .unwrap();
    assert_eq!(outcome.materialised, vec![deps_rel("org.vibevm.wal/0.3.0")]);
    assert!(outcome.skipped.is_empty());
    assert!(
        outcome.integrity_warnings.is_empty(),
        "an unverifiable slot rematerialises without claiming divergence"
    );
    assert!(sentinel.is_file());
}

#[test]
fn trust_presence_never_consults_the_slot_verifier() {
    let (ws, dep, _ws_dir, _pkg, sentinel) = verified_slot_fixture();

    // The panicking verifier proves the fast path accepts by presence
    // alone and never hashes under `trust-presence`.
    let outcome = apply_resolution_with(
        &ws,
        std::slice::from_ref(&dep),
        SlotIntegrity::TrustPresence,
        Some(&UntouchedVerifier),
        None,
    )
    .unwrap();
    assert_eq!(outcome.skipped, vec![deps_rel("org.vibevm.wal/0.3.0")]);
    assert!(outcome.materialised.is_empty());
    assert!(sentinel.is_file());
}

// --- PROP-020 2.1 — pre-install hooks ride the materialise pass ---------
