//! Unit tests for the declared built-in schedule.

use specmark::verifies;

use super::*;
use crate::SpecAddress;
use crate::compiler::ir::{DocumentAddress, SourceFormatId};
use crate::compiler::pass::{IrPayload, PassSegmentError};
use crate::compiler::pipeline::{CompilerPipelineError, ScheduleItem};
use crate::compiler::transform::registry_test_support::{identity_plan, identity_registry};

fn source(format: &str, text: &str) -> SourceIr {
    SourceIr::reached(
        DocumentAddress::Spec(SpecAddress::parse("spec://org.demo/pkg/common/doc#root").unwrap()),
        SourceFormatId::new(format).unwrap(),
        text,
    )
}

fn source_at(anchor: &str, format: &str, text: &str) -> SourceIr {
    SourceIr::reached(
        DocumentAddress::Spec(
            SpecAddress::parse(&format!("spec://org.demo/pkg/common/doc#{anchor}")).unwrap(),
        ),
        SourceFormatId::new(format).unwrap(),
        text,
    )
}

fn seed() -> SpecAddress {
    SpecAddress::parse("spec://org.demo/pkg/common/doc#root").unwrap()
}

fn spec(raw: &str) -> SpecAddress {
    SpecAddress::parse(raw).unwrap()
}

fn plan(mode: StaticCompileMode) -> ArtifactPlan {
    ArtifactPlan::compatibility(seed(), mode)
}

