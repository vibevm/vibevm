//! `BACKLOG.md` `B-117` at the wrapper: the separator half of the `paths`
//! contract, in its own cell.
//!
//! It is a different law from the selector VERDICTS beside it. A backslashed
//! declared path is not "out of scope" — it is malformed, and deciding a
//! malformed value against a glob would answer a question that was never
//! well posed. Its world is also its own: the only subject that can carry
//! such a path is one the compiler REACHED, because every boundary that
//! authors a contribution path already refuses a backslashed one.

use specmark::verifies;

use crate::compiler::builtin::{ArtifactCompileError, without_verify_each};

use super::fault::TransformError;
use super::plan::TransformStage;
use super::plan_test_support::build_or_panic;
use super::plan_validate::bounded;
use super::schedule_selector_tests::{
    AT_SOURCE, ArtifactResult, entry, expect_refusal, expected, paths, run, transform_fault,
};
use super::schedule_selector_vehicles::source_sightings;
use super::schedule_selector_worlds::use_world;
use super::selector_admission::SelectorAdmissionError;

/// `BACKLOG.md` `B-117` at the wrapper (§6.6): a backslashed declared path
/// meeting any selector refuses before any behavior runs.
///
/// The world is built rather than borrowed because the shared fixture cannot
/// express this: the artifact plan already refuses a backslashed CONTRIBUTION
/// path at its own boundary, so the only subject that can carry one is a
/// REACHED document, whose declared path is its address' own `doc_path` — and
/// `SpecAddress::parse` admits a backslash inside a path segment. The selector
/// names `boot/*`, which the declared root's own path (`roots/main.md`) does not
/// satisfy, so the root is skipped and the reached document is the first thing
/// any behavior could have seen: an empty sighting log therefore means the
/// refusal really did precede the behavior.
///
/// The test runs under production construction, and the first assertion says
/// why. With the TEST-ONLY inter-pass verifier armed, T7's own entry check
/// refuses this subject one layer earlier — a different law, in a different
/// family, and not one production runs. A `#[cfg(test)]` seam cannot be what
/// closes `B-117`, so the guarantee is asserted against the construction
/// production actually builds.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_backslashed_declared_path_refuses_at_the_wrapper_before_any_behavior_runs() {
    let armed = expect_refusal(
        compile_use_world("boot\\entry"),
        "armed, the inter-pass verifier refuses the subject first",
    );
    assert!(
        !matches!(armed, ArtifactCompileError::Transform(_)),
        "the armed verifier's refusal is a different family — so it cannot be what closes B-117"
    );

    without_verify_each(|| {
        let error = expect_refusal(
            compile_use_world("boot\\entry"),
            "a path that cannot obey the `paths` contract refuses",
        );
        let fault = transform_fault(&error);
        let TransformError::Selector {
            preview,
            order,
            stage,
            source,
        } = fault
        else {
            panic!("a malformed path is a stated contract violated, not a capability gap: {fault}")
        };
        assert!(
            matches!(
                source,
                SelectorAdmissionError::BackslashedDeclaredPath { .. }
            ),
            "the separator contract has its own typed arm: {source}"
        );
        assert_eq!(*order, 0, "the entry identity rides along");
        assert_eq!(*stage, TransformStage::Source);
        assert_eq!(*preview, bounded("org.demo/tools#src"));
        assert!(
            source_sightings().is_empty(),
            "no behavior ran before the refusal"
        );

        // Only the separator differed: the forward-slashed twin compiles under
        // the same construction, and the same selector then names the same
        // reached document.
        compile_use_world("boot/entry").expect("the forward-slashed twin compiles");
        assert_eq!(
            source_sightings(),
            expected(&[("spec://org.demo/back/boot/entry#root", "boot/entry")]),
            "the twin's reached document is in scope, so the red was the separator"
        );
    });
}

/// Compile the `#use` world whose reached document's `doc_path` is spelled
/// `used`, under one `boot/*`-scoped source entry.
fn compile_use_world(used: &str) -> ArtifactResult {
    let (plan, world) = use_world(used);
    let scoped = build_or_panic(vec![entry(&AT_SOURCE, paths(vec!["boot/*"]))]);
    run(plan.with_transforms(scoped), &world)
}
