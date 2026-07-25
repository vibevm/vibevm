//! DRIFT-017 — a run that changes nothing writes nothing.
//!
//! Every assertion here is about the *disk*, not about the answer: the
//! answers were already pinned by DRIFT-010's warm/cold equality tests,
//! and this task changes none of them. What it changes is how often the
//! campaign's several megabytes are serialised, fsync'd and renamed over
//! the top of bytes that already said the same thing.
//!
//! So the instrument is the modification time. A skipped write does not
//! open the file, so its mtime is the sharpest available statement that
//! nothing happened to it — sharper than the reported tally, which is
//! this code's own opinion of what it did.

use super::*;
use std::collections::BTreeMap;
use std::time::SystemTime;

/// Every artifact one scan of a fixture campaign can write, in the two
/// places they live: the tracked cache beside the journal, the five
/// projections under `state/`, and the payload sidecar outside both.
fn artifacts(root: &Path) -> Vec<PathBuf> {
    let run = root.join("campaigns/progress-test/run");
    let mut all = vec![run.join("cache.json")];
    for name in [
        "corpus.json",
        "campaign.json",
        "findings.json",
        "tasks.json",
        "docdebt.json",
    ] {
        all.push(run.join("state").join(name));
    }
    all.push(sidecar_dir(root).join(sidecar::PAYLOAD_FILE));
    all
}

/// When each artifact that exists was last modified. An absent file is
/// absent from the map rather than defaulted, so "it appeared" and "it was
/// rewritten" stay two different observations.
fn mtimes(root: &Path) -> BTreeMap<PathBuf, SystemTime> {
    artifacts(root)
        .into_iter()
        .filter_map(|p| {
            let t = std::fs::metadata(&p).ok()?.modified().ok()?;
            Some((p, t))
        })
        .collect()
}

/// A fixture tree, scanned twice, so the campaign zone is in the steady
/// state every assertion below starts from: the first scan creates the
/// artifacts, the second is the run that must already be doing nothing.
///
/// Propagates rather than panicking — the decision to fail the run belongs
/// to the `#[test]` that called it, which is the same rule the floor
/// caught `incremental_fixture` on in DRIFT-010 and the sidecar's
/// `payload_for` in DRIFT-016.
fn settled(root: &Path, ctx: &crate::output::Context) -> Result<()> {
    incremental_fixture(root)?;
    scan(ctx, &args(root, false))?;
    scan(ctx, &args(root, false))
}

/// The task's headline (§6): scan a fixture twice, take every mtime after
/// the first, and assert none of them moved after the second.
///
/// The reported tally is checked against the same run, because §4.3 makes
/// the skip observable rather than merely real — a `--json` consumer that
/// is told `written: true` while the file did not move is being lied to
/// just as surely as one told nothing at all.
#[test]
fn second_scan_writes_nothing() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    let ctx = crate::output::Context::from_flags(true, false, None, true);
    incremental_fixture(root).expect("fixture tree");

    scan(&ctx, &args(root, false)).expect("first scan");
    let after_first = mtimes(root);
    assert_eq!(
        after_first.len(),
        artifacts(root).len(),
        "the first scan wrote every artifact — an empty zone has nothing to skip"
    );

    scan(&ctx, &args(root, false)).expect("second scan");
    assert_eq!(
        mtimes(root),
        after_first,
        "nothing moved on the second scan"
    );

    // …and the run says so, artifact by artifact.
    let mut g = ground(&args(root, false)).expect("ground");
    let refreshed = refresh_state(&mut g).expect("refresh");
    assert_eq!(
        refreshed.writes.values().filter(|w| **w).count(),
        0,
        "every artifact reported as skipped: {:?}",
        refreshed.writes
    );
    assert_eq!(refreshed.tally(), (0, artifacts(root).len()));
    assert_eq!(mtimes(root), after_first, "…and the third run agrees");
}

/// The other half of the same claim: the skip is keyed on content, so an
/// edit lands. One marked fact is added to one file, which moves that
/// file's record, the corpus row built from it, the counters
/// `campaign.json` carries, and the payload behind all three.
///
/// The three passthrough projections must *not* move: they belong to other
/// subsystems and this run has nothing to say about them.
#[test]
fn edited_file_forces_the_write() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    let ctx = crate::output::Context::from_flags(true, false, None, true);
    settled(root, &ctx).expect("a settled fixture campaign");
    let before = mtimes(root);

    let text = std::fs::read_to_string(root.join("spec/a.md")).expect("read a");
    std::fs::write(
        root.join("spec/a.md"),
        format!("{text}\n##a5 A newly added claim. @freeze/done\n"),
    )
    .expect("edit a");

    let mut g = ground(&args(root, false)).expect("ground");
    let refreshed = refresh_state(&mut g).expect("refresh");
    let after = mtimes(root);

    for name in [
        "cache.json",
        "corpus.json",
        "campaign.json",
        "payloads.json",
    ] {
        assert_eq!(
            refreshed.writes.get(name),
            Some(&true),
            "`{name}` reported as written"
        );
    }
    for (path, was) in &before {
        let name = path
            .file_name()
            .expect("name")
            .to_string_lossy()
            .into_owned();
        let now = after.get(path).expect("artifact still there");
        match name.as_str() {
            "findings.json" | "tasks.json" | "docdebt.json" => {
                assert_eq!(now, was, "`{name}` belongs to another subsystem");
                assert_eq!(refreshed.writes.get(&name), Some(&false), "{name}");
            }
            _ => assert_ne!(now, was, "`{name}` carries the edit and was rewritten"),
        }
    }

    // The edit is in the file, not merely in its timestamp.
    let corpus = read_state(root, "corpus.json");
    assert!(corpus.contains("\"markers\": 6"), "the new marker landed");
}

