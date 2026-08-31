use std::convert::Infallible;

use specmark::verifies;

use super::*;
use crate::{DocTree, SpecAddress};

use crate::compiler::ir::{ArtifactContext, DocumentAddress, SourceFormatId, StaticCompileMode};
use crate::compiler::pass::{IdentityPass, IrPayload, PassName};

fn pass_name(value: &str) -> PassName {
    PassName::new(value).unwrap()
}

fn source(anchor: &str, text: &str) -> SourceIr {
    SourceIr::reached(
        DocumentAddress::Spec(
            SpecAddress::parse(&format!("spec://org.demo/pkg/common/shared#{anchor}")).unwrap(),
        ),
        SourceFormatId::new("markdown").unwrap(),
        text,
    )
}

struct AppendSource {
    name: PassName,
}

impl Pass for AppendSource {
    type Input = SourceIr;
    type Output = SourceIr;
    type Error = Infallible;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, mut input: SourceIr) -> Result<SourceIr, Infallible> {
        input.text_mut().push_str("TRANSFORMED\n");
        Ok(input)
    }
}

struct ParseOwned {
    name: PassName,
}

impl Pass for ParseOwned {
    type Input = SourceIr;
    type Output = DocumentIr;
    type Error = Infallible;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, input: SourceIr) -> Result<DocumentIr, Infallible> {
        let (address, format, subject, text) = input.into_parts();
        let tree = DocTree::parse(&text);
        Ok(DocumentIr::new(
            SourceIr::new(address, format, subject, text),
            tree,
        ))
    }
}

struct ReparseOwned {
    name: PassName,
}

impl Pass for ReparseOwned {
    type Input = DocumentIr;
    type Output = DocumentIr;
    type Error = Infallible;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, input: DocumentIr) -> Result<DocumentIr, Infallible> {
        let (mut source, _tree) = input.into_parts();
        source.text_mut().push_str("DOCUMENT-PASS\n");
        let tree = DocTree::parse(source.text());
        let mut output = DocumentIr::new(source, DocTree::parse("discarded"));
        *output.tree_mut() = tree;
        Ok(output)
    }
}

struct EmitDocuments {
    name: PassName,
}

impl Pass for EmitDocuments {
    type Input = Documents;
    type Output = EmittedIr;
    type Error = Infallible;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, mut input: Documents) -> Result<EmittedIr, Infallible> {
        for document in input.iter_mut() {
            let text = document.source().text().to_string();
            *document.tree_mut() = DocTree::parse(&text);
        }
        let bytes = input
            .into_iter()
            .flat_map(|document| document.into_parts().0.into_parts().3.into_bytes())
            .collect();
        Ok(EmittedIr::testing(
            ArtifactContext::compatibility(StaticCompileMode::Plain),
            bytes,
        ))
    }
}

fn runnable_pipeline() -> CompilerPipeline<'static> {
    let mut pipeline = CompilerPipeline::default();
    pipeline
        .push_document(AppendSource {
            name: pass_name("source-transform"),
        })
        .unwrap();
    pipeline
        .push_document(ParseOwned {
            name: pass_name("parse"),
        })
        .unwrap();
    pipeline
        .push_document(ReparseOwned {
            name: pass_name("document-transform"),
        })
        .unwrap();
    pipeline
        .push_artifact(EmitDocuments {
            name: pass_name("emit:test"),
        })
        .unwrap();
    pipeline
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn scheduler_maps_documents_gathers_once_then_runs_the_artifact_segment() {
    let pipeline = runnable_pipeline();
    let emitted = pipeline
        .run(vec![source("one", "ONE\n"), source("two", "TWO\n")])
        .unwrap();

    assert_eq!(emitted.context().artifact().as_str(), "static-fragment");
    assert_eq!(
        String::from_utf8(emitted.bytes).unwrap(),
        concat!(
            "ONE\nTRANSFORMED\nDOCUMENT-PASS\n",
            "TWO\nTRANSFORMED\nDOCUMENT-PASS\n",
        )
    );
    assert!(matches!(
        pipeline.schedule().as_slice(),
        [
            ScheduleItem::Pass(_),
            ScheduleItem::Pass(_),
            ScheduleItem::Pass(_),
            ScheduleItem::GatherDocuments,
            ScheduleItem::Pass(_),
        ]
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
fn duplicate_pass_names_are_global_across_both_segments() {
    let mut pipeline = CompilerPipeline::default();
    pipeline
        .push_document(IdentityPass::<SourceIr>::new(pass_name("same")))
        .unwrap();
    let error = pipeline
        .push_artifact(IdentityPass::<Documents>::new(pass_name("same")))
        .unwrap_err();
    assert!(matches!(
        error,
        CompilerPipelineError::DuplicateName { ref pass } if pass.as_str() == "same"
    ));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-LEVELS")]
fn segments_reject_passes_that_bypass_the_gather_cardinality_boundary() {
    let mut document = CompilerPipeline::default();
    let error = document
        .push_document(IdentityPass::<Documents>::new(pass_name(
            "artifact-in-document",
        )))
        .unwrap_err();
    assert!(matches!(
        error,
        CompilerPipelineError::WrongSegmentCardinality {
            segment: "document",
            expected: IrCardinality::Document,
            input: IrShape {
                cardinality: IrCardinality::Artifact,
                ..
            },
            output: IrShape {
                cardinality: IrCardinality::Artifact,
                ..
            },
            ..
        }
    ));

    let mut artifact = CompilerPipeline::default();
    let error = artifact
        .push_artifact(IdentityPass::<DocumentIr>::new(pass_name(
            "document-in-artifact",
        )))
        .unwrap_err();
    assert!(matches!(
        error,
        CompilerPipelineError::WrongSegmentCardinality {
            segment: "artifact",
            expected: IrCardinality::Artifact,
            input: IrShape {
                cardinality: IrCardinality::Document,
                ..
            },
            output: IrShape {
                cardinality: IrCardinality::Document,
                ..
            },
            ..
        }
    ));
}

#[test]
fn incomplete_schedule_fails_before_consuming_any_source() {
    let mut pipeline = CompilerPipeline::default();
    pipeline
        .push_document(IdentityPass::<SourceIr>::new(pass_name("source-only")))
        .unwrap();
    pipeline
        .push_artifact(EmitDocuments {
            name: pass_name("emit:test"),
        })
        .unwrap();

    let error = pipeline.run(Vec::new()).unwrap_err();
    assert!(matches!(
        error,
        CompilerPipelineError::ScheduleBoundary {
            boundary: "document segment output",
            ..
        }
    ));
}

#[test]
fn gathering_is_typed_and_is_not_a_registered_pass() {
    let documents = GatherDocuments.run(Vec::new());
    assert!(documents.is_empty());
    assert_eq!(Documents::SHAPE.level, IrLevel::Document);
    assert_eq!(Documents::SHAPE.cardinality, IrCardinality::Artifact);
}
