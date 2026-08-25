//! Built-in passes and the declared schedule prefix migrated so far.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR");

use std::collections::HashSet;

use crate::use_graph::{UseGraphError, use_addresses};
use crate::{DocTree, SectionSource, SpecAddress};

use super::close::{CLOSE_PASS_NAME, ClosePass, CloseState};
use super::ir::{ClosureIr, DocumentAddress, DocumentIr, SourceFormatId, SourceIr};
use super::pass::{Pass, PassName, PassSegmentError};
use super::pipeline::{CompilerPipeline, CompilerPipelineError};

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
pub(crate) struct BuiltinSchedule {
    pipeline: CompilerPipeline,
    close_state: CloseState,
}

impl BuiltinSchedule {
    fn new(seed: SpecAddress) -> Self {
        let close_state = CloseState::default();
        let mut pipeline = CompilerPipeline::default();
        pipeline
            .push_document(ParsePass::new())
            .expect("the static built-in parse schedule is valid");
        pipeline
            .push_artifact(ClosePass::new(seed, close_state.clone()))
            .expect("the static built-in close schedule is valid");
        Self {
            pipeline,
            close_state,
        }
    }

    fn parse_source(&self, source: SourceIr) -> DocumentIr {
        self.pipeline
            .run_document(source)
            .expect("the private parse segment accepts canonical Markdown sources")
    }

    fn record_failure(&self, address: &SpecAddress, reason: String) {
        self.close_state.record_failure(address, reason);
    }

    fn close(&self, documents: Vec<DocumentIr>) -> Result<ClosureIr, UseGraphError> {
        let documents = self.pipeline.gather_documents(documents);
        match self.pipeline.run_to_closure(documents) {
            Ok(closure) => Ok(closure),
            Err(CompilerPipelineError::Segment(PassSegmentError::PassFailed { pass, source }))
                if pass.as_str() == CLOSE_PASS_NAME =>
            {
                source
                    .downcast::<UseGraphError>()
                    .map(|error| Err(*error))
                    .unwrap_or_else(|source| {
                        panic!("the close pass returned an unexpected error type: {source}")
                    })
            }
            Err(error) => panic!("the private built-in close schedule is invalid: {error}"),
        }
    }
}

/// Compile one compatibility seed's explicit `#use` closure through the
/// declared parse/gather/close schedule. `#source` and `#embed` resolution stay
/// exclusively in their legacy tail until their named pass atoms.
pub(crate) fn compile_closure(
    seed: &SpecAddress,
    source: &impl SectionSource,
) -> Result<ClosureIr, UseGraphError> {
    let schedule = BuiltinSchedule::new(seed.clone());
    let mut seen = HashSet::new();
    let mut documents = Vec::new();
    discover_documents(seed, source, &schedule, &mut seen, &mut documents);
    schedule.close(documents)
}

fn discover_documents(
    address: &SpecAddress,
    source: &impl SectionSource,
    schedule: &BuiltinSchedule,
    seen: &mut HashSet<String>,
    documents: &mut Vec<DocumentIr>,
) {
    if !seen.insert(address.without_pin()) {
        return;
    }
    let text = match source.section_text(address) {
        Ok(text) => text,
        Err(reason) => {
            schedule.record_failure(address, reason);
            return;
        }
    };
    let document = schedule.parse_source(SourceIr::new(
        DocumentAddress::Spec(address.clone()),
        SourceFormatId::canonical_markdown(),
        text,
    ));
    let targets = use_addresses(document.tree().directives());
    documents.push(document);
    for target in targets {
        discover_documents(&target, source, schedule, seen, documents);
    }
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

    fn seed() -> SpecAddress {
        SpecAddress::parse("spec://org.demo/pkg/common/doc#root").unwrap()
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR")]
    fn production_prefix_declares_parse_gather_close() {
        let pipeline = BuiltinSchedule::new(seed()).pipeline;
        let schedule = pipeline.schedule();

        assert!(matches!(
            schedule.as_slice(),
            [
                ScheduleItem::Pass(parse),
                ScheduleItem::GatherDocuments,
                ScheduleItem::Pass(close),
            ] if parse.name.as_str() == PARSE_PASS_NAME
                && parse.input == SourceIr::SHAPE
                && parse.output == DocumentIr::SHAPE
                && close.name.as_str() == CLOSE_PASS_NAME
                && close.input == super::super::ir::Documents::SHAPE
                && close.output == ClosureIr::SHAPE
        ));
    }

    #[test]
    #[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#IR-LEVELS")]
    fn parse_runs_once_for_each_addressed_document_then_gathers() {
        let schedule = BuiltinSchedule::new(seed());
        let documents = schedule
            .pipeline
            .run_documents(vec![
                source(MARKDOWN_FORMAT, "# One {#one}\n"),
                source(MARKDOWN_FORMAT, "# Two {#two}\n"),
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
        let error = BuiltinSchedule::new(seed())
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
}
