//! The safe-read refusals of the capability-backed state file: no link at
//! the final name, no link at the `.vibe` ancestor, no second name, no
//! directory, no non-UTF8 bytes, no over-cap read — and no partial parse of
//! any of them. Each case arms the disk with one hostile shape and asserts
//! the typed erasable-cache refusal, never a followed read, a truncated
//! parse or a silent overwrite.

use std::fs;
use std::path::Path;

use vibe_wire::generated::lifecycle_state::LifecycleState;

use super::{RUN_ID, lease};
use crate::state::io::STATE_CAP;
use crate::{LifecycleStateError, LifecycleStateStore};

const KEY: &str = "org.demo/tools#produce";

fn open(root: &Path) -> LifecycleStateStore {
    LifecycleStateStore::begin(
        lease(root),
        "create".into(),
        vec!["validate".into(), "install".into(), "create".into()],
        "2026-08-28T00:00:00Z".into(),
        RUN_ID.into(),
        false,
    )
    .unwrap()
}

/// Create a filesystem symlink at a FILE name. Privilege-gated on Windows
/// (Win32 1314 without Developer Mode), so every caller is an
/// explicitly-`#[ignore]`d test that ASSERTS the creation where it runs —
/// never a silently-returning pass.
#[cfg(unix)]
fn link(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

#[cfg(windows)]
fn link(target: &Path, link: &Path) -> bool {
    std::os::windows::fs::symlink_file(target, link).is_ok()
}

/// Link a DIRECTORY over a name. On Windows this is a JUNCTION, not a
/// symlink: `mklink /J` needs no Developer-Mode privilege, so the
/// no-follow-on-the-walk law gets an executing proof on ordinary hosts
/// (oracle pattern from `agent/tests`), rather than a skip exactly where it
/// matters most.
#[cfg(windows)]
fn link_dir(target: &Path, link: &Path) -> bool {
    let status = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .expect("mklink is available on Windows");
    assert!(
        status.status.success(),
        "mklink /J needs no privilege; its failure is an environment worth failing on: {}",
        String::from_utf8_lossy(&status.stderr),
    );
    true
}

#[cfg(unix)]
fn link_dir(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).is_ok()
}

/// The unsafe-shape refusal: typed as a READ refusal (the capability cell
/// refusing the shape), not only the erasable-cache prose five variants share.
fn assert_read_refusal(result: Result<Option<LifecycleState>, LifecycleStateError>, label: &str) {
    let error = result.expect_err(label);
    assert!(
        matches!(error, LifecycleStateError::Read { .. }),
        "{label}: the refusal is a typed read refusal: {error}",
    );
    let rendered = error.to_string();
    assert!(
        rendered.contains("remove this erasable cache"),
        "{label}: {rendered}",
    );
}

/// A symlink at the state file's own name refuses: the read must not follow
/// to the target's perfectly valid content, because the FILE AT THE NAME is
/// the thing that must be a regular single-link file. Privilege-gated on
/// Windows (Win32 1314 without Developer Mode), so the test carries an
/// explicit `#[ignore]` there — a skip reads as a skip, never as a pass —
/// and ASSERTS the symlink creation wherever it does run.
#[test]
#[cfg_attr(
    windows,
    ignore = "requires Windows symlink/reparse privilege (worker host returned Win32 1314)"
)]
fn a_symlinked_state_file_refuses_rather_than_following() {
    let dir = tempfile::tempdir().unwrap();
    let store = open(dir.path());
    let outside = dir.path().join("outside.toml");
    fs::write(&outside, fs::read(store.path()).unwrap()).unwrap();
    fs::remove_file(store.path()).unwrap();
    assert!(
        link(&outside, store.path()),
        "where this test runs, the symlink oracle must actually create the link",
    );
    assert_read_refusal(LifecycleStateStore::peek(dir.path()), "a linked state");
}

/// A `.vibe` ancestor that is itself a link refuses the same way: the walk
/// to the state file is one component at a time, no-follow, so a rehosted
/// `.vibe` cannot redirect the state read outside the workspace root. On
/// Windows the link is a JUNCTION (`mklink /J`, no privilege), so this proof
/// EXECUTES on ordinary hosts rather than skipping where it matters most.
#[test]
fn a_linked_vibe_ancestor_refuses_rather_than_following() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real-vibe");
    fs::create_dir_all(&real).unwrap();
    fs::write(
        real.join("lifecycle.toml"),
        "schema = 1\n[run]\nrequested = 'x'\nchain = []\nstarted = 't'\n[execution]\n",
    )
    .unwrap();
    assert!(
        link_dir(&real, &dir.path().join(".vibe")),
        "the linked-ancestor oracle must actually create the link",
    );
    assert_read_refusal(
        LifecycleStateStore::peek(dir.path()),
        "a linked `.vibe` ancestor",
    );
}

