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
