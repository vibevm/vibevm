//! The local T6b execution vehicles: mutating, failing and recording
//! behaviors that extend a CLONE of the shared identity registry for gap
//! REDs. They never enter `builtins()` and never alter the T5 golden.
//! Causality vehicles append FENCED code blocks — real parsed content with
//! no anchor or fact-id pressure across the shared/qualified lane.
//!
//! The lane-position vehicles live in `schedule_lane_vehicles`, beside the
//! T6c admission tests they exist for.

use std::sync::Arc;

use crate::DocTree;
use crate::compiler::ir::{
    DocumentAddress, DocumentIr, DocumentProvider, DocumentSubject, SourceIr,
};
use crate::compiler::worklist::document_key;

use super::behavior::{TransformBehavior, TransformBehaviorError};
use super::plan::{TransformConfig, TransformStage};
use super::registry::TransformRegistry;
use super::registry_test_support::identity_registry;

// Per-thread invocation counters: the suite runs tests in parallel, and a
// process-wide static would let one test's vehicles pollute another's count.
std::thread_local! {
    pub(super) static SOURCE_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    pub(super) static DOCUMENT_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    pub(super) static EMITTED_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

pub(super) fn reset_vehicle_counts() {
    SOURCE_COUNT.with(|count| count.set(0));
    DOCUMENT_COUNT.with(|count| count.set(0));
    EMITTED_COUNT.with(|count| count.set(0));
}

/// Appends one fenced block to the raw text: only a source-position wrapper
/// can feed this into the parser, and a fence mints no anchor or fact id.
pub(super) struct AppendBlockSource;

impl TransformBehavior for AppendBlockSource {
    fn name(&self) -> &str {
        "test-source-append"
    }
    fn epoch(&self) -> u32 {
        1
    }
    fn stage(&self) -> TransformStage {
        TransformStage::Source
    }
    fn run_source(
        &self,
        _config: Option<&TransformConfig>,
        input: SourceIr,
    ) -> Result<SourceIr, TransformBehaviorError> {
        let invocation = SOURCE_COUNT.with(|count| count.replace(count.get() + 1));
        let text = format!("{}\n```\nAppended-{invocation}\n```\n", input.text());
        Ok(SourceIr::new(
            input.address().clone(),
            input.format().clone(),
            input.subject().clone(),
            text,
        ))
    }
}

/// Re-parses the document text plus one fenced block into the TREE only: the
/// paired source stays byte-identical, which only a document-position wrapper
/// can achieve.
pub(super) struct BlockTreeDocument;

impl TransformBehavior for BlockTreeDocument {
    fn name(&self) -> &str {
        "test-tree-section"
    }
    fn epoch(&self) -> u32 {
        1
    }
    fn stage(&self) -> TransformStage {
        TransformStage::Document
    }
    fn run_document(
        &self,
        _config: Option<&TransformConfig>,
        input: DocumentIr,
    ) -> Result<DocumentIr, TransformBehaviorError> {
        let invocation = DOCUMENT_COUNT.with(|count| count.replace(count.get() + 1));
        let text = format!(
            "{}\n```\nAppended-{invocation}\n```\n",
            input.source().text()
        );
        Ok(DocumentIr::new(
            input.source().clone(),
            DocTree::parse(&text),
        ))
    }
}

/// Returns the artifact bytes plus one newline — the minimal CHANGING emitted
/// behavior, and the vehicle every T9 reconstruction proof drives.
///
/// One newline is enough on purpose: the reconstruction law is about the
/// artifact rebuilt around new bytes, not about how interesting the new bytes
/// are, and a one-byte delta keeps a chained plan's expected tape trivially
/// derivable from the baseline's.
pub(super) struct AppendEmitted;

impl TransformBehavior for AppendEmitted {
    fn name(&self) -> &str {
        "test-emit-append"
    }
    fn epoch(&self) -> u32 {
        1
    }
    fn stage(&self) -> TransformStage {
        TransformStage::Emitted
    }
    fn run_emitted(
        &self,
        _config: Option<&TransformConfig>,
        mut input: Vec<u8>,
    ) -> Result<Vec<u8>, TransformBehaviorError> {
        EMITTED_COUNT.with(|count| count.set(count.get() + 1));
        input.extend_from_slice(b"\n");
        Ok(input)
    }
}

/// Refuses every invocation with the behavior family's typed error.
pub(super) struct FailingSource;

impl TransformBehavior for FailingSource {
    fn name(&self) -> &str {
        "test-source-fails"
    }
    fn epoch(&self) -> u32 {
        1
    }
    fn stage(&self) -> TransformStage {
        TransformStage::Source
    }
    fn run_source(
        &self,
        _config: Option<&TransformConfig>,
        input: SourceIr,
    ) -> Result<SourceIr, TransformBehaviorError> {
        Err(self.wrong_stage(TransformStage::Source)).map(|_: SourceIr| input)
    }
}

/// Refuses every emitted invocation with the behavior family's typed error:
/// the emitted-path classification proof (the failing box crosses the
/// artifact segment, not discovery).
pub(super) struct FailingEmitted;

impl TransformBehavior for FailingEmitted {
    fn name(&self) -> &str {
        "test-emit-fails"
    }
    fn epoch(&self) -> u32 {
        1
    }
    fn stage(&self) -> TransformStage {
        TransformStage::Emitted
    }
    fn run_emitted(
        &self,
        _config: Option<&TransformConfig>,
        input: Vec<u8>,
    ) -> Result<Vec<u8>, TransformBehaviorError> {
        Err(self.wrong_stage(TransformStage::Emitted)).map(|_: Vec<u8>| input)
    }
}

/// Returns a source whose static origin is blank: the inter-pass verifier
/// rejects it, and the fault must attribute to the transform pass.
pub(super) struct BlankOriginSource;

impl TransformBehavior for BlankOriginSource {
    fn name(&self) -> &str {
        "test-source-blank-origin"
    }
    fn epoch(&self) -> u32 {
        1
    }
    fn stage(&self) -> TransformStage {
        TransformStage::Source
    }
    fn run_source(
        &self,
        _config: Option<&TransformConfig>,
        input: SourceIr,
    ) -> Result<SourceIr, TransformBehaviorError> {
        Ok(SourceIr::new(
            DocumentAddress::StaticEntry {
                origin: String::new(),
                path: "boot/x.md".to_string(),
            },
            input.format().clone(),
            // The subject rides through untouched: the fault under test is
            // the blank origin, not a moved subject.
            input.subject().clone(),
            input.text().to_string(),
        ))
    }
}

/// Returns a source whose subject claims a different declared path: the T7
/// carrier is immutable, so the inter-pass verifier must refuse it and name
/// `subject.declared_path`.
pub(super) struct RetargetSubjectSource;

impl TransformBehavior for RetargetSubjectSource {
    fn name(&self) -> &str {
        "test-source-retarget-subject"
    }
    fn epoch(&self) -> u32 {
        1
    }
    fn stage(&self) -> TransformStage {
        TransformStage::Source
    }
    fn run_source(
        &self,
        _config: Option<&TransformConfig>,
        input: SourceIr,
    ) -> Result<SourceIr, TransformBehaviorError> {
        Ok(SourceIr::new(
            input.address().clone(),
            input.format().clone(),
            DocumentSubject::declared(DocumentProvider::Undetermined, FORGED_PATH),
            input.text().to_string(),
        ))
    }
}

/// The same forgery one position later: a document-position behavior may
/// rewrite the tree, never the subject its source carries.
pub(super) struct RetargetSubjectDocument;

impl TransformBehavior for RetargetSubjectDocument {
    fn name(&self) -> &str {
        "test-document-retarget-subject"
    }
    fn epoch(&self) -> u32 {
        1
    }
    fn stage(&self) -> TransformStage {
        TransformStage::Document
    }
    fn run_document(
        &self,
        _config: Option<&TransformConfig>,
        input: DocumentIr,
    ) -> Result<DocumentIr, TransformBehaviorError> {
        let (source, tree) = input.into_parts();
        let (address, format, _subject, text) = source.into_parts();
        Ok(DocumentIr::new(
            SourceIr::new(
                address,
                format,
                DocumentSubject::declared(DocumentProvider::Undetermined, FORGED_PATH),
                text,
            ),
            tree,
        ))
    }
}

/// The one forged path both subject-rewriting vehicles claim.
pub(super) const FORGED_PATH: &str = "boot/forged.md";

thread_local! {
    pub(super) static DELIVERED_CONFIGS: std::cell::RefCell<Vec<&'static str>> = const {
        std::cell::RefCell::new(Vec::new())
    };
    /// One `(address label, declared path, provider spelling)` row per
    /// document the document position saw, in invocation order.
    ///
    /// The provider is recorded as its own rendering rather than as "is there
    /// one": the two absences are DIFFERENT answers, and a boolean would fuse
    /// exactly the distinction the carrier exists to keep.
    pub(super) static OBSERVED_SUBJECTS: std::cell::RefCell<Vec<(String, String, String)>> = const {
        std::cell::RefCell::new(Vec::new())
    };
}

