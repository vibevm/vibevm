//! Integration oracles for the machine-global store's public,
//! root-less surface — `store_root` / `insert_from` / `lookup` /
//! `list_versions` / `list_all` (PROP-010 §2.7).
//!
//! The root-less functions resolve the store through
//! `$VIBE_SETTINGS` → `~/.vibe`, and this workspace is edition 2024
//! with `#![forbid(unsafe_code)]` in-crate: `std::env::set_var` is
//! unsafe there, and libtest runs bodies on many threads. So the
//! env-shaped scenarios use the crate tree's established idiom
//! (`vibe-core`'s `user_config/tests.rs`): re-execute this very test
//! binary as a child whose environment is set at spawn — the child
//! exercises the real root-less functions against a temp settings
//! home; the parent drives the child and asserts on both the exit
//! status and the store the child left on disk. No test in this file
//! ever touches the operator's real `~/.vibe`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use vibe_core::Group;
use vibe_core::settings::SETTINGS_DIR_ENV;
use vibe_registry::store::{self, InsertOutcome};

/// Set on the re-executed child: the marked test takes its child
/// branch instead of spawning another copy of itself.
const CHILD_MARKER: &str = "R1_STORE_CHILD";
/// Set (with [`CHILD_MARKER`]) on the red-proof child only: after the
/// green scenarios, the child disarms the write-once guard with a
/// direct filesystem overwrite and then runs the green test's sentinel
/// assertion — which must fail, proving the test detects a rewrite.
const RED_MARKER: &str = "R1_STORE_RED_PROOF";

fn group() -> Group {
    Group::parse("org.vibevm").unwrap()
}

fn version(v: &str) -> semver::Version {
    semver::Version::parse(v).unwrap()
}

/// A minimal package source tree with a sentinel file whose bytes and
/// mtime the write-once scenario watches.
fn make_src(root: &Path, name: &str, body: &str) -> PathBuf {
    let src = root.join(name);
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("vibe.toml"),
        "[package]\ngroup = \"org.vibevm\"\nname = \"wal\"\nkind = \"flow\"\nversion = \"0.2.0\"\n",
    )
    .unwrap();
    fs::write(src.join("SENTINEL.md"), body).unwrap();
    src
}

/// Re-execute this binary running only `test_name`, with `extra_env`
/// set on the child.
fn spawn_child(test_name: &str, extra_env: &[(&str, PathBuf)]) -> std::process::Output {
    let mut cmd = Command::new(std::env::current_exe().expect("test binary path"));
    cmd.arg(test_name)
        .arg("--exact")
        .arg("--test-threads=1")
        .arg("--nocapture")
        .env(CHILD_MARKER, "1");
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    cmd.output().expect("re-exec the test binary")
}

/// The root-less public API, end to end against a temp settings home:
/// insert → layout, lookup, the index views; write-once on the second
/// insert; accretion of a second version. The real functions run in a
/// child whose `$VIBE_SETTINGS` is a temp dir; the parent then reads
/// the store the child left behind.
#[test]
fn rootless_store_roundtrip_write_once_and_index_views() {
    let settings = tempfile::tempdir().unwrap();
    if std::env::var_os(CHILD_MARKER).is_some() {
        child_scenarios();
        println!("CHILD-OK");
        return;
    }

    let out = spawn_child(
        "rootless_store_roundtrip_write_once_and_index_views",
        &[(SETTINGS_DIR_ENV, settings.path().to_path_buf())],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "child failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("CHILD-OK"),
        "child did not report completion:\n{stdout}"
    );

    // Independent of the child's prints: the store the child left
    // under the temp settings home carries the documented layout
    // (`<settings>/cache/<group>/<name>/v<version>/` — the layout IS
    // the index) and both versions accreted.
    let entry_a = settings
        .path()
        .join("cache")
        .join("org.vibevm")
        .join("wal")
        .join("v0.2.0");
    let entry_b = settings
        .path()
        .join("cache")
        .join("org.vibevm")
        .join("wal")
        .join("v0.3.0");
    assert!(entry_a.join("vibe.toml").is_file(), "entry v0.2.0 missing");
    assert!(entry_a.join("SENTINEL.md").is_file());
    assert!(entry_b.join("vibe.toml").is_file(), "entry v0.3.0 missing");
}

