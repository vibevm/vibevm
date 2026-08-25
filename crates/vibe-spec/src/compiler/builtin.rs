//! Built-in passes and the declared schedule prefix migrated so far.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#IR-REFACTOR");

use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::use_graph::{UseGraphError, use_addresses};
use crate::{DirectiveKind, DocTree, SectionSource, SpecAddress};

use super::close::{CLOSE_PASS_NAME, ClosePass, CloseState};
use super::ir::{ClosureIr, DocumentAddress, DocumentIr, SourceFormatId, SourceIr};
use super::merge::{MERGE_PASS_NAME, MergePass, MergePassError};
use super::pass::{Pass, PassName, PassSegmentError};
use super::pipeline::{CompilerPipeline, CompilerPipelineError};
use super::source_snapshot::{DocumentObservation, ExpansionObservation, SourceResolutionSnapshot};

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
        pipeline
            .push_artifact(MergePass::new())
            .expect("the static built-in merge schedule is valid");
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

    fn close(
        &self,
        documents: Vec<DocumentIr>,
    ) -> Result<ClosureIr, crate::pipeline::CompileError> {
        let documents = self.pipeline.gather_documents(documents);
        match self.pipeline.run_to_closure(documents) {
            Ok(closure) => Ok(closure),
            Err(CompilerPipelineError::Segment(PassSegmentError::PassFailed { pass, source }))
                if pass.as_str() == CLOSE_PASS_NAME =>
            {
                source
                    .downcast::<UseGraphError>()
                    .map(|error| Err(crate::pipeline::CompileError::UseGraph(*error)))
                    .unwrap_or_else(|source| {
                        panic!("the close pass returned an unexpected error type: {source}")
                    })
            }
            Err(CompilerPipelineError::Segment(PassSegmentError::PassFailed { pass, source }))
                if pass.as_str() == MERGE_PASS_NAME =>
            {
                source
                    .downcast::<MergePassError>()
                    .map(|error| Err(error.into_compile_error()))
                    .unwrap_or_else(|source| {
                        panic!("the merge pass returned an unexpected error type: {source}")
                    })
            }
            Err(error) => panic!("the private built-in close schedule is invalid: {error}"),
        }
    }
}

/// Compile one compatibility seed through parse/gather/close/merge. Source
/// observations are frozen before gather and interpreted only by merge;
/// `#embed` stays exclusively in its legacy owner.
pub(crate) fn compile_merged_closure(
    seed: &SpecAddress,
    source: &impl SectionSource,
) -> Result<ClosureIr, crate::pipeline::CompileError> {
    let schedule = BuiltinSchedule::new(seed.clone());
    let (documents, snapshot) = discover_documents(seed, source, &schedule);
    schedule.close_state.set_pending_sources(snapshot);
    schedule.close(documents)
}

fn discover_documents(
    seed: &SpecAddress,
    source: &impl SectionSource,
    schedule: &BuiltinSchedule,
) -> (Vec<DocumentIr>, SourceResolutionSnapshot) {
    let mut resolved = BTreeMap::new();
    let mut failures = BTreeMap::new();
    let mut discovery_order = Vec::new();
    let mut use_seen = HashSet::new();
    let mut use_order = Vec::new();
    discover_uses(
        seed,
        source,
        schedule,
        &mut use_seen,
        &mut use_order,
        &mut discovery_order,
        &mut resolved,
        &mut failures,
    );

    let mut expansions = BTreeMap::new();
    let mut source_seen = HashSet::new();
    for key in use_order.clone() {
        discover_sources(
            &key,
            source,
            schedule,
            &mut source_seen,
            &mut discovery_order,
            &mut resolved,
            &mut failures,
            &mut expansions,
        );
    }

    let mut observations = failures;
    for (key, document) in &resolved {
        observations.insert(key.clone(), DocumentObservation::Resolved(document.clone()));
    }
    let snapshot = SourceResolutionSnapshot {
        discovery_order: discovery_order.clone(),
        documents: observations,
        expansions,
        explicit_use_keys: use_order.iter().cloned().collect::<BTreeSet<_>>(),
    };
    let documents = discovery_order
        .into_iter()
        .filter_map(|key| resolved.remove(&key))
        .collect();
    (documents, snapshot)
}

