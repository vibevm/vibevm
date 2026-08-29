//! Unit tests for the declared built-in schedule.

use specmark::verifies;

use super::*;
use crate::SpecAddress;
use crate::compiler::ir::{DocumentAddress, SourceFormatId};
use crate::compiler::pass::{IrPayload, PassSegmentError};
use crate::compiler::pipeline::{CompilerPipelineError, ScheduleItem};

fn source(format: &str, text: &str) -> SourceIr {
    SourceIr::new(
        DocumentAddress::Spec(SpecAddress::parse("spec://org.demo/pkg/common/doc#root").unwrap()),
        SourceFormatId::new(format).unwrap(),
        text,
    )
}

fn source_at(anchor: &str, format: &str, text: &str) -> SourceIr {
    SourceIr::new(
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

fn plan(mode: StaticCompileMode) -> ArtifactPlan {
    ArtifactPlan::compatibility(seed(), mode)
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn production_lane_declares_parse_gather_close_merge_embed_qualify_absorb_link_assemble() {
    let pipeline = BuiltinSchedule::assembled(&plan(StaticCompileMode::Plain)).pipeline;
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
    let schedule = BuiltinSchedule::linked(&plan(StaticCompileMode::Plain));
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
    let error = BuiltinSchedule::linked(&plan(StaticCompileMode::Plain))
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
fn empty_and_nonempty_carriage_keep_the_declared_nine_item_schedule() {
    // T4 carriage is inert (ABI §7.1): neither an empty nor an attached
    // nonempty transform plan adds any pass to the assembled built-in
    // schedule; execution begins with T5/T6, not here.
    let empty = BuiltinSchedule::assembled(&plan(StaticCompileMode::Plain));
    let carried_plan = plan(StaticCompileMode::Plain)
        .with_transforms(crate::compiler::transform::carriage::one_document_transform());
    let nonempty = BuiltinSchedule::assembled(&carried_plan);

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
    for schedule in [empty, nonempty] {
        let items = schedule.pipeline_for_test().schedule();
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
        assert!(
            pass_names
                .iter()
                .all(|name| !name.starts_with("transform:")),
            "no transform pass exists in T4: {pass_names:?}"
        );
    }
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#TRANSFORM-PLAN-IDENTITY")]
fn opaque_retarget_forwards_the_whole_transform_plan() {
    // The test vehicle retarget helper must forward the COMPLETE plan, not
    // rebuild from contributions alone: silently dropping a nonempty carried
    // plan is the one carriage regression T4 must make red (ABI §7.1).
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
