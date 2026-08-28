//! The prelude epoch's own reds — currently the ONE trace-home join.

use super::*;

use vibe_lifecycle::RunIdentity;
use vibe_wire::generated::shared::{Timestamp, TraceReportStatus};

const PLAIN: &str = "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n";

/// A run id and start spelled exactly as `select_run_identity` hands them over.
fn identity(compile_trace: bool) -> RunIdentity {
    RunIdentity {
        run_id: "0123456789abcdef0123456789abcdef".to_string(),
        started: "2026-08-28T10:00:00Z".to_string(),
        adopted: false,
        compile_trace,
        superseded_trace: None,
    }
}

/// One epoch over a real selection, assembled exactly as `run_prelude` returns
/// it — the point is to drive the REAL join, not a stand-in for it.
fn prelude(selection: PreparedSelection, compile_trace: bool) -> RunPrelude {
    RunPrelude {
        identity: identity(compile_trace),
        selection,
        lease: vibe_test_support::retained_lifecycle_lease(),
        selected: ".".to_string(),
    }
}

/// A counting clock, so "this path never asked what time it is" is an
/// assertion rather than a hope.
struct Ticks(std::cell::Cell<usize>);

impl Ticks {
    fn new() -> Self {
        Self(std::cell::Cell::new(0))
    }

    fn clock(&self) -> impl Fn() -> Timestamp + '_ {
        move || {
            self.0.set(self.0.get() + 1);
            Timestamp::from_timestamp(0, 0).expect("a fixture instant is representable")
        }
    }

    fn calls(&self) -> usize {
        self.0.get()
    }
}

/// A LOADED tree names the one canonical trace home, and the owner really
/// opens there.
#[test]
fn a_loaded_epoch_opens_its_trace_under_the_canonical_workspace_root() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("vibe.toml"), PLAIN).unwrap();
    let root = crate::install::resolve_project_root(dir.path()).unwrap();
    let selection = crate::SelectedManifest::read(&root).prepare();
    let home = selection
        .loaded_root()
        .expect("the fixture really loads a tree")
        .to_path_buf();

    let clock = Ticks::new();
    let preparation = prelude(selection, true).prepare_trace(&clock.clock());

    assert!(preparation.trace_requested());
    assert!(preparation.recorder().is_some(), "the run really opened");
    drop(preparation);
    assert_eq!(clock.calls(), 0, "nothing was displaced, so nothing asked");
    assert!(
        home.join(".vibe")
            .join("trace")
            .join("0123456789abcdef0123456789abcdef")
            .exists(),
        "the trace lives under the LOADED root, not somewhere re-derived",
    );
}

/// Discovery failed, so there is no canonical root to name — and the join
/// stands down instead of substituting the selected project root.
///
/// This is the arm the wrong version gets wrong. Falling back to
/// `selection.root()` compiles, passes every behaviour test that only looks at
/// a happy tree, and silently lets two members of one workspace hold
/// independent trace locks over the same work. Here it is a red: the epoch must
/// create NOTHING on disk, and must still say the trace was REQUESTED so the
/// report can explain itself.
#[test]
fn an_unloadable_epoch_stands_down_without_a_lock_or_a_tree() {
    let dir = tempfile::tempdir().unwrap();
    // No manifest at all: the snapshot does not parse, so no load is attempted
    // and `loaded_root()` is `None` while `root()` is still this directory.
    let selection = crate::SelectedManifest::read(dir.path()).prepare();
    assert!(selection.loaded_root().is_none(), "the premise");
    assert_eq!(
        selection.root(),
        dir.path(),
        "and a root IS still available"
    );

    let clock = Ticks::new();
    let preparation = prelude(selection, true).prepare_trace(&clock.clock());

    assert!(
        preparation.recorder().is_none(),
        "no recorder without a canonical root",
    );
    assert!(
        preparation.trace_requested(),
        "but the request is still owed an explanation",
    );
    assert_eq!(clock.calls(), 0);
    assert!(
        !dir.path().join(".vibe").exists(),
        "and nothing was created under the selected root to hold it",
    );

    // The member says WHICH way it failed, rather than looking like a run that
    // had nothing to record.
    let finalized = crate::trace::finalize(
        preparation,
        crate::trace::CommandExit::Success(()),
        &clock.clock(),
    );
    let member = finalized.trace.expect("a requested trace reports its fate");
    assert_eq!(member.status, TraceReportStatus::Unavailable);
    assert_eq!(clock.calls(), 0, "there was nothing to finish");
}

/// The request still decides the session on the stand-down arm: an untraced
/// invocation is `disabled`, not a silent `unavailable`.
#[test]
fn an_unrequested_unloadable_epoch_is_disabled_rather_than_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    let selection = crate::SelectedManifest::read(dir.path()).prepare();

    let clock = Ticks::new();
    let preparation = prelude(selection, false).prepare_trace(&clock.clock());

    assert!(!preparation.trace_requested());
    let finalized = crate::trace::finalize(
        preparation,
        crate::trace::CommandExit::Success(()),
        &clock.clock(),
    );
    assert!(finalized.trace.is_none(), "off means off");
    assert_eq!(clock.calls(), 0);
}
