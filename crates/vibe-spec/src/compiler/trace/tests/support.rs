//! The shared observer fixture: recording sinks, the smallest honest
//! two-document world, the real artifact plan, and the pass vehicles that
//! make each refusal reachable.

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::sync::Mutex;

use super::super::*;
use crate::compiler::ir::{
    ArtifactContext, ArtifactFrame, ArtifactId, ArtifactInput, ArtifactPlan, ArtifactTarget,
    DocumentAddress, DocumentIr, Documents, SourceFormatId, SourceIr, StaticCompileMode,
};
use crate::compiler::pass::{DynPass, IrPayload, Pass, PassDescriptor, PassName, PassSegmentError};
use crate::compiler::pipeline::{CompilerPipeline, ScheduleItem};
use crate::{SectionSource, SpecAddress};

/// One recorded observation, flattened to exactly what a recorder would keep —
/// every member the GENERATED trace-index type, so a real writer would move
/// these straight into a `PassEvent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Recorded {
    pub(super) pass: String,
    pub(super) status: index::PassStatus,
    pub(super) input: index::PassShape,
    pub(super) output: index::PassShape,
    /// `(pass, verify, encode)` presence — never an elapsed number, because a
    /// timing assertion on a real clock is a flake.
    pub(super) timings: (bool, bool, bool),
    pub(super) diagnostic: Option<String>,
    pub(super) snapshot: Option<Vec<u8>>,
}

fn recorded(event: &PassTraceEvent<'_>) -> Recorded {
    Recorded {
        pass: event.pass().to_string(),
        status: event.status().clone(),
        input: event.input().clone(),
        output: event.output().clone(),
        timings: (
            event.pass_duration().is_some(),
            event.verify_duration().is_some(),
            event.encode_duration().is_some(),
        ),
        diagnostic: event.diagnostic().map(str::to_string),
        snapshot: event.snapshot().map(<[u8]>::to_vec),
    }
}

#[derive(Default)]
pub(super) struct Recorder {
    events: Mutex<Vec<Recorded>>,
    /// When set, this sink stands down on every accepted output — the shape a
    /// workspace writer takes once its published-byte counter is spent.
    stand_down: bool,
}

impl Recorder {
    pub(super) fn on_budget() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            stand_down: true,
        }
    }

    pub(super) fn events(&self) -> Vec<Recorded> {
        self.events
            .lock()
            .expect("the recorder mutex is never poisoned")
            .clone()
    }

    pub(super) fn names(&self) -> Vec<String> {
        self.events().into_iter().map(|event| event.pass).collect()
    }

    pub(super) fn snapshots(&self) -> Vec<Vec<u8>> {
        self.events()
            .into_iter()
            .filter_map(|event| event.snapshot)
            .collect()
    }
}

impl CompileTraceSink for Recorder {
    fn record(&self, event: &PassTraceEvent<'_>) {
        self.events
            .lock()
            .expect("the recorder mutex is never poisoned")
            .push(recorded(event));
    }

    fn before_snapshot(&self, _pass: &str, _output: &index::PassShape) -> SnapshotDecision {
        if self.stand_down {
            SnapshotDecision::SkipBudget
        } else {
            SnapshotDecision::Encode
        }
    }
}

/// A sink that overrides nothing, so the DEFAULT pre-encode decision — and
/// only that — is what drives it.
#[derive(Default)]
pub(super) struct DefaultingRecorder(Mutex<Vec<Recorded>>);

impl DefaultingRecorder {
    pub(super) fn events(&self) -> Vec<Recorded> {
        self.0
            .lock()
            .expect("the recorder mutex is never poisoned")
            .clone()
    }
}

impl CompileTraceSink for DefaultingRecorder {
    fn record(&self, event: &PassTraceEvent<'_>) {
        self.0
            .lock()
            .expect("the recorder mutex is never poisoned")
            .push(recorded(event));
    }
}

/// Which observer callback a defective sink blows up in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PanicIn {
    Record,
    BeforeSnapshot,
}

/// A sink that PANICS instead of answering — an arbitrary downstream defect.
///
/// The panic message is deliberately recognisable, so a test can prove it
/// never reaches the compiler's own result or error text.
pub(super) struct PanickingSink {
    pub(super) at: PanicIn,
    calls: Mutex<usize>,
}

impl PanickingSink {
    pub(super) fn at(at: PanicIn) -> Self {
        Self {
            at,
            calls: Mutex::new(0),
        }
    }

    /// How many times the compiler reached the defective callback — proof the
    /// schedule kept going instead of stopping at the first blow-up.
    pub(super) fn calls(&self) -> usize {
        *self
            .calls
            .lock()
            .expect("the counter mutex is never poisoned")
    }

    /// Counted with the guard RELEASED before the blow-up, so the panic
    /// cannot poison the counter the assertions read afterwards.
    fn count(&self) {
        *self
            .calls
            .lock()
            .expect("the counter mutex is never poisoned") += 1;
    }
}

pub(super) const OBSERVER_PANIC: &str = "the observer is defective";

impl CompileTraceSink for PanickingSink {
    fn record(&self, _event: &PassTraceEvent<'_>) {
        if self.at == PanicIn::Record {
            self.count();
            panic!("{OBSERVER_PANIC}");
        }
    }

