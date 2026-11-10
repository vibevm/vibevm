//! The extension-world fixture P7 left open, and the reason it was open.
//!
//! P7's second deviation read: *"No extension-world fixture in the
//! integration suite. The owner-managed and pending-native seams have their
//! own prompt tests at the unit level, where a provider can be supplied; an
//! integration fixture would have to build one and would be testing the
//! provider."*
//!
//! **That reason is answered rather than worked around.** The fixture here
//! supplies a provider that **refuses to bind**, and asserts it was never
//! asked — so nothing about any provider's behaviour is under test. What is
//! under test is the seam. `#SYS-LANE-VERBATIM` says a whole regeneration
//! is driven through **one** of the two publishing seams, chosen once per
//! run and never mixed: `install/bootgen.rs` drives every node through the
//! builtin loop, or `regenerate_boot_from_traced_native_prepared` drives
//! every node through the owner-managed one. The prefix "reuses that output
//! as one admitted segment through whichever of the two publishing seams the
//! node's own world selected — it does not add a third publishing seam, and
//! it does not assume one."
//!
//! Until now that was a claim about two code paths with no test comparing
//! them end to end. It is now a measurement: the same fixture inputs,
//! regenerated through the owner-managed seam, publish `SYSTEM.md`,
//! `SYSTEM.meta.json` and `AGENT-LANE.md` **byte for byte** as the builtin
//! seam does — the one artifact set a caller consumes, compared at the one
//! granularity that admits no interpretation.
//!
//! **Why no native extension is declared in the fixture.** A node whose
//! world holds no owner runtime still goes through the owner-managed seam:
//! that seam is selected for the *run*, not per node. So this fixture is
//! exactly the honest shape of the open question — "does an extension-world
//! run publish the same prefix?" — with the provider out of the answer.
//! Declaring a native handler would put a `cargo` build and a reply codec
//! between the inputs and the artifacts, which is the test the deviation
//! correctly refused to write.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use vibe_core::layout;
use vibe_spec::{
    CompilerNativeCall, CompilerNativeInvoker, CompilerNativeInvokerError,
    CompilerNativeInvokerErrorKind, CompilerNativePolicy, CompilerPendingSet,
};
use vibe_workspace::Workspace;
use vibe_workspace::WorkspaceError;
use vibe_workspace::extension_world::{
    CompilerNativeFactBinding, CompilerNativeFactError, ExtensionWorldEpoch,
    OwnerNativeCompileBinding, OwnerNativeCompileProvider, OwnerRuntimeId, OwnerRuntimeLowering,
    OwnerRuntimeRunFacts, OwnerRuntimeView, PendingBuildFact,
};
use vibe_workspace::install::regenerate_boot_from_traced_native;

use super::fixtures::{Fixture, Shape, build};

/// The three published artifacts, as a consumer reads them.
const PUBLISHED: [&str; 3] = [
    layout::SYSTEM_MD,
    layout::SYSTEM_META_JSON,
    layout::AGENT_LANE_MD,
];

/// A provider that refuses to bind, and records whether it was asked.
///
/// It is the whole point of this fixture. The seam does hand a provider the
/// **policy map** it keeps for every node and unit — that is the manager's
/// own bookkeeping, computed before anything is compiled — but with no
/// native handler declared it never *binds* one, and an unbound provider
/// cannot have contributed a byte. A provider that answered would make the
/// test a test of the answer.
struct NeverBound {
    /// Set if `bind` is ever reached, which would invalidate the claim.
    bound: Arc<AtomicBool>,
}

/// The binding type the provider's associated type needs and no run reaches.
struct Unreachable;

impl CompilerNativeInvoker for Unreachable {
    fn invoke(&self, _call: CompilerNativeCall<'_>) -> Result<Vec<u8>, CompilerNativeInvokerError> {
        Err(CompilerNativeInvokerError::new(
            CompilerNativeInvokerErrorKind::InvocationFailed,
            "this fixture's provider is never bound",
        ))
    }
}

impl CompilerNativeFactBinding for Unreachable {
    fn invoker(&self) -> &dyn CompilerNativeInvoker {
        self
    }

    fn take_pending_build_facts(
        &self,
        _pending: &CompilerPendingSet,
    ) -> Result<Vec<PendingBuildFact>, CompilerNativeFactError> {
        Err(CompilerNativeFactError::already_taken())
    }

    fn finish_ready(&self) -> Result<(), CompilerNativeFactError> {
        Err(CompilerNativeFactError::already_taken())
    }
}

impl OwnerNativeCompileProvider for NeverBound {
    type Binding<'owner> = Unreachable;

    fn bind<'owner>(
        &mut self,
        _owner: OwnerRuntimeView<'owner>,
    ) -> Result<OwnerNativeCompileBinding<Self::Binding<'owner>>, WorkspaceError> {
        self.bound.store(true, Ordering::SeqCst);
        Err(WorkspaceError::NativeCompileProvider {
            owner: "<fixture>".to_owned(),
            reason: "this fixture's world holds no owner runtime, so nothing may be bound"
                .to_owned(),
        })
    }
}