/// A second hard link refuses, and this proof may not skip: both names live
/// in the same temp fixture on the same filesystem. A state file another
/// name can also rewrite is not exclusively owned by the lifecycle.
#[test]
fn a_hardlinked_state_file_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let store = open(dir.path());
    fs::hard_link(store.path(), dir.path().join("second.toml")).expect(
        "both names sit in one temp fixture on one filesystem; a hard link must be creatable \
             here or the single-link law has no proof on this host",
    );
    assert_read_refusal(LifecycleStateStore::peek(dir.path()), "a hardlinked state");
}

/// A directory at the state path refuses: it is not a regular file, and a
/// read that "helpfully" walked into it would be following shape, not law.
#[test]
fn a_directory_at_the_state_path_refuses() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join(".vibe/lifecycle.toml")).unwrap();
    assert_read_refusal(LifecycleStateStore::peek(dir.path()), "a directory state");
}

/// A workspace root that is not absolute is a ROOT problem, not a state
/// problem: the typed refusal names the root and its remedy says to pass the
/// canonical absolute root. It is NOT a `Read`, and it never advises
/// deleting the state cache — the cache is intact and irrelevant.
#[test]
fn a_relative_workspace_root_refuses_without_advising_cache_deletion() {
    let error = LifecycleStateStore::peek(Path::new("relative/ws"))
        .expect_err("a relative root cannot be pinned");
    assert!(
        matches!(error, LifecycleStateError::Root { .. }),
        "the refusal is the typed root refusal: {error}",
    );
    let rendered = error.to_string();
    assert!(
        !rendered.contains("remove this erasable cache"),
        "a root problem must never advise deleting the state: {rendered}",
    );
    assert!(
        rendered.to_lowercase().contains("absolute"),
        "the remedy says to pass an absolute root: {rendered}",
    );
    assert!(rendered.contains("relative/ws"), "{rendered}");
}

/// The same law for a root that is absolute but does not exist: still a ROOT
/// refusal naming the root, never a `Read` and never cache-deletion advice —
/// there may be no state file at all yet, and the remedy is to pass the
/// root that exists.
#[test]
fn a_missing_workspace_root_refuses_without_advising_cache_deletion() {
    let missing = tempfile::tempdir()
        .unwrap()
        .path()
        .join("no-such-workspace");
    let error = LifecycleStateStore::peek(&missing).expect_err("a missing root cannot be pinned");
    assert!(
        matches!(error, LifecycleStateError::Root { .. }),
        "the refusal is the typed root refusal: {error}",
    );
    let rendered = error.to_string();
    assert!(
        !rendered.contains("remove this erasable cache"),
        "a root problem must never advise deleting the state: {rendered}",
    );
    assert!(
        rendered.to_lowercase().contains("absolute"),
        "the remedy says to pass an absolute root: {rendered}",
    );
    assert!(
        rendered.contains("no-such-workspace"),
        "the remedy names the actual root: {rendered}",
    );
}

/// Non-UTF8 bytes refuse as their own typed outcome, before TOML is ever
/// asked to parse: an encoding refusal is not a malformed-document refusal,
/// and conflating them hides what the operator must remove.
#[test]
fn a_non_utf8_state_file_refuses_before_parsing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(LifecycleStateStore::FILE);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"schema = 1\n[run]\nstarted = \xff\xfe\n").unwrap();
    let error = LifecycleStateStore::peek(dir.path())
        .expect_err("non-UTF8 bytes are not a state")
        .to_string();
    assert!(error.contains("not valid UTF-8"), "{error}");
    assert!(error.contains("remove this erasable cache"), "{error}");
}

