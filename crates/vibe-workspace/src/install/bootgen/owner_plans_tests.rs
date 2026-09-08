//! Structural pins for the sole owner-runtime lowering authority.

use specmark::verifies;

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#COMPILE-ACTIVATION")]
fn boot_has_no_independent_plan_collector_or_lowerer() {
    let adapter = include_str!("owner_plans.rs");
    let runtime = include_str!("../../extension_world/runtime.rs");
    for forbidden in [
        "lower_owner_view",
        "collect_owner_view",
        "unit_owner_plan",
        "node_owner_plan",
        "unit_owner_plans",
    ] {
        assert!(
            !adapter.contains(forbidden),
            "`{forbidden}` would restore a second lowering path"
        );
    }
    assert!(!adapter.contains("TransformPlan::from_effective_rows"));
    assert!(!runtime.contains("TransformPlan::from_effective_rows"));
    for authority in [
        "collect_owner_mechanisms(&view)",
        "collect_owner_view(view, presets)",
        "lower_effective_compile_rows(&compile_rows)",
    ] {
        assert_eq!(
            runtime.matches(authority).count(),
            1,
            "`{authority}` must appear exactly once in the sole runtime lowerer"
        );
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn generation_fingerprints_and_emission_read_retained_runtimes() {
    let composition = include_str!("../bootgen.rs");
    let emission = include_str!("hybrid_emit.rs");
    let attachment = include_str!("../../boot_artifacts.rs");
    assert!(composition.contains("lower_owner_runtimes(workspace, &world, lowering)"));
    assert!(composition.contains("plan_digest_frames(&runtimes)"));
    assert!(composition.contains("runtimes.node(rel)?.compile_plans().clone()"));
    assert!(emission.contains("runtimes.unit(&owner)?.compile_plans().clone()"));
    assert!(attachment.contains("let plan = plans.attach_to("));
    assert!(attachment.contains("let compiled = compile_static_artifact_with_plans("));
    assert!(attachment.contains("CompilePlans::transforms_only(TransformPlan::empty())"));
    assert!(!composition.contains("transform_plan().clone()"));
    assert!(!emission.contains("transform_plan().clone()"));
    assert!(!composition.contains("unit_owner_plans"));
}
