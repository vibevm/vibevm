//! End-to-end tests for `vibe cache check` / `--repair` (PROP-010
//! §2.8 CMD-CHECK / CMD-CHECK-REPAIR): the integrity sweep over the
//! machine store and the cheapest-first repair ladder. A separate
//! test binary from `cli_cache.rs` along the file-length budget seam.
//!
//! Isolation: `UserScratch` points `$VIBE_SETTINGS` (and with it the
//! store, `<settings>/cache`) at a temp home — the real `~/.vibe` is
//! never touched. The fixture is a `file://` directory registry (no
//! git): the local-directory backend serves `[[registry]] url =
//! "file:///…"` straight off the filesystem.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::UserScratch;

/// The machine store root for a scratch home (`<settings>/cache`).
fn store_dir(user: &UserScratch) -> PathBuf {
    user.settings.join("cache")
}

/// The `file:///…` url form the local-directory registry backend opens.
fn file_url(path: &Path) -> String {
    format!("file:///{}", path.to_string_lossy().replace('\\', "/"))
}

/// A hermetic `file://` directory registry: `org.example/parent` in
/// TWO versions (so a repair that wrongly advanced the version is
/// catchable — `REPAIR-DOES-NOT-PULL`), v0.1.0 `[requires]`-ing
/// `org.example/child`, plus `org.example/child@0.1.0`.
fn make_dir_registry(root: &Path) -> PathBuf {
    let registry = root.join("registry");
    let parent_v1 = registry.join("org.example").join("parent").join("v0.1.0");
    let parent_v2 = registry.join("org.example").join("parent").join("v0.2.0");
    let child = registry.join("org.example").join("child").join("v0.1.0");
    for dir in [&parent_v1, &parent_v2] {
        fs::create_dir_all(dir.join(common::spec_rel("flows/parent"))).unwrap();
    }
    fs::create_dir_all(child.join(common::spec_rel("flows/child"))).unwrap();

    fs::write(
        parent_v1.join("vibe.toml"),
        "[package]\n\
         group = \"org.example\"\n\
         name = \"parent\"\n\
         kind = \"flow\"\n\
         version = \"0.1.0\"\n\n\
         [requires.packages]\n\
         \"org.example/child\" = \"^0.1.0\"\n",
    )
    .unwrap();
    // Version-stamped payload: after a repair the entry must carry the
    // v0.1.0 marker again — NOT v0.2.0's.
    fs::write(
        parent_v1.join(common::spec_rel("flows/parent/P.md")),
        "parent v0.1.0\n",
    )
    .unwrap();

    fs::write(
        parent_v2.join("vibe.toml"),
        "[package]\n\
         group = \"org.example\"\n\
         name = \"parent\"\n\
         kind = \"flow\"\n\
         version = \"0.2.0\"\n",
    )
    .unwrap();
    fs::write(
        parent_v2.join(common::spec_rel("flows/parent/P.md")),
        "parent v0.2.0\n",
    )
    .unwrap();

    fs::write(
        child.join("vibe.toml"),
        "[package]\n\
         group = \"org.example\"\n\
         name = \"child\"\n\
         kind = \"flow\"\n\
         version = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(
        child.join(common::spec_rel("flows/child/C.md")),
        "# child\n",
    )
    .unwrap();
    registry
}

/// A scratch home wired to the fixture registry projectlessly, plus a
/// bare working directory. Returns `(outer, user, bare)` — `outer`
/// owns the temp tree the registry lives in, so it must outlive every
/// command this test runs.
fn wired_user() -> (tempfile::TempDir, UserScratch, tempfile::TempDir) {
    let outer = tempfile::tempdir().unwrap();
    let registry = make_dir_registry(outer.path());
    let user = UserScratch::new();
    fs::write(
        user.settings.join("registry.toml"),
        format!(
            "[[registry]]\nname = \"fixture\"\nurl = \"{}\"\n",
            file_url(&registry)
        ),
    )
    .unwrap();
    let bare = tempfile::tempdir().unwrap();
    (outer, user, bare)
}

/// `vibe cache add org.example/parent@=0.1.0` under `user`/`bare` —
/// lands parent@0.1.0 + child@0.1.0 in the store with their sidecars.
/// Pinned on purpose: the registry also carries v0.2.0, and these
/// tests assert the store holds v0.1.0 — the newer version never
/// enters the store at all, which is half of the REPAIR-DOES-NOT-PULL
/// proof.
fn prewarm(user: &UserScratch, bare: &Path) {
    let out = user
        .vibe()
        .current_dir(bare)
        .arg("cache")
        .arg("add")
        .arg("org.example/parent@=0.1.0")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "pre-warm failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A clean store sweeps to exit 0 with the all-ok count line.
#[test]
fn check_passes_on_a_clean_store() {
    let (_outer, user, bare) = wired_user();
    prewarm(&user, bare.path());

    let out = user
        .vibe()
        .current_dir(bare.path())
        .arg("cache")
        .arg("check")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "a clean store must pass; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("2 ok, 0 mismatched, 0 unrecorded"),
        "expected the all-ok summary; got:\n{stdout}"
    );
}

