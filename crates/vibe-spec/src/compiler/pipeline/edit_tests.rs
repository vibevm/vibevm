use super::*;
use crate::compiler::ir::{DocumentIr, Documents, EmittedIr, SourceIr};
use crate::compiler::pass::{IdentityPass, Pass, PassName, PassSegmentError};

struct ParseForEdit {
    name: PassName,
}

impl Pass for ParseForEdit {
    type Input = SourceIr;
    type Output = DocumentIr;
    type Error = std::convert::Infallible;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, _input: SourceIr) -> Result<DocumentIr, Self::Error> {
        unreachable!("schedule edit tests never execute their shape-only passes")
    }
}

struct DocumentToSource {
    name: PassName,
}

impl Pass for DocumentToSource {
    type Input = DocumentIr;
    type Output = SourceIr;
    type Error = std::convert::Infallible;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, _input: DocumentIr) -> Result<SourceIr, Self::Error> {
        unreachable!("schedule edit tests never execute their shape-only passes")
    }
}

struct EmitForEdit {
    name: PassName,
}

impl Pass for EmitForEdit {
    type Input = Documents;
    type Output = EmittedIr;
    type Error = std::convert::Infallible;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, _input: Documents) -> Result<EmittedIr, Self::Error> {
        unreachable!("schedule edit tests never execute their shape-only passes")
    }
}

fn name(value: &str) -> PassName {
    PassName::new(value).unwrap()
}

fn document(value: &str) -> IdentityPass<DocumentIr> {
    IdentityPass::new(name(value))
}

fn schedule_names(pipeline: &CompilerPipeline<'_>) -> Vec<String> {
    pipeline
        .schedule()
        .into_iter()
        .filter_map(|item| match item {
            ScheduleItem::Pass(pass) => Some(pass.name.as_str().to_owned()),
            ScheduleItem::GatherDocuments => None,
        })
        .collect()
}

fn base() -> CompilerPipeline<'static> {
    let mut pipeline = CompilerPipeline::default();
    pipeline
        .push_builtin_document(ParseForEdit {
            name: name("first"),
        })
        .unwrap();
    pipeline.push_builtin_document(document("anchor")).unwrap();
    pipeline
        .push_builtin_document(document("replace-me"))
        .unwrap();
    pipeline.push_builtin_document(document("last")).unwrap();
    pipeline
        .push_builtin_artifact(EmitForEdit { name: name("emit") })
        .unwrap();
    pipeline
}

#[test]
fn declaration_order_is_retained_on_both_sides_and_replacement_removes_the_builtin() {
    let mut pipeline = base();
    pipeline
        .apply_builtin_edits(vec![
            PipelineEdit::before(name("anchor"), document("before-a")),
            PipelineEdit::before(name("anchor"), document("before-b")),
            PipelineEdit::after(name("anchor"), document("after-a")),
            PipelineEdit::after(name("anchor"), document("after-b")),
            PipelineEdit::replace(name("replace-me"), document("replacement")),
        ])
        .unwrap();
    assert_eq!(
        schedule_names(&pipeline),
        [
            "first",
            "before-a",
            "before-b",
            "anchor",
            "after-a",
            "after-b",
            "replacement",
            "last",
            "emit",
        ]
    );
}

#[test]
fn one_builtin_accepts_at_most_one_replacement() {
    let mut pipeline = base();
    let error = pipeline
        .apply_builtin_edits(vec![
            PipelineEdit::replace(name("anchor"), document("replacement-a")),
            PipelineEdit::replace(name("anchor"), document("replacement-b")),
        ])
        .unwrap_err();
    assert!(matches!(
        error,
        CompilerPipelineError::DuplicateReplacement { ref anchor }
            if anchor.as_str() == "anchor"
    ));
}

