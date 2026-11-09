//! One capture of a monorepo at one commit, shared by every reader that is
//! alive at the same time (PROP-073 `#NATIVE-PREPARATION`).
//!
//! Under the controlled transport a monorepo is captured into an
//! operation-owned snapshot. An install reads the same repository in several
//! stages — version discovery, the solver's metadata, the held-row fetch —
//! and each used to capture it again (a 79 MB repository three times in
//! one cold install). A capture is now registered here under the canonical
//! url and both the requested reference and the commit it resolved to; a
//! later request for the same url at that reference or that commit reuses
//! it while any reader still holds it.
//!
//! Entries are weak: the snapshot directory lives exactly as long as its
//! last reader, so a long-lived process (`vibe mcp serve`) never pins old
//! snapshots, and a branch requested after every reader of an earlier
//! capture is gone is captured afresh — the reuse never outlives the
//! operation that made the capture.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, Weak};

/// A captured tree, its commit and the candidate url that served it.
#[derive(Debug)]
pub(crate) struct Snapshot {
    pub(crate) tree: PathBuf,
    pub(crate) commit: String,
    pub(crate) served_url: String,
    pub(crate) root: PathBuf,
    _dir: tempfile::TempDir,
}

impl Snapshot {
    pub(crate) fn new(
        dir: tempfile::TempDir,
        tree: PathBuf,
        commit: String,
        served_url: String,
    ) -> Self {
        Snapshot {
            root: dir.path().to_path_buf(),
            tree,
            commit,
            served_url,
            _dir: dir,
        }
    }
}

struct Entry {
    url: String,
    reference: String,
    snapshot: Weak<Snapshot>,
}

static LIVE: Mutex<Vec<Entry>> = Mutex::new(Vec::new());

/// The live capture of `url` at `reference` (a ref name or a commit), if a
/// reader still holds one.
pub(crate) fn find(url: &str, reference: &str) -> Option<Arc<Snapshot>> {
    let mut live = LIVE.lock().ok()?;
    live.retain(|entry| entry.snapshot.strong_count() > 0);
    live.iter()
        .filter(|entry| entry.url == url)
        .filter_map(|entry| entry.snapshot.upgrade().map(|s| (entry, s)))
        .find(|(entry, snapshot)| entry.reference == reference || snapshot.commit == reference)
        .map(|(_, snapshot)| snapshot)
}

/// Register a fresh capture of `url` requested at `reference`.
pub(crate) fn register(url: &str, reference: &str, snapshot: &Arc<Snapshot>) {
    // A poisoned registry only loses sharing, never correctness.
    if let Ok(mut live) = LIVE.lock() {
        live.retain(|entry| entry.snapshot.strong_count() > 0);
        live.push(Entry {
            url: url.to_string(),
            reference: reference.to_string(),
            snapshot: Arc::downgrade(snapshot),
        });
    }
}

/// Captures an owner keeps alive for its own lifetime — one resolver run —
/// so a later stage of the same run finds them though the stage that made
/// them has dropped its reader.
#[derive(Default)]
pub(crate) struct Retained(Mutex<Vec<Arc<Snapshot>>>);

impl Retained {
    pub(crate) fn keep(&self, snapshot: Option<&Arc<Snapshot>>) {
        let Some(snapshot) = snapshot else { return };
        if let Ok(mut held) = self.0.lock()
            && !held.iter().any(|kept| Arc::ptr_eq(kept, snapshot))
        {
            held.push(Arc::clone(snapshot));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(commit: &str) -> Arc<Snapshot> {
        let dir = tempfile::tempdir().unwrap();
        let tree = dir.path().join("tree");
        Arc::new(Snapshot::new(dir, tree, commit.into(), "u".into()))
    }

    #[test]
    fn a_live_capture_is_found_by_reference_or_commit_and_dies_with_its_readers() {
        let url = "ssh://shared-snapshot-test.invalid/vibe.git";
        let held = snapshot("c0ffee");
        let root = held.root.clone();
        register(url, "main", &held);
        assert_eq!(find(url, "main").unwrap().commit, "c0ffee");
        assert_eq!(find(url, "c0ffee").unwrap().commit, "c0ffee");
        assert!(find(url, "other").is_none());
        assert!(find("ssh://elsewhere.invalid/vibe.git", "main").is_none());
        drop(held);
        assert!(find(url, "main").is_none(), "no reader, no reuse");
        assert!(
            !root.exists(),
            "the snapshot directory went with its last reader"
        );
    }
}
