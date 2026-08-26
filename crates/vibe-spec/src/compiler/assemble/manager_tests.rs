use std::convert::Infallible;

use specmark::verifies;

use super::*;
use crate::compiler::ir::{ClosureIr, Documents, LaneIr, LinkState};
use crate::compiler::pass::{
    AnyIr, IdentityPass, IrPayload, Pass, PassName, PassSegment, PassSegmentError,
};
use crate::compiler::pipeline::{CompilerPipeline, CompilerPipelineError};

fn name(value: &str) -> PassName {
    PassName::new(value).unwrap()
}

struct CloseStub {
    name: PassName,
    output: ClosureIr,
}

impl Pass for CloseStub {
    type Input = Documents;
    type Output = ClosureIr;
    type Error = Infallible;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, _input: Documents) -> Result<ClosureIr, Infallible> {
        Ok(self.output.clone())
    }
}

fn prefix() -> CompilerPipeline {
    let mut pipeline = CompilerPipeline::default();
    pipeline
        .push_artifact(CloseStub {
            name: name("linked-fixture"),
            output: super::tests::fixture(),
        })
        .unwrap();
    pipeline
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#PASS-LOWERING")]
fn assemble_descriptor_is_closure_artifact_to_lane_artifact() {
    let pass = AssemblePass::new();
    assert_eq!(pass.name().as_str(), ASSEMBLE_PASS_NAME);
    assert_eq!(ClosureIr::SHAPE, <AssemblePass as Pass>::Input::SHAPE);
    assert_eq!(LaneIr::SHAPE, <AssemblePass as Pass>::Output::SHAPE);
}

#[test]
fn manager_invokes_assemble_once_for_one_heterogeneous_artifact() {
    let mut pipeline = prefix();
    pipeline.push_artifact(AssemblePass::new()).unwrap();
    reset_assemble_invocations();
    let lane = pipeline.run_to_lane(Documents::new(Vec::new())).unwrap();
    assert_eq!(assemble_invocations(), 1);
    assert_eq!(lane.contributions.len(), 5);
}

#[test]
fn removing_or_replacing_assemble_breaks_the_real_lane_boundary() {
    let removed = prefix()
        .run_to_lane(Documents::new(Vec::new()))
        .unwrap_err();
    assert!(matches!(
        removed,
        CompilerPipelineError::ScheduleBoundary {
            boundary: "artifact lane output",
            ..
        }
    ));

    let mut identity = prefix();
    identity
        .push_artifact(IdentityPass::<ClosureIr>::new(name("assemble-identity")))
        .unwrap();
    let error = identity
        .run_to_lane(Documents::new(Vec::new()))
        .unwrap_err();
    assert!(matches!(
        error,
        CompilerPipelineError::ScheduleBoundary {
            boundary: "artifact lane output",
            ..
        }
    ));
}

#[test]
fn invalid_link_is_attributed_to_assemble_with_its_concrete_source() {
    let mut closure = super::tests::fixture();
    closure.link = LinkState::Unlinked;
    let mut segment = PassSegment::default();
    segment.push(AssemblePass::new()).unwrap();
    let error = segment.run(AnyIr::Closure(closure)).unwrap_err();
    let PassSegmentError::PassFailed { pass, source } = error else {
        panic!("expected named pass failure")
    };
    assert_eq!(pass.as_str(), ASSEMBLE_PASS_NAME);
    assert!(matches!(
        source.downcast_ref::<AssemblePassError>(),
        Some(AssemblePassError::InvalidLink(error))
            if matches!(error.as_ref(), LinkPassError::Unlinked)
    ));
}

#[test]
fn a_second_assemble_is_rejected_by_name_before_shape() {
    let mut pipeline = prefix();
    pipeline.push_artifact(AssemblePass::new()).unwrap();
    let error = pipeline.push_artifact(AssemblePass::new()).unwrap_err();
    assert!(matches!(
        error,
        CompilerPipelineError::DuplicateName { ref pass }
            if pass.as_str() == ASSEMBLE_PASS_NAME
    ));
}
