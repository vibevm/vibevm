//! T6b schedule-construction tests (ABI §6.3): the frozen refusal
//! precedence, whole-plan resolution before any parse, the exact four
//! positions with stable within-stage order, and the exact pass-name
//! spelling the schedule owns.

use specmark::verifies;

use crate::SpecAddress;
use crate::compiler::builtin::{BuiltinSchedule, reset_parse_invocations};
use crate::compiler::ir::{
    ArtifactContext, ArtifactFrame, ArtifactId, ArtifactInput, ArtifactPlan, ArtifactTarget,
    StaticCompileMode,
};
use crate::compiler::pipeline::ScheduleItem;

use super::fault::TransformError;
use super::plan::{TransformImplementation, TransformSeed, TransformStage};
use super::plan_test_support::{SelectorShape, compiled_selector, dependency_seed};
use super::registry::TransformRegistry;
use super::registry_test_support::{identity_plan, identity_registry, identity_seed};
use crate::compiler::builtin::ArtifactCompileError;

const ALPHA: &str = "spec://org.demo/alpha/boot/entry#root";

/// One minimal StaticLane plan: the only frame a nonempty transform plan
/// may legally execute on.
fn lane_plan() -> ArtifactPlan {
    let context = ArtifactContext::new(
        ArtifactId::new("static-xml").unwrap(),
        ArtifactTarget::StaticXml,
        ArtifactFrame::StaticLane {
            generated_path: "vibevm/vibespecs/boot/STATIC.xml".to_string(),
            source_root: "vibevm/vibedeps".to_string(),
        },
        StaticCompileMode::QualifyPerNode,
    )
    .unwrap();
    ArtifactPlan::new(
        context,
        vec![ArtifactInput::normal("org.demo/alpha", "boot/alpha.md", spec(ALPHA)).unwrap()],
    )
    .unwrap()
}

fn spec(raw: &str) -> SpecAddress {
    SpecAddress::parse(raw).unwrap()
}

/// One seed with full control over its implementation, config and selector.
fn reseed(
    key: &str,
    stage: TransformStage,
    implementation: TransformImplementation,
    selector: Option<vibe_extension_registry::CompiledSelector>,
) -> TransformSeed {
    let base = dependency_seed(key, stage.clone());
    TransformSeed::new(
        base.key().clone(),
        base.provider().clone(),
        stage,
        implementation,
        None,
        selector,
    )
}

use super::plan_test_support::build_or_panic;

fn plan_of(seeds: Vec<TransformSeed>) -> ArtifactPlan {
    lane_plan().with_transforms(build_or_panic(seeds))
}

fn identity_impl(stage: &TransformStage) -> TransformImplementation {
    let name = match stage {
        TransformStage::Source => "test-identity-source",
        TransformStage::Document => "test-identity-document",
        TransformStage::Lane => "test-identity-lane",
        TransformStage::Emitted => "test-identity-emitted",
    };
    TransformImplementation::builtin_candidate(name, 1)
}