/// A tampered byte makes the sweep name the identity and BOTH hashes
/// (recorded vs computed) in the StoreEntryMismatch grammar, and exit
/// non-zero. This is the test the disarmed-recompute red proof drops.
#[test]
fn check_names_tampered_entry_and_exits_nonzero() {
    let (_outer, user, bare) = wired_user();
    prewarm(&user, bare.path());
    let entry = store_dir(&user).join("org.example/parent/v0.1.0");
    fs::write(
        entry.join(common::spec_rel("flows/parent/P.md")),
        "parent v0.1.0\nTAMPERED\n",
    )
    .unwrap();

    let out = user
        .vibe()
        .current_dir(bare.path())
        .arg("cache")
        .arg("check")
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "a tampered entry must fail the sweep"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("org.example/parent@0.1.0"),
        "the report must name the identity; got:\n{stdout}"
    );
    assert!(
        stdout.contains("recorded sidecar pins"),
        "the mismatch line must carry the StoreEntryMismatch grammar; got:\n{stdout}"
    );
    assert_eq!(
        stdout.matches("sha256:").count(),
        2,
        "the mismatch line must carry BOTH hashes (recorded + computed); got:\n{stdout}"
    );
    assert!(
        stdout.contains("1 mismatched"),
        "the summary must count the mismatch; got:\n{stdout}"
    );
}

/// `--repair` re-fetches a tampered EXTRACTED entry at the SAME exact
/// version — the registry also carries v0.2.0 and it must NOT be
/// pulled (`REPAIR-DOES-NOT-PULL`) — after which a plain check exits
/// 0 again.
#[test]
fn repair_refetches_tampered_entry_at_the_same_version() {
    let (_outer, user, bare) = wired_user();
    prewarm(&user, bare.path());
    let store = store_dir(&user);
    let entry = store.join("org.example/parent/v0.1.0");
    fs::write(
        entry.join(common::spec_rel("flows/parent/P.md")),
        "parent v0.1.0\nTAMPERED\n",
    )
    .unwrap();

    let out = user
        .vibe()
        .current_dir(bare.path())
        .arg("cache")
        .arg("check")
        .arg("--repair")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "repair must land the entry back on its feet; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("re-fetched"),
        "the repair report must name the re-fetch; got:\n{stdout}"
    );

    // The entry is the ORIGINAL v0.1.0 again — not v0.2.0's payload.
    assert_eq!(
        fs::read_to_string(entry.join(common::spec_rel("flows/parent/P.md"))).unwrap(),
        "parent v0.1.0\n",
        "repair restores the recorded version's bytes"
    );
    assert!(
        !store.join("org.example/parent/v0.2.0").exists(),
        "repair must not advance to a newer version (REPAIR-DOES-NOT-PULL)"
    );
    // The re-fetch recorded a fresh sidecar with the fresh insert.
    assert!(store.join("org.example/parent/v0.1.0.sha256").is_file());

    // And the plain sweep now passes.
    let out = user
        .vibe()
        .current_dir(bare.path())
        .arg("cache")
        .arg("check")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "post-repair check must pass; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// An erased sidecar is the `unrecorded` class (named, non-zero exit);
/// `--repair` records a sidecar from the entry's current bytes and a
/// plain check passes again.
#[test]
fn unrecorded_entry_is_named_and_repair_records_now() {
    let (_outer, user, bare) = wired_user();
    prewarm(&user, bare.path());
    let store = store_dir(&user);
    let sidecar = store.join("org.example/parent/v0.1.0.sha256");
    assert!(sidecar.is_file(), "the pre-warm recorded a sidecar");
    fs::remove_file(&sidecar).unwrap();

    let out = user
        .vibe()
        .current_dir(bare.path())
        .arg("cache")
        .arg("check")
        .output()
        .unwrap();
    assert!(!out.status.success(), "an unrecorded entry must not pass");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("unrecorded") && stdout.contains("org.example/parent@0.1.0"),
        "the report must name the unrecorded class and identity; got:\n{stdout}"
    );

    let out = user
        .vibe()
        .current_dir(bare.path())
        .arg("cache")
        .arg("check")
        .arg("--repair")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "recording the sidecar must repair it; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("recorded"),
        "the repair report must name the recorded-now class; got:\n{stdout}"
    );
    assert!(
        sidecar.is_file(),
        "the sidecar is back on disk after repair"
    );

    let out = user
        .vibe()
        .current_dir(bare.path())
        .arg("cache")
        .arg("check")
        .output()
        .unwrap();
    assert!(out.status.success(), "post-repair check must pass");
}
