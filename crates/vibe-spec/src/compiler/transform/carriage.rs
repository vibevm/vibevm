//! Test-only nonempty plan construction for the T4 carriage tests
//! (`R4-TRANSFORM-PLAN-ABI-v0.1.md` §7.1). `builtin_candidate` stays
//! module-private inside [`super::plan`]; this is the one sanctioned
//! producer of a nonempty [`TransformPlan`] for the artifact-plan tests.

use super::plan::TransformPlan;
use super::plan::TransformStage;
use super::plan_test_support::{build_or_panic, dependency_seed};

/// One lawful nonempty plan: a single document-stage transform under the
/// default dependency provider. Enough to make carriage observable; never
/// enough to execute anything (execution is T5/T6, not T4).
pub(crate) fn one_document_transform() -> TransformPlan {
    build_or_panic(vec![dependency_seed(
        "org.demo/tools#carriage",
        TransformStage::Document,
    )])
}