/// §5, absolute: the campaign verdicts are the one thing worth an
/// unconditional fsync, so no comparison may ever swallow a change to
/// them. Both directions are asserted — a moved map is always written, and
/// a map that leaves the observed scope is a moved map too.
#[test]
fn verdict_change_always_writes() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    let ctx = crate::output::Context::from_flags(true, false, None, true);
    settled(root, &ctx).expect("a settled fixture campaign");
    let cache_path = root.join("campaigns/progress-test/run/cache.json");

    // A verdict arrives the way the campaign writes them.
    let mut c = cache::Cache::load(&cache_path).expect("load");
    c.files
        .get_mut("spec/a.md")
        .expect("record")
        .campaign
        .insert("verdicts".into(), serde_json::json!({"alpha": "confirmed"}));
    assert!(
        c.store(&cache_path).expect("store"),
        "a moved map is written"
    );
    assert!(
        !c.store(&cache_path).expect("restore"),
        "…and the identical map behind it is not"
    );
    assert!(
        std::fs::read_to_string(&cache_path)
            .expect("read")
            .contains("confirmed"),
        "the verdict is on disk"
    );

    // The scan that follows finds the cache already saying what it would
    // have said, so it does not rewrite it — but `corpus.json` carries
    // each record's campaign map, and that map moved. The projection is
    // never left behind a verdict just because the cache was not rewritten.
    let mut g = ground(&args(root, false)).expect("ground");
    let refreshed = refresh_state(&mut g).expect("refresh");
    assert_eq!(
        refreshed.writes.get("cache.json"),
        Some(&false),
        "the cache already held the verdict"
    );
    assert_eq!(
        refreshed.writes.get("corpus.json"),
        Some(&true),
        "the projection caught up with it"
    );
    assert!(read_state(root, "corpus.json").contains("confirmed"));
    assert!(
        cache::Cache::load(&cache_path).expect("reload").files["spec/a.md"]
            .campaign
            .contains_key("verdicts"),
        "the verdict survived the run that skipped the write"
    );

    // Now the record leaves the observed scope: the maps genuinely differ,
    // so the file must move even though nothing in the tree was edited.
    std::fs::write(root.join("progress.toml"), fixture_config("spec/b.md"))
        .expect("narrow the scope");
    let mut g = ground(&args(root, false)).expect("ground narrow");
    let refreshed = refresh_state(&mut g).expect("refresh narrow");
    assert_eq!(refreshed.writes.get("cache.json"), Some(&true));
    assert!(
        !std::fs::read_to_string(&cache_path)
            .expect("read")
            .contains("confirmed"),
        "the pruned record's verdict is gone, and the prune was written"
    );
}

/// The edge cases §4 names, on every artifact rather than on a chosen one:
/// a file that is not there is always written, and a file that is there
/// but cannot be read is replaced rather than trusted.
///
/// Both are the same rule seen twice — a comparison that cannot prove the
/// file already says this falls back to writing. The exception is named
/// where it bites: the three passthrough projections are seeded when
/// absent and never rewritten, here or before this task, because they
/// belong to other subsystems. Replacing a torn `findings.json` with an
/// empty one is data loss wearing repair's clothes.
#[test]
fn absent_file_is_always_written() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path();
    let ctx = crate::output::Context::from_flags(true, false, None, true);
    settled(root, &ctx).expect("a settled fixture campaign");

    // Absent ⇒ written. Every artifact, including the passthroughs, which
    // is the one case this run does have something to say about them.
    for path in artifacts(root) {
        let name = path
            .file_name()
            .expect("name")
            .to_string_lossy()
            .into_owned();
        std::fs::remove_file(&path).unwrap_or_else(|e| panic!("remove {name}: {e}"));
        let mut g = ground(&args(root, false)).expect("ground");
        let refreshed = refresh_state(&mut g).expect("refresh");
        assert_eq!(
            refreshed.writes.get(&name),
            Some(&true),
            "`{name}` was absent and got written"
        );
        assert!(path.is_file(), "`{name}` was put back");
    }

    // Present but unreadable ⇒ replaced, for the four this run derives.
    for name in [
        "cache.json",
        "corpus.json",
        "campaign.json",
        "payloads.json",
    ] {
        let path = artifacts(root)
            .into_iter()
            .find(|p| p.file_name().is_some_and(|n| n == name))
            .unwrap_or_else(|| panic!("no artifact named {name}"));
        std::fs::write(&path, [0xff, 0xfe, b'{']).expect("clobber");
        let mut g = ground(&args(root, false)).expect("ground");
        let refreshed = refresh_state(&mut g).expect("refresh");
        assert_eq!(
            refreshed.writes.get(name),
            Some(&true),
            "`{name}` was unreadable and got rewritten"
        );
        assert!(
            serde_json::from_str::<serde_json::Value>(
                &std::fs::read_to_string(&path).expect("read back")
            )
            .is_ok(),
            "`{name}` is valid JSON again"
        );
    }
}