/// `BuiltinSchedule` is not `Debug`; refuse by match, never by `unwrap_err`.
fn expect_refusal(result: Result<BuiltinSchedule, ArtifactCompileError>) -> ArtifactCompileError {
    match result {
        Ok(_) => panic!("the schedule must refuse"),
        Err(error) => error,
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn the_whole_plan_resolves_before_the_first_parse() {
    reset_parse_invocations();
    let plan = plan_of(vec![
        identity_seed("org.demo/tools#doc", TransformStage::Document),
        reseed(
            "org.demo/tools#later",
            TransformStage::Document,
            TransformImplementation::builtin_candidate("xml-minify", 1),
            None,
        ),
    ]);
    let error = expect_refusal(BuiltinSchedule::emitted_for_test(
        &plan,
        &identity_registry(),
    ));
    assert!(matches!(error, ArtifactCompileError::Transform(_)));
    assert_eq!(
        crate::compiler::builtin::parse_invocations(),
        0,
        "resolution precedes the first parse"
    );
    // Positive control: the identity entry alone builds and runs discovery.
    let good = plan_of(vec![identity_seed(
        "org.demo/tools#doc",
        TransformStage::Document,
    )]);
    assert!(BuiltinSchedule::emitted_for_test(&good, &identity_registry()).is_ok());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn the_first_fault_in_authored_plan_order_wins() {
    // Entry 0 carries an epoch mismatch; entry 1 an unknown name. Plan order,
    // not byte-sorted key order (`later` < `doc` would flip them), names the
    // fault: the preview names entry 0's key and order.
    let plan = plan_of(vec![
        reseed(
            "org.demo/tools#zzz",
            TransformStage::Document,
            TransformImplementation::builtin_candidate("test-identity-document", 2),
            None,
        ),
        reseed(
            "org.demo/tools#aaa",
            TransformStage::Document,
            TransformImplementation::builtin_candidate("no-such-builtin", 1),
            None,
        ),
    ]);
    let error = expect_refusal(BuiltinSchedule::emitted_for_test(
        &plan,
        &identity_registry(),
    ));
    let ArtifactCompileError::Transform(public) = &error else {
        panic!("the refusal is the transform family: {error:?}")
    };
    // Exact variant matching: entry 0's epoch mismatch wins over entry 1's
    // unknown name — plan order, not byte-sorted key order.
    assert!(
        matches!(
            public.inner(),
            TransformError::Resolution {
                order: 0,
                source: super::registry::TransformRegistryError::EpochMismatch {
                    requested: 2,
                    catalog: 1,
                    ..
                },
                ..
            }
        ),
        "plan order wins: {public}"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn registry_precedence_name_epoch_stage_stays_exact_through_the_schedule() {
    // Unknown name beats wrong epoch and wrong stage in ONE entry; a known
    // name with a wrong epoch beats a wrong stage; the matching triple
    // resolves.
    // One entry whose name is off-catalog while its epoch and declared
    // stage are also wrong: the name lookup refuses first.
    let unknown_all = reseed(
        "org.demo/tools#u",
        TransformStage::Document,
        TransformImplementation::builtin_candidate("log", 2),
        None,
    );
    let plan = plan_of(vec![unknown_all]);
    let error = expect_refusal(BuiltinSchedule::emitted_for_test(
        &plan,
        &identity_registry(),
    ));
    let ArtifactCompileError::Transform(public) = &error else {
        panic!("transform family: {error:?}")
    };
    assert!(
        matches!(
            public.inner(),
            TransformError::Resolution {
                source: super::registry::TransformRegistryError::UnknownBuiltin { .. },
                ..
            }
        ),
        "the name lookup refuses first: {public}"
    );

    let epoch_first = reseed(
        "org.demo/tools#e",
        TransformStage::Document,
        TransformImplementation::builtin_candidate("test-identity-document", 7),
        None,
    );
    let plan = plan_of(vec![epoch_first]);
    let error = expect_refusal(BuiltinSchedule::emitted_for_test(
        &plan,
        &identity_registry(),
    ));
    let ArtifactCompileError::Transform(public) = &error else {
        panic!("transform family: {error:?}")
    };
    assert!(
        matches!(
            public.inner(),
            TransformError::Resolution {
                source: super::registry::TransformRegistryError::EpochMismatch {
                    requested: 7,
                    catalog: 1,
                    ..
                },
                ..
            }
        ),
        "the epoch mismatch names both epochs: {public}"
    );

    let stage_mismatch = reseed(
        "org.demo/tools#s",
        TransformStage::Document,
        identity_impl(&TransformStage::Lane),
        None,
    );
    let plan = plan_of(vec![stage_mismatch]);
    let error = expect_refusal(BuiltinSchedule::emitted_for_test(
        &plan,
        &identity_registry(),
    ));
    let ArtifactCompileError::Transform(public) = &error else {
        panic!("transform family: {error:?}")
    };
    assert!(
        matches!(
            public.inner(),
            TransformError::Resolution {
                source: super::registry::TransformRegistryError::StageMismatch { .. },
                ..
            }
        ),
        "the stage mismatch names both stages: {public}"
    );
}

/// Canonical selector PRESENCE still splits at build — and T8 stopped it being
/// a construction refusal.
///
/// The T6b law this replaces refused every selector-bearing source/document
/// entry here, because judging one needed a document subject and no document
/// exists while a plan is being resolved. T7 landed the subject and T8 moved the
/// verdict to the wrappers, where a document does exist, so all three shapes now
/// construct. What still splits at build is canonicalization alone: a real
/// dimension and a present-EMPTY one stay present, a both-absent authored
/// selector becomes outer absence. The match-time verdict each presence then
/// earns — including the one surviving refusal — is asserted live in
/// `schedule_selector_tests`.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn canonical_selector_presence_splits_at_build_and_no_longer_refuses_construction() {
    for (key, packages, retained, label) in [
        (
            "org.demo/tools#sel",
            Some(vec!["org.demo/*"]),
            true,
            "real dimension",
        ),
        (
            "org.demo/tools#empty",
            Some(vec![]),
            true,
            "present-empty dimension",
        ),
        (
            "org.demo/tools#none",
            None,
            false,
            "both-absent authored selector",
        ),
    ] {
        reset_parse_invocations();
        let plan = plan_of(vec![reseed(
            key,
            TransformStage::Source,
            identity_impl(&TransformStage::Source),
            Some(compiled_selector(SelectorShape::Dimensions {
                packages,
                paths: None,
            })),
        )]);
        assert_eq!(
            plan.transforms().entries()[0].seed().selector().is_some(),
            retained,
            "{label}: build canonicalization alone decides presence"
        );
        assert!(
            BuiltinSchedule::linked_for_test(&plan, &identity_registry()).is_ok(),
            "{label}: a selector is no longer a construction refusal"
        );
        assert_eq!(
            crate::compiler::builtin::parse_invocations(),
            0,
            "{label}: construction still touches no document"
        );
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn interleaved_plan_order_partitions_stably_with_exact_names() {
    // Authored (unsorted) keys across all four stages, interleaved in plan
    // order: each stage keeps its own authored relative order, and the exact
    // pass-name spelling — prefix, stage token, full ExtensionKey with `/`
    // and `#` — is the schedule identity.
    let plan = plan_of(vec![
        identity_seed("org.demo/zeta#d1", TransformStage::Document),
        identity_seed("org.demo/beta#l1", TransformStage::Lane),
        identity_seed("org.demo/alpha#d2", TransformStage::Document),
        identity_seed("org.demo/omega#e1", TransformStage::Emitted),
        identity_seed("org.demo/yankee#s1", TransformStage::Source),
        identity_seed("org.demo/mid#d3", TransformStage::Document),
    ]);
    let schedule = BuiltinSchedule::emitted_for_test(&plan, &identity_registry())
        .expect("the interleaved identity plan resolves");
    let items = schedule.pipeline_for_test().schedule();
    let names: Vec<&str> = items
        .iter()
        .filter_map(|item| match item {
            ScheduleItem::Pass(pass) => Some(pass.name.as_str()),
            ScheduleItem::GatherDocuments => None,
        })
        .collect();
    assert_eq!(
        names,
        [
            "transform:source:org.demo/yankee#s1",
            "parse",
            "transform:document:org.demo/zeta#d1",
            "transform:document:org.demo/alpha#d2",
            "transform:document:org.demo/mid#d3",
            "close",
            "merge",
            "embed",
            "qualify",
            "absorb",
            "link",
            "assemble",
            "transform:lane:org.demo/beta#l1",
            "emit:static-xml",
            "transform:emitted:org.demo/omega#e1",
        ],
        "stable partition, authored within-stage order, exact spellings"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn the_empty_production_registry_refuses_a_nonempty_plan_at_the_schedule() {
    reset_parse_invocations();
    let plan = lane_plan().with_transforms(identity_plan(&[(
        "org.demo/tools#doc",
        TransformStage::Document,
    )]));
    let error = expect_refusal(BuiltinSchedule::emitted_for_test(
        &plan,
        &TransformRegistry::builtins(),
    ));
    assert!(matches!(error, ArtifactCompileError::Transform(_)));
    assert_eq!(crate::compiler::builtin::parse_invocations(), 0);
    // The empty plan under the empty production registry still builds: the
    // historical schedule is reachable in production.
    assert!(
        BuiltinSchedule::emitted_for_test(&lane_plan(), &TransformRegistry::builtins()).is_ok()
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_global_pass_name_collision_is_an_entry_fault_with_full_row_identity() {
    // The pipeline's global duplicate-name backstop, exercised at the
    // transform seam: preinserting a pass under the EXACT planned transform
    // name makes the source push refuse with the row's bounded preview,
    // dense order and exact stage riding along the typed DuplicateName.
    use crate::compiler::ir::SourceIr;
    use crate::compiler::pass::IdentityPass;
    use crate::compiler::pass::PassName;
    use crate::compiler::pipeline::CompilerPipeline;

    let plan = plan_of(vec![
        identity_seed("org.demo/tools#src", TransformStage::Source),
        identity_seed("org.demo/tools#later", TransformStage::Source),
    ]);
    let schedule = super::schedule::TransformSchedule::resolve(&plan, &identity_registry())
        .expect("the identity plan resolves");

    let mut pipeline = CompilerPipeline::default();
    pipeline
        .push_document(IdentityPass::<SourceIr>::new(
            PassName::new("transform:source:org.demo/tools#src")
                .expect("the colliding fixture name is non-blank"),
        ))
        .expect("the colliding fixture pass enters the document segment");

    let error = schedule
        .push_source_before_parse(&mut pipeline)
        .expect_err("the global name set refuses the duplicate");
    assert!(
        matches!(
            &error,
            TransformError::Schedule {
                order: 0,
                stage,
                source: crate::compiler::pipeline::CompilerPipelineError::DuplicateName { pass },
                ..
            } if *stage == TransformStage::Source
                && pass.as_str() == "transform:source:org.demo/tools#src"
        ),
        "the insertion fault carries the row identity and the typed source: {error:?}"
    );
    // The second row never enters after the first refused: partial pushes
    // stop at the faulting entry.
    let error = schedule
        .push_source_before_parse(&mut pipeline)
        .expect_err("the same collision refuses again at the same row");
    assert!(matches!(&error, TransformError::Schedule { order: 0, .. }));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_blank_rendered_pass_name_refuses_typed_not_by_panic() {
    // PassName construction refuses through the typed Name fault: a seed key
    // that renders blank after the mandated prefix still builds a legal PLAN
    // (the key grammar is checked at build), so this path is belt-only —
    // exercised by constructing the row's fault projection directly.
    let row_preview = super::plan_validate::bounded("org.demo/tools#blank");
    let fault = TransformError::Name {
        preview: row_preview,
        order: 3,
        stage: TransformStage::Source,
        source: crate::compiler::pass::PassNameError,
    };
    let public = super::fault::TransformCompileError::new(fault);
    assert!(
        public.to_string().contains("has no valid pass name"),
        "the typed name fault renders: {public}"
    );
    assert!(
        matches!(
            public.inner(),
            TransformError::Name {
                order: 3,
                stage: TransformStage::Source,
                ..
            }
        ),
        "the exact variant carries entry identity: {public:?}"
    );
}
