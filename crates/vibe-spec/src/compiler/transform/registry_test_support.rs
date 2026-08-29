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

use crate::compiler::ir::{DocumentIr, LaneIr, SourceIr};

use super::behavior::{TransformBehavior, TransformBehaviorError};
use super::plan::{TransformConfig, TransformStage};
use super::registry::TransformRegistry;

/// One identity vehicle per stage: it declares its name/epoch/stage and
/// returns its declared carrier unchanged; every other stage's method keeps
/// the trait's typed wrong-stage default.
macro_rules! identity_vehicle {
    ($type:ident, $name:literal, $variant:ident, $method:ident, $carrier:ty) => {
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
    SourceIr
);
identity_vehicle!(
    DocumentIdentity,
    "test-identity-document",
    Document,
    run_document,
    DocumentIr
);
identity_vehicle!(LaneIdentity, "test-identity-lane", Lane, run_lane, LaneIr);
identity_vehicle!(
    EmittedIdentity,
    "test-identity-emitted",
    Emitted,
    run_emitted,
    Vec<u8>
);

/// The one test registry carrying the four identity vehicles, epoch 1 each,
/// registered through the production registration path.
pub(crate) fn identity_registry() -> TransformRegistry {
    let mut registry = TransformRegistry::builtins();
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