/// Regenerate `fixture` once more, through the **owner-managed** seam.
///
/// Returns the policies the seam handed the provider factory, having first
/// asserted that no provider was ever **bound** — which is the claim that
/// keeps every provider's behaviour out of the comparison.
fn regenerate_through_the_extension_world(
    fixture: &Fixture,
) -> BTreeMap<OwnerRuntimeId, CompilerNativePolicy> {
    let workspace = Workspace::load(fixture.ws.path()).expect("the fixture workspace loads");
    let world = ExtensionWorldEpoch::from_resolution(&workspace.root, &fixture.resolution)
        .expect("the fixture's extension world");
    let facts = OwnerRuntimeRunFacts {
        run_id: "system-prompt-extension-world".to_owned(),
        state_root: workspace.root.join(".vibe"),
        platform: "linux-x86_64".to_owned(),
        offline: true,
        // A fixed instant, because a generated artifact carries no
        // timestamp and a run fact must not be the thing that makes two
        // otherwise equal prefixes differ (`#SYS-NO-VOLATILE`).
        created_at: "2026-11-10T00:00:00Z".to_owned(),
    };
    let bound = Arc::new(AtomicBool::new(false));
    let mut asked: BTreeMap<OwnerRuntimeId, CompilerNativePolicy> = BTreeMap::new();
    let mut make_provider = |policies: BTreeMap<OwnerRuntimeId, CompilerNativePolicy>| {
        asked = policies;
        Ok(NeverBound {
            bound: Arc::clone(&bound),
        })
    };
    let (nodes, _carriage) = regenerate_boot_from_traced_native(
        &workspace,
        &fixture.resolution,
        world,
        fixture.format,
        None,
        OwnerRuntimeLowering::compatibility_root_without_presets(),
        facts,
        &mut make_provider,
    )
    .expect("the owner-managed seam regenerates this fixture");
    assert_eq!(nodes, ["."], "the fixture has one node");
    assert!(
        !bound.load(Ordering::SeqCst),
        "the seam bound the provider, so a provider is in the answer"
    );
    asked
}

#[test]
fn the_owner_managed_seam_publishes_the_same_three_artifacts_byte_for_byte() {
    let fixture = build(Shape::standard());
    let builtin: Vec<Vec<u8>> = PUBLISHED
        .iter()
        .map(|name| fixture.artifact(name))
        .collect();
    for (name, bytes) in PUBLISHED.iter().zip(&builtin) {
        assert!(!bytes.is_empty(), "the builtin seam published no `{name}`");
    }

    // The policy map is the manager's bookkeeping over the world's nodes
    // and units, not an invocation: it is non-empty here and no provider is
    // bound, which is exactly the shape of a run whose extension world
    // declares no native handler.
    let asked = regenerate_through_the_extension_world(&fixture);
    assert!(!asked.is_empty(), "the owner-managed seam kept no policies");
    for (owner, policy) in &asked {
        assert!(
            !format!("{policy:?}").contains("Resolve"),
            "`{owner:?}` was handed a resolving policy, which would invoke a compiler"
        );
    }

    for (name, before) in PUBLISHED.iter().zip(&builtin) {
        assert_eq!(
            &fixture.artifact(name),
            before,
            "`{name}` differs between the two publishing seams"
        );
    }
}

#[test]
fn the_extension_world_run_keeps_the_sidecar_and_the_lane_coherent() {
    // Byte-equality above already implies this, and it is asserted
    // separately because the two artifacts are consumed by two different
    // readers and a seam that published one of them from a different
    // observation would be a seam that published an incoherent pair.
    let fixture = build(Shape::standard());
    regenerate_through_the_extension_world(&fixture);

    let prefix = fixture.artifact(layout::SYSTEM_MD);
    let sidecar = fixture.sidecar();
    assert_eq!(
        sidecar["prefix_len"].as_u64().expect("a prefix length"),
        prefix.len() as u64
    );
    let lane =
        String::from_utf8(fixture.artifact(layout::AGENT_LANE_MD)).expect("the lane is text");
    vibe_workspace::system_prompt::verify_lane_markers(lane.as_bytes())
        .expect("the owner-managed seam's lane verifies");
    // The lane never carries the prefix's own segment frames: agent mode is
    // the lane alone, not `SYSTEM.md` minus a segment (`#SYS-AGENT-LANE`).
    assert!(
        !lane.contains("vibe:prompt-segment"),
        "a prefix frame reached the agent lane"
    );
}

#[test]
fn a_second_extension_world_run_rewrites_nothing() {
    // `#SYS-NO-EQUAL-REWRITE` through the other seam: the determinism claim
    // is about the artifacts, so it has to hold for whichever seam wrote
    // them.
    let fixture = build(Shape::standard());
    regenerate_through_the_extension_world(&fixture);
    let first: Vec<Vec<u8>> = PUBLISHED
        .iter()
        .map(|name| fixture.artifact(name))
        .collect();
    let modified: Vec<std::time::SystemTime> = PUBLISHED
        .iter()
        .map(|name| {
            std::fs::metadata(fixture.boot(name))
                .expect("a published artifact")
                .modified()
                .expect("a modification time")
        })
        .collect();

    regenerate_through_the_extension_world(&fixture);
    for ((name, before), was) in PUBLISHED.iter().zip(&first).zip(&modified) {
        assert_eq!(&fixture.artifact(name), before, "`{name}` changed");
        assert_eq!(
            &std::fs::metadata(fixture.boot(name))
                .expect("a published artifact")
                .modified()
                .expect("a modification time"),
            was,
            "`{name}` was rewritten with the same bytes"
        );
    }
}
