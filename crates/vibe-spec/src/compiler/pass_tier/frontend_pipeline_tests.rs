use std::convert::Infallible;
use std::sync::{Arc, Mutex};

use crate::compiler::ir::{ArtifactInput, ArtifactPlan, ArtifactTarget, DocumentIr, SourceIr};
use crate::compiler::pass::{Pass, PassName};
use crate::compiler::pipeline::{
    CompilerPipeline, ScheduleItem, gather_invocations, reset_gather_invocations,
};
use crate::compiler::trace::{CompileTraceSink, PassTraceEvent};
use crate::compiler::worklist::discover_with_formats;
use crate::embed::ResolvedSource;
use crate::{DocTree, SectionSource, SpecAddress};

use super::frontend::FrontendSeats;

type Log = Arc<Mutex<Vec<String>>>;

struct RecordSource(Log);

impl Pass for RecordSource {
    type Input = SourceIr;
    type Output = SourceIr;
    type Error = Infallible;

    fn name(&self) -> &PassName {
        static NAME: std::sync::LazyLock<PassName> =
            std::sync::LazyLock::new(|| PassName::new("source-before").unwrap());
        &NAME
    }

    fn run(&self, mut input: SourceIr) -> Result<SourceIr, Infallible> {
        self.0
            .lock()
            .unwrap()
            .push(format!("source:{}", input.format().as_str()));
        input.text_mut().push_str("\nsource-before");
        Ok(input)
    }
}

struct ParseMarkdown(Log);

impl Pass for ParseMarkdown {
    type Input = SourceIr;
    type Output = DocumentIr;
    type Error = Infallible;

    fn name(&self) -> &PassName {
        static NAME: std::sync::LazyLock<PassName> =
            std::sync::LazyLock::new(|| PassName::new("parse").unwrap());
        &NAME
    }

    fn run(&self, input: SourceIr) -> Result<DocumentIr, Infallible> {
        self.0.lock().unwrap().push("parse:markdown".into());
        let tree = DocTree::parse(input.text());
        Ok(DocumentIr::new(input, tree))
    }
}

struct ParseTxt {
    log: Log,
    title: String,
}

impl Pass for ParseTxt {
    type Input = SourceIr;
    type Output = DocumentIr;
    type Error = Infallible;

    fn name(&self) -> &PassName {
        static NAME: std::sync::LazyLock<PassName> =
            std::sync::LazyLock::new(|| PassName::new("pass:org.demo/formats#txt").unwrap());
        &NAME
    }

    fn run(&self, input: SourceIr) -> Result<DocumentIr, Infallible> {
        self.log.lock().unwrap().push("parse:txt".into());
        assert!(input.text().ends_with("source-before"));
        let body = input
            .text()
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");
        let tree = DocTree::parse(&format!("# {} {{#root}}\n\n{body}\n", self.title));
        Ok(DocumentIr::new(input, tree))
    }
}

struct RecordDocument(Log);

impl Pass for RecordDocument {
    type Input = DocumentIr;
    type Output = DocumentIr;
    type Error = Infallible;

    fn name(&self) -> &PassName {
        static NAME: std::sync::LazyLock<PassName> =
            std::sync::LazyLock::new(|| PassName::new("document-after").unwrap());
        &NAME
    }

    fn run(&self, input: DocumentIr) -> Result<DocumentIr, Infallible> {
        self.0
            .lock()
            .unwrap()
            .push(format!("document:{}", input.source().format().as_str()));
        Ok(input)
    }
}

struct MixedSource;

#[derive(Default)]
struct TraceNames(Mutex<Vec<String>>);

impl CompileTraceSink for TraceNames {
    fn record(&self, event: &PassTraceEvent<'_>) {
        self.0.lock().unwrap().push(event.pass().to_owned());
    }
}

impl SectionSource for MixedSource {
    fn section_text(&self, _addr: &SpecAddress) -> Result<String, String> {
        panic!("the typed discovery boundary must be used")
    }

    fn source_metadata(
        &self,
        addr: &SpecAddress,
        active_formats: &[String],
    ) -> Result<crate::ResolvedSourceMetadata, String> {
        if addr.to_string().contains("/txt/") {
            assert_eq!(active_formats, ["txt"]);
            crate::ResolvedSourceMetadata::new("NOTE.txt", "txt".into(), "NOTE".into())
        } else {
            crate::ResolvedSourceMetadata::new("MD.md", "markdown".into(), "MD".into())
        }
    }

    fn read_resolved_source(
        &self,
        _addr: &SpecAddress,
        metadata: crate::ResolvedSourceMetadata,
    ) -> Result<ResolvedSource, String> {
        if metadata.format() == "txt" {
            ResolvedSource::custom("first\n\nsecond\n".into(), "txt".into(), "NOTE".into())
        } else {
            Ok(ResolvedSource::markdown(
                "# MD {#root}\n\nbody\n".into(),
                "MD".into(),
            ))
        }
    }
}

fn address(package: &str) -> SpecAddress {
    SpecAddress::parse(&format!("spec://org.demo/{package}/common/NOTE#root")).unwrap()
}

#[test]
fn mixed_formats_share_transform_brackets_and_one_gather_in_worklist_order() {
    let log = Log::default();
    let mut pipeline = CompilerPipeline::default();
    pipeline.push_document(RecordSource(log.clone())).unwrap();
    pipeline
        .push_builtin_document(ParseMarkdown(log.clone()))
        .unwrap();
    pipeline.push_document(RecordDocument(log.clone())).unwrap();

    let mut frontends = FrontendSeats::default();
    let txt_log = log.clone();
    frontends.replace_for_test("txt", move |stem| ParseTxt {
        log: txt_log.clone(),
        title: stem.to_owned(),
    });
    frontends.validate(&pipeline).unwrap();
    let trace = TraceNames::default();

    let md = address("md");
    let txt = address("txt");
    let plan = ArtifactPlan::static_lane(
        ArtifactTarget::StaticMarkdown,
        "vibevm/vibespecs/boot/STATIC.md",
        "vibevm/vibedeps",
        vec![
            ArtifactInput::normal("org.demo/md", "common/NOTE.md", md).unwrap(),
            ArtifactInput::normal("org.demo/txt", "common/NOTE.txt", txt).unwrap(),
        ],
    )
    .unwrap();

    let worklist = discover_with_formats(
        &plan,
        &MixedSource,
        &["txt".into()],
        |source, stem| frontends.run(&pipeline, source, stem, Some(&trace)),
        |_address, _reason| {},
    )
    .unwrap();

    assert_eq!(
        *log.lock().unwrap(),
        [
            "source:markdown",
            "parse:markdown",
            "document:markdown",
            "source:txt",
            "parse:txt",
            "document:txt",
        ]
    );
    assert_eq!(worklist.documents.len(), 2);
    let tree = worklist.documents[1].tree();
    assert_eq!(
        tree.node(tree.node(tree.root()).children[0]).heading,
        "NOTE"
    );
    assert_eq!(
        *trace.0.lock().unwrap(),
        [
            "source-before",
            "parse",
            "document-after",
            "source-before",
            "pass:org.demo/formats#txt",
            "document-after",
        ]
    );
    assert_eq!(
        pipeline
            .schedule()
            .iter()
            .filter(|item| matches!(item, ScheduleItem::GatherDocuments))
            .count(),
        1
    );
    reset_gather_invocations();
    pipeline.gather_documents(worklist.documents).unwrap();
    assert_eq!(gather_invocations(), 1);
}