#[allow(clippy::too_many_arguments)]
fn discover_uses(
    address: &SpecAddress,
    source: &impl SectionSource,
    schedule: &BuiltinSchedule,
    seen: &mut HashSet<String>,
    use_order: &mut Vec<String>,
    discovery_order: &mut Vec<String>,
    resolved: &mut BTreeMap<String, DocumentIr>,
    failures: &mut BTreeMap<String, DocumentObservation>,
) {
    if !seen.insert(address.without_pin()) {
        return;
    }
    let key = address.without_pin();
    let text = match source.section_text(address) {
        Ok(text) => text,
        Err(reason) => {
            schedule.record_failure(address, reason.clone());
            failures
                .entry(key)
                .or_insert_with(|| DocumentObservation::Failed {
                    requested: address.clone(),
                    reason,
                });
            return;
        }
    };
    let document = schedule.parse_source(SourceIr::new(
        DocumentAddress::Spec(address.clone()),
        SourceFormatId::canonical_markdown(),
        text,
    ));
    let targets = use_addresses(document.tree().directives());
    discovery_order.push(key.clone());
    use_order.push(key.clone());
    resolved.insert(key, document);
    for target in targets {
        discover_uses(
            &target,
            source,
            schedule,
            seen,
            use_order,
            discovery_order,
            resolved,
            failures,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn discover_sources(
    key: &str,
    source: &impl SectionSource,
    schedule: &BuiltinSchedule,
    seen: &mut HashSet<String>,
    discovery_order: &mut Vec<String>,
    resolved: &mut BTreeMap<String, DocumentIr>,
    failures: &mut BTreeMap<String, DocumentObservation>,
    expansions: &mut BTreeMap<String, ExpansionObservation>,
) {
    if !seen.insert(key.to_string()) {
        return;
    }
    let Some(document) = resolved.get(key) else {
        return;
    };
    let patterns: Vec<SpecAddress> = document
        .tree()
        .directives()
        .directives
        .iter()
        .filter(|directive| directive.kind == DirectiveKind::Source)
        .map(|directive| directive.address.clone())
        .collect();
    for pattern in patterns {
        let pattern_key = pattern.without_pin();
        if !expansions.contains_key(&pattern_key) {
            let observation = match source.expand_pattern(&pattern) {
                Ok(targets) => ExpansionObservation::Resolved {
                    requested: pattern.clone(),
                    targets,
                },
                Err(reason) => ExpansionObservation::Failed {
                    requested: pattern.clone(),
                    reason,
                },
            };
            expansions.insert(pattern_key.clone(), observation);
        }
        let targets = match &expansions[&pattern_key] {
            ExpansionObservation::Resolved { targets, .. } => targets.clone(),
            ExpansionObservation::Failed { .. } => continue,
        };
        for target in targets {
            let target_key = target.without_pin();
            if !resolved.contains_key(&target_key) && !failures.contains_key(&target_key) {
                match source.section_text(&target) {
                    Ok(text) => {
                        let document = schedule.parse_source(SourceIr::new(
                            DocumentAddress::Spec(target.clone()),
                            SourceFormatId::canonical_markdown(),
                            text,
                        ));
                        discovery_order.push(target_key.clone());
                        resolved.insert(target_key.clone(), document);
                    }
                    Err(reason) => {
                        failures.insert(
                            target_key.clone(),
                            DocumentObservation::Failed {
                                requested: target.clone(),
                                reason,
                            },
                        );
                    }
                }
            }
            if resolved.contains_key(&target_key) {
                discover_sources(
                    &target_key,
                    source,
                    schedule,
                    seen,
                    discovery_order,
                    resolved,
                    failures,
                    expansions,
                );
            }
        }
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
    fn production_prefix_declares_parse_gather_close_merge() {
        let pipeline = BuiltinSchedule::new(seed()).pipeline;
        let schedule = pipeline.schedule();

        assert!(matches!(
            schedule.as_slice(),
            [
                ScheduleItem::Pass(parse),
                ScheduleItem::GatherDocuments,
                ScheduleItem::Pass(close),
                ScheduleItem::Pass(merge),
            ] if parse.name.as_str() == PARSE_PASS_NAME
                && parse.input == SourceIr::SHAPE
                && parse.output == DocumentIr::SHAPE
                && close.name.as_str() == CLOSE_PASS_NAME
                && close.input == super::super::ir::Documents::SHAPE
                && close.output == ClosureIr::SHAPE
                && merge.name.as_str() == MERGE_PASS_NAME
                && merge.input == ClosureIr::SHAPE
                && merge.output == ClosureIr::SHAPE
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
