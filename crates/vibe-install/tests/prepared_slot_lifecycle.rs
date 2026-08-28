//! Reds for the PREPARED slot-lifecycle constructor.
//!
//! `from_projection_observed_prepared` is the real constructor and discovers
//! nothing: the workspace root that anchors the run state and the slot lock,
//! the node envelopes the world is built from, and the removed-park
//! reconciliation all read the value the caller handed it.
//!
//! Two facts prove that, and both need a workspace whose root is NOT the
//! selected project — otherwise "used the prepared value" and "rediscovered
//! from the project root" produce the same answer and nothing is being tested.

use std::sync::Arc;

use vibe_core::manifest::Manifest;
use vibe_core::{ContentHash, Group, PackageKind};
use vibe_install::{InstallSlotLifecycle, SlotLifecycleSeams};
use vibe_lifecycle::process::StreamMode;
use vibe_lifecycle::{LifecycleLease, RunMetadata};
use vibe_wire::generated::lifecycle::e1::context::RunAgentMode;
use vibe_workspace::Workspace;
use vibe_workspace::install::ResolvedDep;

fn metadata(root: &std::path::Path) -> RunMetadata {
    RunMetadata {
        requested: "install".into(),
        chain: vec!["validate".into(), "install".into()],
        offline: true,
        assume_yes: true,
        agent_mode: RunAgentMode::Cli,
        force: false,
        trace_compile: false,
        run_id: vibe_lifecycle::process::allocate_run_id(root).unwrap(),
        started: "2026-08-27T00:00:00Z".into(),
    }
}

/// An outer workspace root with one member, `app`, plus a materialised slot
/// under the ROOT's `vibedeps/` — which is where a workspace keeps them.
///
/// The selected project is the member. So `workspace.root` and `project_root`
/// are different directories, and every place the constructor uses one of them
/// is separately observable.
struct Fixture {
    dir: tempfile::TempDir,
    manifest: Manifest,
    resolution: Vec<ResolvedDep>,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("vibe.toml"),
            "[project]\nname = \"outer\"\ngroup = \"org.demo\"\nversion = \"0.1.0\"\n\n\
             [workspace]\nmembers = [\"app\"]\n",
        )
        .unwrap();
        let app = dir.path().join("app");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(
            app.join("vibe.toml"),
            "[package]\ngroup = \"org.demo\"\nname = \"app\"\nkind = \"flow\"\n\
             version = \"0.1.0\"\n",
        )
        .unwrap();

        let slot = dir
            .path()
            .join(vibe_core::layout::current_vibedeps_root())
            .join("org.demo.tools")
            .join("1.0.0");
        std::fs::create_dir_all(&slot).unwrap();
        std::fs::write(
            slot.join("vibe.toml"),
            "[package]\ngroup = \"org.demo\"\nname = \"tools\"\nkind = \"tool\"\n\
             version = \"1.0.0\"\n",
        )
        .unwrap();

        let manifest = Manifest::read(app.join("vibe.toml")).unwrap();
        let dep = ResolvedDep {
            kind: PackageKind::Tool,
            group: Group::parse("org.demo").unwrap(),
            name: "tools".into(),
            version: "1.0.0".parse().unwrap(),
            content_dir: slot.clone(),
            source_hash: Some(ContentHash::parse("sha256:aa").unwrap()),
            manifest: Manifest::read(slot.join("vibe.toml")).unwrap(),
            requires: Vec::new(),
            admitted_by: None,
            via_override: None,
            source_mutable: false,
            in_place_changed: None,
        };
        Self {
            dir,
            manifest,
            resolution: vec![dep],
        }
    }

    fn project_root(&self) -> std::path::PathBuf {
        self.dir.path().join("app")
    }

    fn prepared(&self) -> Workspace {
        Workspace::discover(self.project_root()).expect("the member's tree discovers")
    }
}

fn build_prepared(
    fixture: &Fixture,
    workspace: &Workspace,
) -> Result<InstallSlotLifecycle, vibe_install::Error> {
    let project_root = fixture.project_root();
    // A real lease over the PREPARED value's own root: the constructor
    // anchors state where the lease pins, and this test's whole point is
    // that the prepared root — not a rediscovery — is the one that wins.
    let lease = Arc::new(LifecycleLease::acquire(&workspace.root).unwrap());
    InstallSlotLifecycle::from_projection_observed_prepared(
        &project_root,
        &fixture.manifest,
        &fixture.resolution,
        &fixture.resolution,
        workspace,
        metadata(&project_root),
        StreamMode::Capture,
        SlotLifecycleSeams::refusing(),
        lease,
    )
}

/// The run is anchored at the PREPARED workspace's root.
///
/// The member is the selected project, so a constructor that rediscovered from
/// `project_root` would still find the outer root and this would prove nothing
/// — which is why the prepared value handed in is deliberately a DIFFERENT
/// tree: one rooted at the member itself. The run state must appear where that
/// value says, not where a rediscovery would have put it.
#[test]
fn the_prepared_workspace_root_anchors_the_run() {
    let fixture = Fixture::new();
    // A tree rooted at the member — the shape a standalone `app` would have.
    let mut standalone = fixture.prepared();
    standalone.root = fixture.project_root();
    standalone.members.clear();

    let _lifecycle = build_prepared(&fixture, &standalone).expect("the lifecycle constructs");

    assert!(
        vibe_lifecycle::LifecycleStateStore::peek(&fixture.project_root())
            .unwrap()
            .is_some(),
        "the run was begun under the root the PREPARED value names",
    );
    assert!(
        vibe_lifecycle::LifecycleStateStore::peek(fixture.dir.path())
            .unwrap()
            .is_none(),
        "and not under the root a rediscovery would have found",
    );
}

/// The prepared value survives a selected manifest that no longer parses;
/// the compatibility wrapper does not.
///
/// Corruption after preparation is the shape a resume actually meets: the
/// caller read the tree, work happened, and the file underneath may be
/// anything by the time the slot run is rebuilt. The prepared constructor is
/// indifferent to it. The wrapper — same arguments, same fixture — refuses,
/// which is what makes the first half a difference rather than a coincidence.
#[test]
fn a_corrupt_selected_manifest_does_not_reach_the_prepared_constructor() {
    let fixture = Fixture::new();
    let workspace = fixture.prepared();
    std::fs::write(
        fixture.project_root().join("vibe.toml"),
        "[package\nbroken\n",
    )
    .unwrap();

    let _lifecycle = build_prepared(&fixture, &workspace)
        .expect("the prepared tree is authoritative, whatever the file says");

    let project_root = fixture.project_root();
    let refused = InstallSlotLifecycle::from_projection_observed(
        &project_root,
        &fixture.manifest,
        &fixture.resolution,
        &fixture.resolution,
        metadata(&project_root),
        StreamMode::Capture,
        SlotLifecycleSeams::refusing(),
        // The wrapper refuses at its own discovery against the corrupt
        // manifest before any lease agreement is judged, so the proof's
        // root is irrelevant here — it only has to be a real lease.
        Arc::new(LifecycleLease::acquire(&project_root).unwrap()),
    );
    assert!(
        refused.is_err(),
        "the wrapper really discovers, and the file really is unparseable",
    );
}
