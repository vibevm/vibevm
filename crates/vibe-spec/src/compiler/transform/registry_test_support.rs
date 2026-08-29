//! The one reusable cfg-test identity catalog (R4-TRANSFORM-PLAN-ABI §6.1):
//! the four `test-identity-*` behavior vehicles and the single builder of
//! the test registry that carries them.
//!
//! T5's own tests consume this module, and T6's four-position tests consume
//! the SAME registry — one test authority, never a copied catalog per test
//! cell. The vehicles never enter production code or the public crate
//! surface: the module is `#[cfg(test)] pub(crate)` in `transform/mod.rs`,
//! so nothing outside a test build can name it.

use std::sync::Arc;

use vibe_core::manifest::ExtensionKey;

use crate::compiler::ir::{DocumentIr, LaneIr, SourceIr};

use super::behavior::{TransformBehavior, TransformBehaviorError};
use super::plan::{
    TransformConfig, TransformImplementation, TransformPlan, TransformProvider, TransformSeed,
    TransformStage,
};
use super::registry::TransformRegistry;

// Per-thread invocation counters for the four shared identity vehicles:
// commissioning tests assert the BEHAVIOR methods ran (5/5/1/1), never a
// proxy counter. Thread-local because the suite runs tests in parallel.
std::thread_local! {
    static IDENTITY_SOURCE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static IDENTITY_DOCUMENT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static IDENTITY_LANE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static IDENTITY_EMITTED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Reset all four identity invocation counters.
pub(crate) fn reset_identity_invocations() {
    IDENTITY_SOURCE.with(|count| count.set(0));
    IDENTITY_DOCUMENT.with(|count| count.set(0));
    IDENTITY_LANE.with(|count| count.set(0));
    IDENTITY_EMITTED.with(|count| count.set(0));
}

/// The four identity invocation counts `(source, document, lane, emitted)`.
pub(crate) fn identity_invocations() -> (usize, usize, usize, usize) {
    (
        IDENTITY_SOURCE.with(std::cell::Cell::get),
        IDENTITY_DOCUMENT.with(std::cell::Cell::get),
        IDENTITY_LANE.with(std::cell::Cell::get),
        IDENTITY_EMITTED.with(std::cell::Cell::get),
    )
}

/// One identity vehicle per stage: it declares its name/epoch/stage, counts
/// its own invocation, and returns its declared carrier unchanged; every
/// other stage's method keeps the trait's typed wrong-stage default. The
/// names/epochs/stages and the catalog golden are exactly T5's.
macro_rules! identity_vehicle {
    ($type:ident, $name:literal, $variant:ident, $method:ident, $carrier:ty, $count:path) => {
        struct $type;

        impl TransformBehavior for $type {
            fn name(&self) -> &str {
                $name
            }
            fn epoch(&self) -> u32 {
                1
            }
            fn stage(&self) -> TransformStage {
                TransformStage::$variant
            }
            fn $method(
                &self,
                _config: Option<&TransformConfig>,
                input: $carrier,
            ) -> Result<$carrier, TransformBehaviorError> {
                $count.with(|count| count.set(count.get() + 1));
                Ok(input)
            }
        }
    };
}

identity_vehicle!(
    SourceIdentity,
    "test-identity-source",
    Source,
    run_source,
    SourceIr,
    IDENTITY_SOURCE
);
identity_vehicle!(
    DocumentIdentity,
    "test-identity-document",
    Document,
    run_document,
    DocumentIr,
    IDENTITY_DOCUMENT
);
identity_vehicle!(
    LaneIdentity,
    "test-identity-lane",
    Lane,
    run_lane,
    LaneIr,
    IDENTITY_LANE
);
identity_vehicle!(
    EmittedIdentity,
    "test-identity-emitted",
    Emitted,
    run_emitted,
    Vec<u8>,
    IDENTITY_EMITTED
);

/// The one test registry carrying the four identity vehicles, epoch 1 each,
/// registered through the production registration path.
///
/// It starts EMPTY rather than from `TransformRegistry::builtins()`. Since
/// R4.2 the production catalog really ships a behavior, and a test catalog
/// that silently inherited it would make "an off-catalog name refuses"
/// untestable for exactly the names that matter most — the shipping ones. A
/// test that wants the production catalog asks for it by name.
pub(crate) fn identity_registry() -> TransformRegistry {
    let mut registry = TransformRegistry::default();
    for (behavior, _, _) in identity_vehicles() {
        registry.register(behavior).expect("test vehicle registers");
    }
    registry
}

/// The four registered identity behaviors with their exact names and stages,
/// for tests that drive the behaviors directly (vectors, wrong-stage REDs,
/// pointer identity).
pub(crate) fn identity_vehicles() -> Vec<(Arc<dyn TransformBehavior>, &'static str, TransformStage)>
{
    vec![
        (
            Arc::new(SourceIdentity),
            "test-identity-source",
            TransformStage::Source,
        ),
        (
            Arc::new(DocumentIdentity),
            "test-identity-document",
            TransformStage::Document,
        ),
        (
            Arc::new(LaneIdentity),
            "test-identity-lane",
            TransformStage::Lane,
        ),
        (
            Arc::new(EmittedIdentity),
            "test-identity-emitted",
            TransformStage::Emitted,
        ),
    ]
}

/// The exact identity-catalog name one stage resolves to.
fn identity_name(stage: &TransformStage) -> &'static str {
    match stage {
        TransformStage::Source => "test-identity-source",
        TransformStage::Document => "test-identity-document",
        TransformStage::Lane => "test-identity-lane",
        TransformStage::Emitted => "test-identity-emitted",
    }
}

/// One seed whose implementation resolves in [`identity_registry`] at the
/// given stage: the minimal T6b execution fixture. Config and selector stay
/// absent; callers wrap [`TransformSeed::new`] directly for those cases.
pub(crate) fn identity_seed(key: &str, stage: TransformStage) -> TransformSeed {
    TransformSeed::new(
        ExtensionKey::authored(key),
        TransformProvider::from(&super::plan_test_support::default_dependency()),
        stage.clone(),
        TransformImplementation::builtin_candidate(identity_name(&stage), 1),
        None,
        None,
    )
}

/// One lawful plan of identity-catalog entries, ready to attach with
/// [`ArtifactPlan::with_transforms`].
pub(crate) fn identity_plan(entries: &[(&str, TransformStage)]) -> TransformPlan {
    let seeds = entries
        .iter()
        .map(|(key, stage)| identity_seed(key, stage.clone()))
        .collect();
    TransformPlan::build(seeds).expect("the identity plan builds")
}