/// One byte over the 8 MiB cap refuses with the real length and the cap in
/// the message, and — the part an after-the-fact check cannot promise — no
/// parse ever happens: the refusal is not `Malformed`, because the bytes
/// were never handed to the parser.
#[test]
fn an_over_cap_state_file_refuses_whole_not_parsed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(LifecycleStateStore::FILE);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut hostile = vec![b' '; STATE_CAP + 1];
    hostile[0] = b'#';
    fs::write(&path, &hostile).unwrap();
    let error =
        LifecycleStateStore::peek(dir.path()).expect_err("cap + 1 bytes are not a bounded read");
    assert!(
        matches!(error, LifecycleStateError::Read { .. }),
        "the over-cap refusal is a read refusal: {error}",
    );
    let rendered = error.to_string();
    assert!(rendered.contains("8388608"), "the cap: {rendered}");
    assert!(rendered.contains("8388609"), "the real length: {rendered}");
    assert!(
        !rendered.contains("malformed"),
        "no partial parse may be reported: {rendered}",
    );
}

/// A CANDIDATE that encodes over the read cap is refused before any
/// publication attempt: the store never writes bytes its own bounded read
/// would refuse next time — that would be manufacturing an unreadable state
/// out of a healthy one. The refusal names the size and the cap, and the
/// transaction leaves the exact prior memory, the exact prior bytes and no
/// staging file behind.
#[test]
fn an_over_cap_candidate_is_refused_before_publication() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(dir.path());
    store
        .checkpoint(
            KEY.into(),
            super::record_for(
                KEY,
                RUN_ID,
                super::ExecutionRecordStatus::Ok,
                "sha256:prior",
            ),
        )
        .unwrap();
    let before = store.state().clone();
    let prior_bytes = fs::read(store.path()).unwrap();

    // One legitimate row whose fingerprint alone pushes the encoding past the
    // cap: the generated type imposes no length law the validator could have
    // caught earlier, so the size gate is the only thing standing.
    let runaway = "a".repeat(STATE_CAP + 1);
    let error = store
        .checkpoint(
            "org.demo/tools#runaway".into(),
            super::record_for(
                "org.demo/tools#runaway",
                RUN_ID,
                super::ExecutionRecordStatus::Ok,
                &runaway,
            ),
        )
        .expect_err("a candidate over the read cap is never published");
    let LifecycleStateError::TooLarge { size, cap, .. } = &error else {
        panic!("the refusal is the typed size gate: {error}");
    };
    assert_eq!(*cap, STATE_CAP);
    assert!(*size > STATE_CAP, "the real encoded size: {size}");
    let rendered = error.to_string();
    assert!(rendered.contains(&STATE_CAP.to_string()), "{rendered}");
    assert!(
        rendered.contains("could not read back") || rendered.contains("never publishes"),
        "the remediation names the law: {rendered}",
    );

    assert_eq!(*store.state(), before, "the in-memory state did not move");
    assert_eq!(
        fs::read(store.path()).unwrap(),
        prior_bytes,
        "the durable bytes did not move",
    );
    let mut names: Vec<String> = fs::read_dir(dir.path().join(".vibe"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        // The mutation lease's own lock file is infrastructure of the
        // acquiring command, not state — see `support::vibe_names`.
        .filter(|name| name != crate::lease::LOCK_NAME)
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec!["lifecycle.toml".to_string()],
        "refused before publication means no stage was ever created",
    );
    assert!(store.poison_reason().is_none(), "nothing was poisoned");
}

/// The successful path is unchanged by the hardening: `begin` and
/// `checkpoint` still produce exactly the bytes the generated type's pretty
/// TOML encoding spells — same sections, same field order, byte for byte.
#[test]
fn begin_and_checkpoint_still_write_the_exact_generated_toml_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = open(dir.path());
    store
        .checkpoint(
            KEY.into(),
            super::record_for(KEY, RUN_ID, super::ExecutionRecordStatus::Ok, "sha256:x"),
        )
        .unwrap();
    let durable = fs::read(store.path()).unwrap();
    let expected = toml::to_string_pretty(store.state()).unwrap().into_bytes();
    assert_eq!(
        durable, expected,
        "the durable bytes are the generated encoding of the proven state",
    );
    let parsed: LifecycleState = toml::from_str(&String::from_utf8_lossy(&durable)).unwrap();
    assert_eq!(parsed, *store.state());
    assert!(durable.starts_with(b"schema = 1\n"));
    assert!(
        String::from_utf8_lossy(&durable).contains("[execution.\"org.demo/tools#produce\"]"),
        "the execution section keeps its exact spelling",
    );
}