    fn before_snapshot(&self, _pass: &str, _output: &index::PassShape) -> SnapshotDecision {
        if self.at == PanicIn::BeforeSnapshot {
            self.count();
            panic!("{OBSERVER_PANIC}");
        }
        SnapshotDecision::Encode
    }
}

/// The smallest honest world whose ONE root pulls in a second addressed
/// document, so the real schedule invokes `parse` exactly twice.
pub(super) struct World(BTreeMap<String, String>);

impl World {
    pub(super) fn two_documents() -> Self {
        let mut map = BTreeMap::new();
        map.insert(
            "spec://org.demo/alpha/boot/entry#root".to_string(),
            "# Alpha {#root}\n#use spec://org.demo/shared/boot/base#root\nALPHA\n".to_string(),
        );
        map.insert(
            "spec://org.demo/shared/boot/base#root".to_string(),
            "# Shared {#root}\n##SHARED shared\n".to_string(),
        );
        Self(map)
    }

    /// The same world with the `#use` target missing, so a real built-in pass
    /// fails inside the artifact segment.
    pub(super) fn dangling_use() -> Self {
        let mut world = Self::two_documents();
        world.0.remove("spec://org.demo/shared/boot/base#root");
        world
    }
}

impl SectionSource for World {
    fn section_text(&self, address: &SpecAddress) -> Result<String, String> {
        self.0
            .get(&address.without_pin())
            .cloned()
            .ok_or_else(|| format!("missing {}", address.without_pin()))
    }
}

pub(super) fn plan() -> ArtifactPlan {
    ArtifactPlan::new(
        ArtifactContext::new(
            ArtifactId::new("static-md").unwrap(),
            ArtifactTarget::StaticMarkdown,
            ArtifactFrame::StaticLane {
                generated_path: "vibevm/vibespecs/boot/STATIC.md".to_string(),
                source_root: "vibevm/vibespecs".to_string(),
            },
            StaticCompileMode::QualifyPerNode,
        )
        .unwrap(),
        vec![
            ArtifactInput::normal(
                "org.demo/alpha",
                "boot/entry.md",
                SpecAddress::parse("spec://org.demo/alpha/boot/entry#root").unwrap(),
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

pub(super) fn pass_name(value: &str) -> PassName {
    PassName::new(value).unwrap()
}

pub(super) fn markdown_source(anchor: &str, text: &str) -> SourceIr {
    SourceIr::new(
        DocumentAddress::Spec(
            SpecAddress::parse(&format!("spec://org.demo/pkg/common/doc#{anchor}")).unwrap(),
        ),
        SourceFormatId::new("markdown").unwrap(),
        text,
    )
}

/// The declared schedule's pass names, read from the manager rather than
/// spelled out beside it: a hard-coded expectation would keep passing after
/// the schedule changed under it.
pub(super) fn declared_artifact_passes(pipeline: &CompilerPipeline) -> Vec<String> {
    pipeline
        .schedule()
        .into_iter()
        .filter_map(|item| match item {
            ScheduleItem::Pass(descriptor) => Some(descriptor.name.as_str().to_string()),
            ScheduleItem::GatherDocuments => None,
        })
        .collect()
}

/// A pass whose erased adapter lies about the carrier it returns.
pub(super) struct LyingOutput;

impl DynPass for LyingOutput {
    fn descriptor(&self) -> PassDescriptor {
        PassDescriptor {
            name: pass_name("lying-output"),
            input: SourceIr::SHAPE,
            output: SourceIr::SHAPE,
        }
    }

    fn run_erased(&self, _input: AnyIr) -> Result<AnyIr, PassSegmentError> {
        Ok(AnyIr::Documents(Documents::new(Vec::new())))
    }
}

pub(super) fn parse_like() -> impl Pass {
    struct ParseLike(PassName);
    impl Pass for ParseLike {
        type Input = SourceIr;
        type Output = DocumentIr;
        type Error = Infallible;

        fn name(&self) -> &PassName {
            &self.0
        }

        fn run(&self, input: SourceIr) -> Result<DocumentIr, Infallible> {
            let tree = crate::DocTree::parse(input.text());
            Ok(DocumentIr::new(input, tree))
        }
    }
    ParseLike(pass_name("parse-like"))
}

/// A document transform that forges a duplicate fact anchor, so the semantic
/// verifier refuses an output whose runtime SHAPE is perfectly correct.
pub(super) fn break_anchors() -> impl Pass {
    struct BreakAnchors(PassName);
    impl Pass for BreakAnchors {
        type Input = DocumentIr;
        type Output = DocumentIr;
        type Error = Infallible;

        fn name(&self) -> &PassName {
            &self.0
        }

        fn run(&self, input: DocumentIr) -> Result<DocumentIr, Infallible> {
            let (source, _tree) = input.into_parts();
            let forged = format!("{}\n##dup once\n\n##dup twice\n", source.text());
            Ok(DocumentIr::new(source, crate::DocTree::parse(&forged)))
        }
    }
    BreakAnchors(pass_name("break-anchors"))
}