/// Records the subject every document arrives with, without touching it.
pub(super) struct RecordingSubjects;

impl TransformBehavior for RecordingSubjects {
    fn name(&self) -> &str {
        "test-record-subject"
    }
    fn epoch(&self) -> u32 {
        1
    }
    fn stage(&self) -> TransformStage {
        TransformStage::Document
    }
    fn run_document(
        &self,
        _config: Option<&TransformConfig>,
        input: DocumentIr,
    ) -> Result<DocumentIr, TransformBehaviorError> {
        let source = input.source();
        let subject = source.subject();
        let row = (
            document_key(source.address()).label(),
            subject.declared_path().to_string(),
            subject.provider().to_string(),
        );
        OBSERVED_SUBJECTS.with(|rows| rows.borrow_mut().push(row));
        Ok(input)
    }
}

/// Records the delivered config envelope word for each invocation.
pub(super) struct RecordingDocument;

impl TransformBehavior for RecordingDocument {
    fn name(&self) -> &str {
        "test-record-config"
    }
    fn epoch(&self) -> u32 {
        1
    }
    fn stage(&self) -> TransformStage {
        TransformStage::Document
    }
    fn run_document(
        &self,
        config: Option<&TransformConfig>,
        input: DocumentIr,
    ) -> Result<DocumentIr, TransformBehaviorError> {
        DOCUMENT_COUNT.with(|count| count.set(count.get() + 1));
        let word = match config {
            None => "none",
            Some(config) if config.as_table().is_empty() => "empty",
            Some(_) => "values",
        };
        DELIVERED_CONFIGS.with(|records| records.borrow_mut().push(word));
        Ok(input)
    }
}

/// One registry holding the shared identity catalog plus selected locals.
pub(super) fn registry_with(vehicles: &[Arc<dyn TransformBehavior>]) -> TransformRegistry {
    let mut registry = identity_registry();
    for vehicle in vehicles {
        registry
            .register(vehicle.clone())
            .expect("local vehicle registers");
    }
    registry
}