/// The child half of [`rootless_store_roundtrip_write_once_and_index_views`]:
/// every assertion here runs against the REAL root-less API with
/// `$VIBE_SETTINGS` pointing at the temp home the parent chose.
fn child_scenarios() {
    let settings =
        PathBuf::from(std::env::var_os(SETTINGS_DIR_ENV).expect("child carries VIBE_SETTINGS"));
    let scratch = tempfile::tempdir().unwrap();
    let rootless_root = store::store_root().unwrap();
    assert_eq!(
        rootless_root,
        settings.join("cache"),
        "store_root must be <settings-home>/cache (THE-STORE-IS-DOT-VIBE-CACHE)"
    );

    // Roundtrip: insert → lookup → list_versions / list_all see it.
    let src_a = make_src(scratch.path(), "src-a", "first-class bytes\n");
    let outcome = store::insert_from(&src_a, &group(), "wal", &version("0.2.0")).unwrap();
    let entry_a = rootless_root.join("org.vibevm").join("wal").join("v0.2.0");
    assert_eq!(outcome, InsertOutcome::Inserted(entry_a.clone()));
    assert_eq!(
        store::lookup(&group(), "wal", &version("0.2.0")),
        Some(entry_a.clone())
    );
    assert_eq!(
        store::list_versions(&group(), "wal"),
        vec![version("0.2.0")]
    );
    assert_eq!(
        store::list_all(),
        vec![(group(), "wal".to_string(), version("0.2.0"))]
    );

    // Write-once: a second insert with DIFFERENT content is an
    // AlreadyPresent no-op — the sentinel's bytes and mtime are
    // untouched, the first fetch's bytes stay authoritative.
    let sentinel = entry_a.join("SENTINEL.md");
    let bytes_before = fs::read(&sentinel).unwrap();
    let mtime_before = fs::metadata(&sentinel).unwrap().modified().unwrap();
    let src_b = make_src(scratch.path(), "src-b", "DIFFERENT rogue bytes\n");
    let again = store::insert_from(&src_b, &group(), "wal", &version("0.2.0")).unwrap();
    assert_eq!(again, InsertOutcome::AlreadyPresent(entry_a.clone()));
    assert_eq!(
        fs::read(&sentinel).unwrap(),
        bytes_before,
        "write-once violated: the entry's bytes were rewritten"
    );
    assert_eq!(
        fs::metadata(&sentinel).unwrap().modified().unwrap(),
        mtime_before,
        "write-once violated: the entry's sentinel mtime moved"
    );

    // Accretion: a second version lands beside the first and both are
    // listed (versions ascending, the offline-resolvable inventory).
    let src_c = make_src(scratch.path(), "src-c", "next version\n");
    store::insert_from(&src_c, &group(), "wal", &version("0.3.0")).unwrap();
    assert_eq!(
        store::list_versions(&group(), "wal"),
        vec![version("0.2.0"), version("0.3.0")]
    );
    assert_eq!(
        store::list_all(),
        vec![
            (group(), "wal".to_string(), version("0.2.0")),
            (group(), "wal".to_string(), version("0.3.0")),
        ]
    );

    // Absence is absence: an unknown identity has no entry and no
    // versions.
    assert_eq!(store::lookup(&group(), "nope", &version("0.2.0")), None);
    assert!(store::list_versions(&group(), "nope").is_empty());
}

/// Red proof of the write-once guard: the child runs the green
/// scenarios, then performs the rogue rewrite a broken (non-write-once)
/// insert would perform — a direct filesystem overwrite of the entry —
/// and then runs the SAME sentinel assertion the green scenario relies
/// on. It must fail, with exactly the write-once message; the parent
/// asserts the child failed that way. An ordinary suite run never sets
/// the markers, so this test stays green while proving the guard it
/// stands on; running the red branch by hand is
/// `R1_STORE_CHILD=1 R1_STORE_RED_PROOF=1 cargo test --test store`.
#[test]
fn write_once_guard_disarmed_fails_exactly_its_own_assertion() {
    if std::env::var_os(CHILD_MARKER).is_some() {
        child_scenarios(); // green insert first
        if std::env::var_os(RED_MARKER).is_none() {
            return;
        }
        let entry = store::store_root()
            .unwrap()
            .join("org.vibevm")
            .join("wal")
            .join("v0.2.0");
        let sentinel = entry.join("SENTINEL.md");
        let bytes_before = fs::read(&sentinel).unwrap();
        fs::write(&sentinel, "rogue rewrite from a disarmed guard\n").unwrap();
        assert_eq!(
            fs::read(&sentinel).unwrap(),
            bytes_before,
            "write-once violated: the entry's bytes were rewritten"
        );
        return;
    }

    let settings = tempfile::tempdir().unwrap();
    let out = spawn_child(
        "write_once_guard_disarmed_fails_exactly_its_own_assertion",
        &[
            (SETTINGS_DIR_ENV, settings.path().to_path_buf()),
            (RED_MARKER, PathBuf::from("1")),
        ],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "the disarmed-guard child must fail — it rewrote the entry and \
         the sentinel assertion must have caught it\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stderr.contains("write-once violated: the entry's bytes were rewritten"),
        "the child must fail on exactly the write-once sentinel assertion, got:\n{stderr}"
    );
}