/// One minimal StaticLane plan, so a nonempty transform plan can legally
/// reach schedule construction (compatibility frames refuse nonempty plans).
fn static_lane_plan() -> ArtifactPlan {
    use crate::compiler::ir::{
        ArtifactContext, ArtifactFrame, ArtifactId, ArtifactInput, ArtifactTarget,
    };
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
        vec![
            ArtifactInput::normal(
                "org.demo/alpha",
                "boot/alpha.md",
                spec("spec://org.demo/alpha/boot/entry#root"),
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn production_lane_declares_parse_gather_close_merge_embed_qualify_absorb_link_assemble() {
    let pipeline = BuiltinSchedule::assembled(
        &plan(StaticCompileMode::Plain),
        &TransformRegistry::builtins(),
    )
    .expect("the empty-plan schedule builds")
    .pipeline;
    let schedule = pipeline.schedule();

    assert!(matches!(
        schedule.as_slice(),
        [
            ScheduleItem::Pass(parse),
            ScheduleItem::GatherDocuments,
            ScheduleItem::Pass(close),
            ScheduleItem::Pass(merge),
            ScheduleItem::Pass(embed),
            ScheduleItem::Pass(qualify),
            ScheduleItem::Pass(absorb),
            ScheduleItem::Pass(link),
            ScheduleItem::Pass(assemble),
        ] if parse.name.as_str() == PARSE_PASS_NAME
            && parse.input == SourceIr::SHAPE
            && parse.output == DocumentIr::SHAPE
            && close.name.as_str() == CLOSE_PASS_NAME
            && close.input == super::super::ir::Documents::SHAPE
            && close.output == ClosureIr::SHAPE
            && merge.name.as_str() == MERGE_PASS_NAME
            && merge.input == ClosureIr::SHAPE
            && merge.output == ClosureIr::SHAPE
            && embed.name.as_str() == EMBED_PASS_NAME
            && embed.input == ClosureIr::SHAPE
            && embed.output == ClosureIr::SHAPE
            && qualify.name.as_str() == QUALIFY_PASS_NAME
            && qualify.input == ClosureIr::SHAPE
            && qualify.output == ClosureIr::SHAPE
            && absorb.name.as_str() == ABSORB_PASS_NAME
            && absorb.input == ClosureIr::SHAPE
            && absorb.output == ClosureIr::SHAPE
            && link.name.as_str() == LINK_PASS_NAME
            && link.input == ClosureIr::SHAPE
            && link.output == ClosureIr::SHAPE
            && assemble.name.as_str() == ASSEMBLE_PASS_NAME
            && assemble.input == ClosureIr::SHAPE
            && assemble.output == LaneIr::SHAPE
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-LEVELS")]
fn parse_runs_once_for_each_addressed_document_then_gathers() {
    let schedule = BuiltinSchedule::linked(
        &plan(StaticCompileMode::Plain),
        &TransformRegistry::builtins(),
    )
    .expect("the empty-plan schedule builds");
    // Two distinct canonical addresses: the gather boundary refuses a
    // repeated pinless key, and one document at two addresses is the
    // per-document cardinality this test proves.
    let documents = schedule
        .pipeline
        .run_documents(vec![
            source_at("one", MARKDOWN_FORMAT, "# One {#one}\n"),
            source_at("two", MARKDOWN_FORMAT, "# Two {#two}\n"),
        ])
        .unwrap();

    assert_eq!(documents.len(), 2);
    assert!(
        documents
            .iter()
            .any(|document| document.tree().find_by_anchor("one").is_some())
    );
    assert!(
        documents
            .iter()
            .any(|document| document.tree().find_by_anchor("two").is_some())
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn removing_parse_makes_the_production_schedule_unrunnable() {
    let error = CompilerPipeline::default()
        .run_documents(vec![source(MARKDOWN_FORMAT, "# Doc {#root}\n")])
        .unwrap_err();

    assert!(matches!(
        error,
        CompilerPipelineError::ScheduleBoundary {
            boundary: "document segment input",
            ..
        }
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn parse_failure_keeps_the_pass_name_and_concrete_source() {
    let error = BuiltinSchedule::linked(
        &plan(StaticCompileMode::Plain),
        &TransformRegistry::builtins(),
    )
    .expect("the empty-plan schedule builds")
    .pipeline
    .run_documents(vec![source("unsupported", "body")])
    .unwrap_err();
    let CompilerPipelineError::Segment(PassSegmentError::PassFailed { pass, source }) = error
    else {
        panic!("expected the parse pass failure")
    };

    assert_eq!(pass.as_str(), PARSE_PASS_NAME);
    let parse = source
        .downcast_ref::<ParseError>()
        .expect("the concrete parse error must survive manager attribution");
    assert_eq!(parse.format, "unsupported");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn removing_close_makes_the_gathered_schedule_unrunnable() {
    let mut pipeline = CompilerPipeline::default();
    pipeline.push_document(ParsePass::new()).unwrap();
    let documents = pipeline
        .run_documents(vec![source(MARKDOWN_FORMAT, "# Doc {#root}\n")])
        .unwrap();

    let error = pipeline.run_to_closure(documents).unwrap_err();
    assert!(matches!(
        error,
        CompilerPipelineError::ScheduleBoundary {
            boundary: "artifact segment input",
            ..
        }
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn the_empty_plan_keeps_the_declared_nine_item_schedule() {
    // The empty-plan half of the T4 carriage law (ABI §7.1) survives T6b
    // exactly: no registry injection, no transform pass, no header.
    let empty = BuiltinSchedule::assembled(
        &plan(StaticCompileMode::Plain),
        &TransformRegistry::builtins(),
    )
    .expect("the empty-plan schedule builds");

    let expected = [
        PARSE_PASS_NAME,
        CLOSE_PASS_NAME,
        MERGE_PASS_NAME,
        EMBED_PASS_NAME,
        QUALIFY_PASS_NAME,
        ABSORB_PASS_NAME,
        LINK_PASS_NAME,
        ASSEMBLE_PASS_NAME,
    ];
    let items = empty.pipeline_for_test().schedule();
    assert_eq!(
        items.len(),
        9,
        "eight passes plus the one gather slot: {items:?}"
    );
    let pass_names: Vec<&str> = items
        .iter()
        .filter_map(|item| match item {
            ScheduleItem::Pass(pass) => Some(pass.name.as_str()),
            ScheduleItem::GatherDocuments => None,
        })
        .collect();
    assert_eq!(pass_names, expected, "the historical schedule is exact");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_nonempty_plan_on_a_compatibility_frame_refuses_before_any_lookup() {
    // T6b retires T4's inert-nonempty claim (ABI §6.3): the same carriage
    // that used to schedule identically now refuses at construction — the
    // frame fault fires BEFORE any registry lookup, so even the catalog that
    // could resolve the entry never runs.
    let carried = plan(StaticCompileMode::Plain)
        .with_transforms(crate::compiler::transform::carriage::one_document_transform());
    let error = match BuiltinSchedule::assembled(&carried, &identity_registry()) {
        Ok(_) => panic!("a nonempty compatibility-fragment plan must refuse"),
        Err(error) => error,
    };
    assert!(
        matches!(
            error,
            ArtifactCompileError::Transform(ref transform)
                if matches!(
                    transform.inner(),
                    super::super::transform::fault::TransformError::CompatibilityFragmentPlan { entries: 1 }
                )
        ),
        "the plan-wide frame fault is typed: {error:?}"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_nonempty_identity_plan_adds_the_exact_document_position() {
    // The replacement for T4's nonempty half: with the one cfg-test identity
    // catalog injected, the SAME plan yields the exact historical schedule
    // plus the one document-position wrapper in its frozen slot (ABI §6.3 —
    // parity now holds because the behavior actually ran).
    let carried = static_lane_plan().with_transforms(identity_plan(&[(
        "org.demo/tools#doc",
        crate::compiler::transform::plan::TransformStage::Document,
    )]));
    let schedule = BuiltinSchedule::assembled(&carried, &identity_registry())
        .expect("the identity plan resolves and schedules");
    let items = schedule.pipeline_for_test().schedule();

    let pass_names: Vec<&str> = items
        .iter()
        .filter_map(|item| match item {
            ScheduleItem::Pass(pass) => Some(pass.name.as_str()),
            ScheduleItem::GatherDocuments => None,
        })
        .collect();
    assert_eq!(
        pass_names,
        [
            PARSE_PASS_NAME,
            "transform:document:org.demo/tools#doc",
            CLOSE_PASS_NAME,
            MERGE_PASS_NAME,
            EMBED_PASS_NAME,
            QUALIFY_PASS_NAME,
            ABSORB_PASS_NAME,
            LINK_PASS_NAME,
            ASSEMBLE_PASS_NAME,
        ],
        "the document wrapper sits after parse in the document segment",
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn opaque_retarget_forwards_the_whole_transform_plan() {
    // The test vehicle retarget helper must forward the COMPLETE plan, not
    // rebuild from contributions alone: silently dropping a nonempty carried
    // plan is the one carriage regression T4 must make red (ABI §7.1). T6b
    // keeps proving the carriage while EXECUTION of that forwarded plan on
    // its CompatibilityFragment frame refuses (see the frame test above).
    let base = plan(StaticCompileMode::Plain);
    let transforms = crate::compiler::transform::carriage::one_document_transform();
    let carried = base.clone().with_transforms(transforms.clone());

    let retargeted = super::driver::retarget_custom_for_test("opaque-test", carried).unwrap();
    assert_eq!(
        retargeted.transforms(),
        &transforms,
        "the retarget must retain the attached nonempty plan"
    );
    assert_ne!(
        retargeted.transforms(),
        base.transforms(),
        "a contributions-only rebuild would have pinned empty here"
    );
    assert_eq!(retargeted.contributions(), base.contributions());

    // The empty plan retarget stays empty: compatibility pins empty forever.
    let empty_retarget = super::driver::retarget_custom_for_test("opaque-test", base).unwrap();
    assert!(empty_retarget.transforms().is_empty());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn a_forwarded_custom_frame_with_a_resolvable_transform_still_refuses_before_anything() {
    // The full forwarded-frame law (ABI §6.3): retarget proves the plan
    // crossed intact, then EXECUTION on the custom CompatibilityFragment
    // frame refuses the typed plan-wide fault BEFORE the custom-backend
    // lookup (the builtin registry cannot even select `opaque-test`) and
    // before the first parse. Distinct from the ArtifactPlan::compatibility
    // path: this is a real StaticLane world whose plan was FORWARDED onto a
    // custom frame with a transform the registry could resolve.
    let carried = static_lane_plan().with_transforms(identity_plan(&[(
        "org.demo/tools#doc",
        crate::compiler::transform::plan::TransformStage::Document,
    )]));
    let retargeted = super::driver::retarget_custom_for_test("opaque-test", carried)
        .expect("the retarget forwards the whole plan");
    assert!(!retargeted.transforms().is_empty());
    reset_parse_invocations();

    let error = match BuiltinSchedule::emitted_for_test(
        &retargeted,
        &crate::compiler::transform::registry_test_support::identity_registry(),
    ) {
        Ok(_) => panic!("a forwarded nonempty compatibility-fragment plan must refuse"),
        Err(error) => error,
    };
    assert!(
        matches!(
            error,
            ArtifactCompileError::Transform(ref public)
                if matches!(
                    public.inner(),
                    crate::compiler::transform::fault::TransformError::CompatibilityFragmentPlan { entries: 1 }
                )
        ),
        "the frame fault wins over backend selection: {error:?}"
    );
    assert_eq!(
        parse_invocations(),
        0,
        "the refusal precedes the first parse"
    );
}
