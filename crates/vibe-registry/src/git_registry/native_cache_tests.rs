//! What a durable snapshot may be reused for, and what invalidates it.

use std::fs;

use super::*;

fn state(cache: &NativeCache, reference: &str) -> NativeSnapshotState {
    NativeSnapshotState {
        schema: SCHEMA,
        url: crate::registry_cache::normalize_url(cache.url()),
        reference: reference.to_owned(),
        refname: "refs/heads/main".into(),
        served_url: cache.url().to_owned(),
        object: "a".repeat(40),
        commit: "a".repeat(40),
        generation: "snapshot-fixture".into(),
        tree_digest: "deadbeef".into(),
        admission: "cafebabe".into(),
        checked_at: vibe_core::timestamp::now_utc(),
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-PREPARATION-LAW")]
fn a_state_is_read_back_only_for_its_own_url_reference_and_schema() {
    let root = tempfile::tempdir().unwrap();
    let url = "ssh://corp.invalid/mono.git";
    let cache = NativeCache::at(root.path(), url).unwrap();
    assert!(cache.state("main").is_none(), "nothing recorded yet");

    let recorded = state(&cache, "main");
    cache.write_state(&recorded).unwrap();
    assert_eq!(cache.state("main").unwrap(), recorded);
    // Another reference of the same repository is another snapshot.
    assert!(cache.state("release").is_none());
    // Another repository has its own directory and never reads this one.
    let other = NativeCache::at(root.path(), "ssh://corp.invalid/other.git").unwrap();
    assert!(other.state("main").is_none());
    assert_ne!(other.root(), cache.root());

    // A schema this vibe does not read is answered by capturing afresh,
    // not by a partial read.
    let mut future = recorded.clone();
    future.schema = SCHEMA + 1;
    cache.write_state(&future).unwrap();
    assert!(cache.state("main").is_none());

    // So is a file this vibe cannot parse at all, and one with a field a
    // newer vibe added (`deny_unknown_fields`).
    fs::write(cache.root().join("native.toml"), "not toml at all {{").unwrap();
    assert!(cache.state("main").is_none());
    let mut raw = toml::to_string_pretty(&recorded).unwrap();
    raw.push_str("\nsomething_newer = true\n");
    fs::write(cache.root().join("native.toml"), raw).unwrap();
    assert!(cache.state("main").is_none());
}

#[test]
fn the_same_url_spelling_and_its_normalised_twin_share_one_directory() {
    let root = tempfile::tempdir().unwrap();
    let plain = NativeCache::at(root.path(), "ssh://corp.invalid/mono.git").unwrap();
    let trailing = NativeCache::at(root.path(), "ssh://corp.invalid/mono.git/").unwrap();
    assert_eq!(plain.root(), trailing.root(), "one registry, one cache");
    plain.write_state(&state(&plain, "main")).unwrap();
    assert!(
        trailing.state("main").is_some(),
        "the state is about the normalised url, so either spelling reads it"
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION-LAW"
)]
fn the_tree_witness_sees_content_mode_and_shape_and_a_missing_tree_is_none() {
    let root = tempfile::tempdir().unwrap();
    let tree = root.path().join("tree");
    assert_eq!(tree_digest(&tree).unwrap(), None, "no tree, no digest");

    fs::create_dir_all(tree.join("org.corp/tool/v1.0.0")).unwrap();
    let manifest = tree.join("org.corp/tool/v1.0.0/vibe.toml");
    fs::write(&manifest, b"[package]\nname = 'tool'\n").unwrap();
    let first = tree_digest(&tree).unwrap().unwrap();
    assert_eq!(tree_digest(&tree).unwrap().unwrap(), first, "stable");

    // One byte of one file is a different tree.
    fs::write(&manifest, b"[package]\nname = 'Tool'\n").unwrap();
    assert_ne!(tree_digest(&tree).unwrap().unwrap(), first);
    fs::write(&manifest, b"[package]\nname = 'tool'\n").unwrap();
    assert_eq!(tree_digest(&tree).unwrap().unwrap(), first);

    // So is one more file, and — on Unix — one changed mode.
    let extra = tree.join("org.corp/tool/v1.0.0/extra");
    fs::write(&extra, b"").unwrap();
    assert_ne!(tree_digest(&tree).unwrap().unwrap(), first);
    fs::remove_file(&extra).unwrap();
    assert_eq!(tree_digest(&tree).unwrap().unwrap(), first);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&manifest, fs::Permissions::from_mode(0o755)).unwrap();
        assert_ne!(
            tree_digest(&tree).unwrap().unwrap(),
            first,
            "an executable bit is part of what was staged"
        );
    }
}

#[test]
fn a_freshness_window_of_zero_always_asks_and_an_unreadable_stamp_never_holds() {
    let root = tempfile::tempdir().unwrap();
    let cache = NativeCache::at(root.path(), "ssh://corp.invalid/mono.git").unwrap();
    let now = state(&cache, "main");
    assert!(!within_window(&now, 0), "a zero window always asks");
    assert!(within_window(&now, 3600));

    let mut ancient = now.clone();
    ancient.checked_at = "2001-01-01T00:00:00Z".into();
    assert!(!within_window(&ancient, 3600));

    let mut nonsense = now.clone();
    nonsense.checked_at = "whenever".into();
    assert!(
        !within_window(&nonsense, 3600),
        "a stamp that cannot be read is not a fresh one"
    );
}

#[test]
fn a_touch_moves_only_the_timestamp() {
    let root = tempfile::tempdir().unwrap();
    let cache = NativeCache::at(root.path(), "ssh://corp.invalid/mono.git").unwrap();
    let mut old = state(&cache, "main");
    old.checked_at = "2001-01-01T00:00:00Z".into();
    cache.write_state(&old).unwrap();
    cache.touch(&old);
    let read = cache.state("main").unwrap();
    assert_ne!(read.checked_at, old.checked_at);
    assert_eq!(
        NativeSnapshotState {
            checked_at: old.checked_at.clone(),
            ..read
        },
        old,
        "nothing but the stamp moved"
    );
    assert!(within_window(&cache.state("main").unwrap(), 3600));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-PREPARATION-LAW")]
fn the_write_lock_is_exclusive_between_processes() {
    let root = tempfile::tempdir().unwrap();
    let cache = NativeCache::at(root.path(), "ssh://corp.invalid/mono.git").unwrap();
    let held = cache.lock().unwrap();
    // The same advisory lock another process would take: a second open of
    // the file cannot acquire it while the first holds it.
    let path = cache.root().join("native.lock");
    let rival = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    assert!(
        rival.try_lock().is_err(),
        "a second writer must wait for the first"
    );
    drop(held);
    assert!(
        rival.try_lock().is_ok(),
        "and may proceed once it is released"
    );
}
