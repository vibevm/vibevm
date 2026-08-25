//! Built-in passes and the declared schedule prefix migrated so far.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR");

use crate::DocTree;

use super::ir::{DocumentIr, Documents, SourceIr};
use super::pass::{Pass, PassName};
use super::pipeline::CompilerPipeline;

const PARSE_PASS_NAME: &str = "parse";
const MARKDOWN_FORMAT: &str = "markdown";

/// The built-in source-to-document lowering.
///
/// The shipping [`crate::SectionSource`] seam supplies canonical Markdown even
/// when the authored file is XML, so this pass has one built-in frontend today.
/// R6 may register additional frontend passes without widening this one.
struct ParsePass {
    name: PassName,
}

impl ParsePass {
    fn new() -> Self {
        Self {
            name: PassName::new(PARSE_PASS_NAME)
                .expect("the static built-in parse pass name is non-blank"),
        }
    }
}

impl Pass for ParsePass {
    type Input = SourceIr;
    type Output = DocumentIr;
    type Error = ParseError;

    fn name(&self) -> &PassName {
        &self.name
    }

    fn run(&self, input: SourceIr) -> Result<DocumentIr, ParseError> {
        if input.format().as_str() != MARKDOWN_FORMAT {
            return Err(ParseError {
                format: input.format().as_str().to_string(),
            });
        }

        let tree = DocTree::parse(input.text());
        Ok(DocumentIr::new(input, tree))
    }
}

/// A parse failure remains the source of the manager's named-pass error.
#[derive(Debug, thiserror::Error)]
#[error("the built-in parser does not accept source format `{format}`")]
struct ParseError {
    format: String,
}

/// The declared built-in schedule prefix currently used by production.
///
/// Keeping construction in one function makes the list executable rather than
/// a registry beside a separate call path. R3.2 appends later built-ins here as
/// each phase migrates.
fn declared_pipeline() -> CompilerPipeline {
    let mut pipeline = CompilerPipeline::default();
    pipeline
        .push_document(ParsePass::new())
        .expect("the static built-in parse schedule is valid");
    pipeline
}

/// Parse one worklist through the same pipeline manager future passes use.
pub(crate) fn parse_sources(sources: Vec<SourceIr>) -> Documents {
    declared_pipeline()
        .run_documents(sources)
        .expect("the private built-in parse schedule accepts canonical Markdown sources")
}

#[cfg(test)]
mod tests {
    use specmark::verifies;

    use super::*;
    use crate::SpecAddress;
    use crate::compiler::ir::{DocumentAddress, SourceFormatId};
    use crate::compiler::pass::{IrPayload, PassSegmentError};
    use crate::compiler::pipeline::{CompilerPipelineError, ScheduleItem};

    fn source(format: &str, text: &str) -> SourceIr {
        SourceIr::new(
            DocumentAddress::Spec(
                SpecAddress::parse("spec://org.demo/pkg/common/doc#root").unwrap(),
            ),
            SourceFormatId::new(format).unwrap(),
            text,
        )
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
    fn production_prefix_declares_parse_then_the_gather_boundary() {
        let pipeline = declared_pipeline();
        let schedule = pipeline.schedule();

        assert!(matches!(
            schedule.as_slice(),
            [ScheduleItem::Pass(descriptor), ScheduleItem::GatherDocuments]
                if descriptor.name.as_str() == PARSE_PASS_NAME
                    && descriptor.input == SourceIr::SHAPE
                    && descriptor.output == DocumentIr::SHAPE
        ));
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-LEVELS")]
    fn parse_runs_once_for_each_addressed_document_then_gathers() {
        let documents = parse_sources(vec![
            source(MARKDOWN_FORMAT, "# One {#one}\n"),
            source(MARKDOWN_FORMAT, "# Two {#two}\n"),
        ]);

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
        let error = declared_pipeline()
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
}