#[test]
fn plugin_and_unknown_names_are_not_builtin_anchors() {
    for anchor in ["plugin", "unknown"] {
        let mut pipeline = base();
        pipeline.push_document(document("plugin")).unwrap();
        let error = pipeline
            .apply_builtin_edits(vec![PipelineEdit::after(
                name(anchor),
                document("candidate"),
            )])
            .unwrap_err();
        assert!(matches!(
            error,
            CompilerPipelineError::AnchorNotBuiltin { anchor: ref rejected }
                if rejected.as_str() == anchor
        ));
    }
}

#[test]
fn a_replacement_must_preserve_the_builtin_descriptor_exactly() {
    let mut pipeline = base();
    let error = pipeline
        .apply_builtin_edits(vec![PipelineEdit::replace(
            name("anchor"),
            IdentityPass::<SourceIr>::new(name("wrong-shape")),
        )])
        .unwrap_err();
    assert!(matches!(
        error,
        CompilerPipelineError::ReplacementShape { ref anchor, .. }
            if anchor.as_str() == "anchor"
    ));
}

#[test]
fn the_complete_reconstructed_chain_is_checked_before_the_pipeline_moves() {
    let mut pipeline = base();
    let before = schedule_names(&pipeline);
    let error = pipeline
        .apply_builtin_edits(vec![
            PipelineEdit::after(name("anchor"), document("valid")),
            PipelineEdit::before(
                name("replace-me"),
                IdentityPass::<SourceIr>::new(name("broken")),
            ),
        ])
        .unwrap_err();
    assert!(matches!(
        error,
        CompilerPipelineError::Segment(PassSegmentError::BrokenChain { .. })
    ));
    assert_eq!(
        schedule_names(&pipeline),
        before,
        "validation is transactional"
    );
}

#[test]
fn before_first_cannot_change_input_and_a_corrected_retry_succeeds() {
    let mut pipeline = base();
    let before = schedule_names(&pipeline);
    let error = pipeline
        .apply_builtin_edits(vec![PipelineEdit::before(
            name("first"),
            DocumentToSource {
                name: name("wrong-input"),
            },
        )])
        .unwrap_err();
    assert!(matches!(
        error,
        CompilerPipelineError::ScheduleBoundary {
            boundary: "document segment input",
            ..
        }
    ));
    assert_eq!(schedule_names(&pipeline), before);

    pipeline
        .apply_builtin_edits(vec![PipelineEdit::before(
            name("first"),
            IdentityPass::<SourceIr>::new(name("correct-input")),
        )])
        .unwrap();
    assert_eq!(
        schedule_names(&pipeline),
        [
            "correct-input",
            "first",
            "anchor",
            "replace-me",
            "last",
            "emit"
        ]
    );
}

#[test]
fn after_last_cannot_change_output_and_a_corrected_retry_succeeds() {
    let mut pipeline = base();
    let before = schedule_names(&pipeline);
    let error = pipeline
        .apply_builtin_edits(vec![PipelineEdit::after(
            name("last"),
            DocumentToSource {
                name: name("wrong-output"),
            },
        )])
        .unwrap_err();
    assert!(matches!(
        error,
        CompilerPipelineError::ScheduleBoundary {
            boundary: "document segment output",
            ..
        }
    ));
    assert_eq!(schedule_names(&pipeline), before);

    pipeline
        .apply_builtin_edits(vec![PipelineEdit::after(
            name("last"),
            document("correct-output"),
        )])
        .unwrap();
    assert_eq!(
        schedule_names(&pipeline),
        [
            "first",
            "anchor",
            "replace-me",
            "last",
            "correct-output",
            "emit"
        ]
    );
}

#[test]
fn one_immutable_builtin_snapshot_accepts_one_edit_batch() {
    let mut pipeline = base();
    pipeline.apply_builtin_edits(Vec::new()).unwrap();
    assert!(matches!(
        pipeline.apply_builtin_edits(Vec::new()),
        Err(CompilerPipelineError::EditsAlreadyApplied)
    ));
    assert!(matches!(
        pipeline.push_builtin_document(document("late-builtin")),
        Err(CompilerPipelineError::EditsAlreadyApplied)
    ));
}
